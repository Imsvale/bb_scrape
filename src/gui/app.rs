// src/gui/app.rs
use std::{
    error::Error,
    path::PathBuf,
    sync::{ Arc, Mutex },
    collections::HashMap,
};

use eframe::egui;
use egui::{ Align, Layout, RichText, TextWrapMode };
use egui_extras::{ Column, TableBuilder };

use crate::{
    config::{
        state::{ AppState, GuiState },
        options::{ 
            ExportFormat, 
            ExportType::{ SingleFile, PerTeam }, 
            PageKind,
            TeamSelector,
        },
    },
    file,
    scrape,   // still used by Players page path
    store,
    teams,
};

use super::{
    pages::{ self, AppCtx, Page },
    router,
    progress::GuiProgress,
};

pub fn run(options: eframe::NativeOptions) -> Result<(), Box<dyn Error>> {
    eframe::run_native(
        "Brutalball Scraper",
        options,
        Box::new(|_cc| Ok(Box::new(App::new(AppState::default())))),
    )?;
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum UiFormat { Csv, Tsv }

pub struct App {
    // single source of truth
    state: Arc<Mutex<AppState>>,

    // teams & selection UI (selection lives inside state.gui)
    teams: Vec<(u32, String)>,
    last_clicked: Option<usize>,

    // output text field UX (we map this <-> ExportOptions)
    out_path_text: String,
    out_path_dirty: bool,

    // in-memory data preview
    headers: Option<Vec<String>>,
    rows: Vec<Vec<String>>,

    // status/progress
    status: Arc<Mutex<String>>,
    running: bool,

    // routing
    current_page: PageKind,
    page_data: HashMap<PageKind, store::DataSet>,

    // bootstrap: rebuild view from cache once on first frame
    needs_initial_view: bool,
}

impl App {
    pub fn new(mut state: AppState) -> Self {
        // Teams list (fallback to 0..31 if file missing)
        let teams = match teams::load() {
            Ok(v) if !v.is_empty() => v,
            _ => (0u32..32).map(|id| (id, format!("Team {}", id))).collect(),
        };

        // Default selection: all
        state.gui = GuiState {
            selected_team_ids: teams.iter().map(|(id, _)| *id).collect(),
            ..GuiState::default()
        };

        // Minimal visible table at start (will be rebuilt on first frame)
        let headers: Option<Vec<String>> = None;
        let rows: Vec<Vec<String>> = Vec::new();
        let status = "Idle".to_string();

        // ExportOptions::out_path() -> PathBuf
        let out_path_text = state
            .options
            .export
            .out_path()
            .to_string_lossy()
            .into();

        // Build per-page in-memory cache from disk if present
        let mut page_data = HashMap::new();
        for page in crate::gui::router::all_pages() {
            let kind = page.kind();
            if let Ok(ds) = store::load_dataset(&kind) {
                page_data.insert(kind, ds);
            }
        }

        Self {
            state: Arc::new(Mutex::new(state)),
            teams,
            last_clicked: None,
            out_path_text,
            out_path_dirty: false,
            headers,
            rows,
            status: Arc::new(Mutex::new(status)),
            running: false,
            current_page: PageKind::Players,
            page_data,
            needs_initial_view: true,
        }
    }

    fn set_selection_message(&self) {
        let n = self.state.lock().unwrap().gui.selected_team_ids.len();
        *self.status.lock().unwrap() = format!("Selection: {} team(s) — not scraped yet", n);
    }

    fn make_ctx<'a>(&'a mut self, egui_ctx: &'a egui::Context) -> AppCtx<'a> {
        let status = self.status.clone(); // capture Arc for the callback

        AppCtx {
            egui_ctx,
            app_state: self.state.lock().unwrap(), // holds the lock while AppCtx lives
            headers: &mut self.headers,
            rows: &mut self.rows,
            teams: &self.teams,
            set_status: Box::new(move |s| *status.lock().unwrap() = s),
        }
    }

    /// Mirror the GUI's selected_team_ids into options.scrape.teams.
    fn sync_gui_selection_into_scrape(&mut self) {
        let teams_total = self.teams.len();

        let mut st = self.state.lock().unwrap();
        let sel = &st.gui.selected_team_ids;

        st.options.scrape.teams = if sel.is_empty() {
            // Nothing selected -> empty Ids list (scrape nothing)
            TeamSelector::Ids(Vec::new())
        } else if sel.len() == teams_total {
            TeamSelector::All
        } else if sel.len() == 1 {
            TeamSelector::One(sel[0])
        } else {
            let mut v = sel.clone();
            v.sort_unstable();
            v.dedup();
            TeamSelector::Ids(v)
        };
    }

    /// Rebuild the display headers/rows from the canonical per-page cache
    /// applying selection filters and page-specific view toggles
    fn rebuild_view_from_cache(&mut self, egui_ctx: &egui::Context) {
        let pages = crate::gui::router::all_pages();
        let st = self.state.lock().unwrap();
        let cur = st.gui.current_page_index;
        let sel_ids = st.gui.selected_team_ids.clone();
        drop(st);

        let page = pages[cur];
        let kind = page.kind();

        // 1) pull canonical dataset for this page, if any
        let ds_opt = self.page_data.get(&kind);

        // 2) choose headers baseline (dataset headers or page defaults)
        let headers0: Option<Vec<String>> = match ds_opt.and_then(|ds| ds.headers.clone()) {
            Some(h) => Some(h),
            None => page
                .default_headers()
                .map(|hs| hs.iter().map(|s| s.to_string()).collect()),
        };

        // 3 choose rows baseline (dataset rows or empty)

        let rows0: Vec<Vec<String>> = ds_opt
            .map(|ds| &ds.rows)
            .map(|r| r.clone()) // view starts from canonical data (clone once)
            .unwrap_or_default();

        // 4) filter by team selection if the page wants it
        let filtered = page.filter_rows_for_selection(&sel_ids, &self.teams, &rows0);

        // 5) let the page adapt the view for display (e.g. hide columns)
        let (hdrs, rows) = {
            let mut ctx = self.make_ctx(egui_ctx);
            page.view_for_display(&ctx, &headers0, &filtered)
        };

        self.headers = hdrs;
        self.rows = rows;
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        if std::mem::take(&mut self.needs_initial_view) {
            self.rebuild_view_from_cache(ctx);
        }

        /* -------------- Left panel: team select -------------- */
        egui::SidePanel::left("teams").resizable(false).show(ctx, |ui| {
            ui.heading("Teams");
            ui.horizontal(|ui| {
                if ui.button("All").clicked() {
                    let mut st = self.state.lock().unwrap();
                    st.gui.selected_team_ids = self.teams.iter().map(|(id, _)| *id).collect();
                    drop(st);
                    self.set_selection_message();
                }
                if ui.button("None").clicked() {
                    self.state.lock().unwrap().gui.selected_team_ids.clear();
                    self.set_selection_message();
                }
            });
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                let mut changed = false;

                for (idx, (id, name)) in self.teams.iter().enumerate() {
                    let is_selected = {
                        let st = self.state.lock().unwrap();
                        st.gui.selected_team_ids.contains(id)
                    };

                    let resp = ui.selectable_label(is_selected, name);

                    if resp.clicked() && !self.running {
                        let input = ui.input(|i| i.clone());
                        let mut st = self.state.lock().unwrap();
                        let sel = &mut st.gui.selected_team_ids;

                        if input.modifiers.ctrl {
                            if is_selected { sel.retain(|x| x != id); } else { sel.push(*id); }
                            self.last_clicked = Some(idx);
                        } else if input.modifiers.shift {
                            if let Some(last) = self.last_clicked {
                                let (lo, hi) = if last <= idx { (last, idx) } else { (idx, last) };
                                sel.clear();
                                for j in lo..=hi { sel.push(self.teams[j].0); }
                            }
                        } else {
                            sel.clear();
                            sel.push(*id);
                            self.last_clicked = Some(idx);
                        }
                        changed = true;
                    }
                }

                if changed {
                    self.set_selection_message();
                }
            });
        });

        /* -------------- Center: options + preview + actions -------------- */
        egui::CentralPanel::default().show(ctx, |ui| {
            // --- Tabs row (chunky buttons) ---
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;

                let pages = router::all_pages();
                let st = self.state.lock().unwrap();
                let cur = st.gui.current_page_index;
                drop(st);

                for (idx, page) in pages.iter().enumerate() {
                    let selected = idx == cur;
                    if ui.selectable_label(selected, page.label()).clicked() && !selected {
                        let mut st = self.state.lock().unwrap();
                        st.gui.current_page_index = idx;
                        drop(st);
                        // rebuild the visible view from canonical cache
                        self.rebuild_view_from_cache(ctx);
                    }
                }
            });
            ui.separator();

            // Format selector (binds to ExportOptions)
            let prev_fmt = {
                let st = self.state.lock().unwrap();
                match st.options.export.format {
                    ExportFormat::Csv => UiFormat::Csv,
                    ExportFormat::Tsv => UiFormat::Tsv,
                }
            };

            let mut fmt = prev_fmt;
            ui.horizontal(|ui| {
                ui.label("Format:");
                ui.selectable_value(&mut fmt, UiFormat::Csv, "CSV");
                ui.selectable_value(&mut fmt, UiFormat::Tsv, "TSV");
            });

            if fmt != prev_fmt {
                let mut st = self.state.lock().unwrap();
                let export = &mut st.options.export;

                export.format = match fmt {
                    UiFormat::Csv => ExportFormat::Csv,
                    UiFormat::Tsv => ExportFormat::Tsv,
                };

                if !self.out_path_dirty {
                    self.out_path_text = export.out_path().to_string_lossy().into_owned();
                }
            }

            // Export-affecting toggles (header is universal)
            let current_index = {
                let mut st = self.state.lock().unwrap();
                ui.checkbox(&mut st.options.export.include_headers, "Include headers");
                st.gui.current_page_index
            };

            // Page-specific options (e.g., "Keep #")
            {
                let mut appctx = self.make_ctx(ctx);
                // Page-specific controls (header area)
                let pages = crate::gui::router::all_pages();
                
                let page = pages[current_index];
                page.draw_controls(ui, &mut appctx);

            }

            // Export options (Single vs Per-team + Output field)
            ui.horizontal(|ui| {
                let mut app_state = self.state.lock().unwrap();

                let per_team_allowed = {
                    let pages = router::all_pages();
                    let cur = app_state.gui.current_page_index;
                    matches!(pages[cur].kind(), PageKind::Players)
                };

                // Checkbox only shown when allowed; otherwise force SingleFile
                let export = &mut app_state.options.export;
                let before = export.export_type;
                let mut single = true;
                if per_team_allowed {
                    single = matches!(before, SingleFile);
                    let single_resp = ui.checkbox(&mut single, "All teams in one file");
                    if single_resp.changed() {
                        export.export_type = if single { SingleFile } else { PerTeam };
                        if !self.out_path_dirty {
                            self.out_path_text = export.out_path().to_string_lossy().into_owned();
                        }
                    }
                } else {
                    export.export_type = SingleFile;
                    ui.label("All teams in one file");
                }

                ui.label("Output:");
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.out_path_text)
                        .font(egui::TextStyle::Monospace)
                );
                if resp.changed() { self.out_path_dirty = true; }
            });

            // COPY & EXPORT BUTTONS
            ui.horizontal(|ui| {
                // Copy
                if ui.button("Copy").clicked() {
                    if self.rows.is_empty() {
                        *self.status.lock().unwrap() = "Nothing to copy".to_string();
                    } else {
                        let st = self.state.lock().unwrap().clone();
                        let options = &st.options;

                        let txt = file::to_export_string(options, &self.headers, &self.rows);
                        ctx.copy_text(txt);
                        *self.status.lock().unwrap() = "Copied to clipboard".to_string();
                    }
                }

                // Export
                if ui.button("Export").clicked() {
                    if self.rows.is_empty() {
                        *self.status.lock().unwrap() = "Nothing to export".to_string();
                    } else {
                        // Push Output text → ExportOptions if dirty
                        {
                            let mut st = self.state.lock().unwrap();
                            let export = &mut st.options.export;
                            if self.out_path_dirty {
                                export.set_path(&self.out_path_text);
                                self.out_path_dirty = false;
                            }
                        }

                        // Snapshot for IO
                        let st = self.state.lock().unwrap().clone();
                        let options = &st.options;
                        let export = &options.export;

                        let res: Result<Vec<PathBuf>, Box<dyn Error>> = match export.export_type {
                            SingleFile => file::write_export_single(options, &self.headers, &self.rows)
                                .map(|p| vec![p]),
                            PerTeam => file::write_export_per_team(options, &self.headers, &self.rows, 3),
                        };

                        match res {
                            Ok(paths) => {
                                if let Some(last) = paths.last() {
                                    *self.status.lock().unwrap() =
                                        format!("Exported {} file(s). Last: {}", paths.len(), last.display());
                                } else {
                                    *self.status.lock().unwrap() = "Export done".to_string();
                                }
                            }
                            Err(e) => {
                                *self.status.lock().unwrap() = format!("Export error: {}", e);
                            }
                        }
                    }
                }

                // SCRAPE
                let red = egui::Color32::from_rgb(220, 30, 30);
                let black = egui::Color32::BLACK;
                if ui
                    .add(
                        egui::Button::new(egui::RichText::new("SCRAPE").color(black).strong())
                            .fill(red),
                    )
                    .clicked()
                {
                    // snapshot for selection
                    let snapshot = self.state.lock().unwrap().clone();
                    let pages = crate::gui::router::all_pages();
                    let page = pages[snapshot.gui.current_page_index];
                    let kind = page.kind();

                    // push GUI team selection into scrape options
                    self.sync_gui_selection_into_scrape();

                    let mut prog = GuiProgress::new(self.status.clone());

                    // short-lived ctx for scrape
                    let ds_res = {
                        let appctx = self.make_ctx(ui.ctx());
                        let r = page.scrape(&appctx, Some(&mut prog));
                        drop(appctx);
                        r
                    };

                    match ds_res {
                        Ok(ds) => {
                            // merge into canonical cache
                            let entry = self.page_data.entry(kind).or_insert(store::DataSet {
                                headers: None,
                                rows: Vec::new(),
                            });
                            page.merge(entry, ds);

                            // persist canonical cache for this page
                            let _ = store::save_dataset(&kind, entry);

                            // rebuild view from canonical cache (respects page toggles)
                            self.rebuild_view_from_cache(ui.ctx());
                            *self.status.lock().unwrap() = "Ready".to_string();
                        }
                        Err(e) => {
                            *self.status.lock().unwrap() = format!("Error: {}", e);
                        }
                    }
                }

                // Status
                let status = self.status.lock().unwrap().clone();
                ui.label(format!("Status: {}", status));
            });

            ui.separator();

            // Live table preview (same builder you had)
            let pages = crate::gui::router::all_pages();
            let cur = self.state.lock().unwrap().gui.current_page_index;
            let page = pages[cur];

            // Prefer live headers; fall back to the page's known headers.
            let headers = self.headers.clone().or_else(|| page.default_headers().map(|s| s.iter().map(|x| x.to_string()).collect()));
            self.headers = headers; // keep it sticky

            let cols = self.headers.as_ref().map(|h| h.len())
                .or_else(|| self.rows.get(0).map(|r| r.len()))
                .unwrap_or_else(|| page.default_headers().map(|h| h.len()).unwrap_or(0));


            let widths = page.preferred_column_widths();

            // If the page provides widths, use them; otherwise default to Players-style
            let mut table = TableBuilder::new(ui)
                .striped(true)
                .min_scrolled_height(0.0);

            if let Some(ws) = widths {
                for (i, w) in ws.iter().copied().enumerate() {
                    let mut col = Column::initial(w as f32).resizable(true);
                    // make some narrow cols unshrinkable
                    if i <= 1 { col = col.at_least(w as f32); }
                    table = table.column(col);
                }
            } else {
                // players default
                table = table
                    .column(Column::initial(60.0).at_least(180.0).resizable(true)) // Name
                    .column(Column::initial(30.0).at_least(30.0).resizable(true))  // Number
                    .column(Column::initial(140.0).at_least(120.0).resizable(true))// Race
                    .column(Column::initial(160.0).at_least(140.0).resizable(true));// Team
                for _ in 4..cols {
                    table = table.column(Column::initial(30.0).at_least(30.0).resizable(true));
                }
            }


            table
                .header(24.0, |mut header| {
                    if let Some(hs) = &self.headers {
                        for h in hs {
                            header.col(|ui| {
                                ui.scope(|ui| {
                                    ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);
                                    ui.with_layout(
                                        Layout::left_to_right(Align::Center),
                                        |ui| { ui.label(RichText::new(h).strong()); }
                                    );
                                });
                            });
                        }
                    } else {
                        for i in 0..cols {
                            header.col(|ui| {
                                ui.scope(|ui| {
                                    ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);
                                    ui.with_layout(
                                        Layout::left_to_right(Align::Center),
                                        |ui| { ui.label(RichText::new(format!("Col {}", i + 1)).strong()); }
                                    );
                                });
                            });
                        }
                    }
                })
                .body(|body| {
                    body.rows(20.0, self.rows.len(), |mut row| {
                        let row_idx = row.index();
                        if let Some(data) = self.rows.get(row_idx) {
                            for (ci, cell) in data.iter().enumerate() {
                                row.col(|ui| {
                                    ui.scope(|ui| {
                                        ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);
                                        let rt = RichText::new(cell);
                                        if ci == 0 {
                                            ui.label(rt);
                                        } else {
                                            ui.with_layout(
                                                Layout::left_to_right(Align::Center),
                                                |ui| { ui.label(rt) },
                                            );
                                        }
                                    });
                                });
                            }
                        }
                    });
                });
        });
    }
}
