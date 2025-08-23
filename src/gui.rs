// src/gui.rs
use std::{
    error::Error,
    path::{ PathBuf },
    sync::{ Arc, Mutex },
    thread,
};

use eframe::egui;
use egui::{ Align, Layout, RichText, TextWrapMode };
use egui_extras::{ Column, TableBuilder };

use crate::{
    config::{
        options::{ ExportFormat, ExportType, PageKind },
        state::{ AppState, GuiState },
    },
    csv::{ to_export_string },
    file,
    scrape,
    store,
    teams,
};

pub fn run(app_state: AppState) -> Result<(), Box<dyn Error>> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Brutalball Scraper",
        options,
        Box::new(|_cc| Ok(Box::new(App::new(app_state)))),
    )?;
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum UiFormat {
    Csv,
    Tsv,
}

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

        // Local data cache
        let mut headers = None;
        let mut rows = Vec::new();
        let mut status = "Idle".to_string();

        if let Ok(ds) = store::load_dataset(&PageKind::Players) {
            headers = ds.headers;
            rows = ds.rows;
            status = "Loaded local data".to_string();
        }
        rows.retain(|r| !r.is_empty() && !(r.len() == 1 && r[0].trim().is_empty()));

        // ExportOptions::out_path() -> PathBuf
        let out_path_text = state
            .options
            .export
            .out_path()
            .to_string_lossy()
            .into();

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
        }
    }

    /* ---------- Output field <-> ExportOptions mapping ---------- */

    fn set_selection_message(&self) {
        *self.status.lock().unwrap() = "Selection changed — not scraped yet.".to_string();
    }

    /* ---------- Scrape/Export hooks ---------- */

    fn recollect(&mut self, ctx: &egui::Context) {
        if self.running {
            return;
        }
        self.running = true;
        *self.status.lock().unwrap() = "Refreshing…".to_string();

        let snapshot = self.state.lock().unwrap().clone();
        let scrape = snapshot.options.scrape;
        let status_arc = self.status.clone();
        let ctx2 = ctx.clone();

        thread::spawn(move || {
            let mut prog = GuiProgress::new(status_arc.clone());
            let ds = scrape::collect_players(&scrape, Some(&mut prog));
            let msg = match ds {
                Ok(ds) => format!(
                    "Ready: {} rows{}",
                    ds.rows.len(),
                    if ds.headers.is_some() { " + headers" } else { "" }
                ),
                Err(e) => format!("Error: {}", e),
            };
            *status_arc.lock().unwrap() = msg;
            ctx2.request_repaint();
        });
    }

    fn sync_collect(&mut self) {
        let mut prog = GuiProgress::new(self.status.clone());
        let snapshot = self.state.lock().unwrap().clone();
        let scrape = snapshot.options.scrape;

        match scrape::collect_players(&scrape, Some(&mut prog)) {
            Ok(ds) => {
                self.headers = ds.headers;
                self.rows = ds.rows;

                let _ = store::save_dataset(
                    &PageKind::Players,
                    &store::Dataset { headers: self.headers.clone(), rows: self.rows.clone() }
                );

                *self.status.lock().unwrap() = "Ready".to_string();
            }
            Err(e) => {
                *self.status.lock().unwrap() = format!("Error: {}", e);
            }
        }
        self.running = false;
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        /* ---------------- Tabs (stub) ---------------- */
        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_label(true, "Players"); // single page for now
            });
        });

        /* -------------- Left: team multi-select -------------- */
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
                            if is_selected {
                                sel.retain(|x| x != id);
                            } else {
                                sel.push(*id);
                            }
                            self.last_clicked = Some(idx);
                        } else if input.modifiers.shift {
                            if let Some(last) = self.last_clicked {
                                let (lo, hi) = if last <= idx { (last, idx) } else { (idx, last) };
                                sel.clear();
                                for j in lo..=hi {
                                    sel.push(self.teams[j].0);
                                }
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
            ui.heading("Players");
            ui.separator();

            // Format selector (binds to ExportOptions)
            let prev_fmt = {

                let st = self.state.lock().unwrap();
                let export = &st.options.export;

                match export.format {
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
                    self.out_path_text = st
                        .options
                        .export
                        .out_path()
                        .to_string_lossy()
                        .into_owned();
                }
            }

            // Export-affecting toggles
            {
                let mut st = self.state.lock().unwrap();
                let export = &mut st.options.export;

                ui.checkbox(&mut export.include_headers, "Include headers");
                ui.checkbox(&mut export.keep_hash, "Keep hash in player numbers");
            }

            // Export options (Single vs Per-team + Output field)
            ui.horizontal(|ui| {
                let mut app_state = self.state.lock().unwrap();
                let export = &mut app_state.options.export;

                let before = &export.export_type;
                let mut single = matches!(before, ExportType::SingleFile);
                let single_resp = ui.checkbox(&mut single, "Single file");

                ui.label("Output:");
                let resp = ui.text_edit_singleline(&mut self.out_path_text);
                if resp.changed() {
                    self.out_path_dirty = true;
                }

                if single_resp.changed() {
                    export.export_type = if single { 
                        ExportType::SingleFile 
                    } else { 
                        ExportType::PerTeam 
                    };
                    
                    if !self.out_path_dirty {
                        // Repaint the field from the model's resolved path/dir
                        self.out_path_text = export
                            .out_path()
                            .to_string_lossy()
                            .into_owned();
                    }
                }
            });

            // #########################
            // # COPY & EXPORT BUTTONS #
            // #########################
            ui.horizontal(|ui| {

                // ################
                // # Button: Copy #
                // ################
                if ui.button("Copy").clicked() {

                    let st = self.state.lock().unwrap().clone();
                    let export = &st.options.export;

                    let txt = to_export_string(
                        &self.headers,
                        &self.rows,
                        export.include_headers,
                        export.keep_hash,
                        export.delimiter().unwrap(), // Option(char) from ExportOptions
                    );
                    ctx.copy_text(txt);
                    *self.status.lock().unwrap() = "Copied to clipboard".to_string();
                }
                // ##################
                // # Button: Export #
                // ##################
                if ui.button("Export").clicked() {
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
                    let export = st.options.export;

                    let res: Result<Vec<PathBuf>, Box<dyn Error>> = match export.export_type {

                        ExportType::SingleFile => file::write_export_single(
                            &export, 
                            &self.headers, 
                            &self.rows
                        )
                        .map(|p| vec![p] ),

                        ExportType::PerTeam => file::write_export_per_team(
                            &export, 
                            &self.headers, 
                            &self.rows, 
                            3, // "Team" column
                        )
                    };

                    match res {
                        Ok(paths) => {
                            if let Some(last) = paths.last() {
                                *self.status.lock().unwrap() =
                                    format!("Exported {} file(s), e.g. {}", paths.len(), last.display());
                            } else {
                                *self.status.lock().unwrap() = "Export done".to_string();
                            }
                        }
                        Err(e) => {
                            *self.status.lock().unwrap() = format!("Export error: {}", e);
                        }
                    }
                }

                // ####################
                // ## BUTTON: SCRAPE ##
                // ####################
                let red = egui::Color32::from_rgb(220, 30, 30);
                let black = egui::Color32::BLACK;
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("SCRAPE").color(black).strong(),
                        )
                        .fill(red),
                    )
                    .clicked()
                {
                    self.sync_collect();
                }

                // Status
                let status = self.status.lock().unwrap().clone();
                ui.label(format!("Status: {}", status));
            });


            ui.separator();

            // Live table preview
            let cols = self
                .headers
                .as_ref()
                .map(|h| h.len())
                .or_else(|| self.rows.get(0).map(|r| r.len()))
                .unwrap_or(0);

            let mut table = TableBuilder::new(ui)
                .striped(true)
                .min_scrolled_height(0.0)
                .column(Column::auto().resizable(true).at_least(180.0)) // Name
                .column(Column::auto().resizable(true).at_least(30.0)) // Number
                .column(Column::auto().resizable(true).at_least(120.0)) // Race
                .column(Column::auto().resizable(true).at_least(140.0)); // Team

            for _ in 4..cols {
                table = table.column(Column::auto().resizable(true).at_least(30.0));
            }

            table
                .header(24.0, |mut header| {
                    if let Some(hs) = &self.headers {
                        for h in hs {
                            header.col(|ui| { ui.scope(|ui| { 
                                ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);
                                ui.with_layout(
                                    Layout::left_to_right(Align::Center),
                                    |ui| { ui.label(RichText::new(h).strong()); }
                                );
                            }); });
                        }
                    } else {
                        for i in 0..cols { 
                            header.col(|ui| { ui.scope(|ui| { 
                                ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);
                                ui.with_layout(
                                    Layout::left_to_right(Align::Center),
                                    |ui| { ui.label(RichText::new(format!("Col {}", i + 1)).strong()); }
                                );
                            }); });
                        }
                    }
                })
                .body(|mut body| {
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
                                                |ui| {
                                                    ui.label(rt);
                                                },
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

/* ---------------- helpers ---------------- */

fn trim_trailing_sep(s: &str) -> &str {
    s.trim_end_matches(['/', '\\'])
}

/* ---------- Progress adapter ---------- */
// gui.rs (near the bottom)

struct GuiProgress {
    status: Arc<Mutex<String>>,
    done: usize,
    total: usize,
}

impl GuiProgress {
    fn new(status: Arc<Mutex<String>>) -> Self {
        Self { status, done: 0, total: 0 }
    }
    fn set_status(&self, msg: impl Into<String>) {
        *self.status.lock().unwrap() = msg.into();
    }
}

impl crate::progress::Progress for GuiProgress {
    fn begin(&mut self, total: usize) {
        self.total = total;
        self.set_status(format!("Starting… {} team(s)", total));
    }
    fn log(&mut self, msg: &str) {
        self.set_status(msg.to_string());
    }
    fn item_done(&mut self, team_id: u32) {
        self.done += 1;
        self.set_status(format!("Fetched team {} ({}/{})", team_id, self.done, self.total));
    }
    fn finish(&mut self) {
        self.set_status("Fetch complete".to_string());
    }
}

