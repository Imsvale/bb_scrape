// src/gui.rs
use std::error::Error;
use crate::{
    csv::{Delim, rows_to_string},
    params::{
        Params, 
        PageKind, 
        DEFAULT_OUT_DIR, 
        DEFAULT_MERGED_FILENAME },
    runner::{self, Progress},
    teams,
};
use eframe::egui;
use egui_extras::{TableBuilder, Column};
use std::{path::PathBuf, sync::{Arc, Mutex}, thread};

pub fn run(params: Params) -> Result<(), Box<dyn Error>> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Brutalball Scraper",
        options,
        Box::new(|_cc| Ok(Box::new(App::new(params)))),
    )?;
    Ok(())
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum UiFormat { Csv, Tsv }

pub struct App {
    params: Params,
    // teams & selection UI
    teams: Vec<(u32, String)>,
    selected: Vec<u32>,
    last_clicked: Option<usize>,

    // format & output path (GUI only)
    fmt: UiFormat,
    out_path: String,

    // in-memory data
    headers: Option<Vec<String>>,
    rows: Vec<Vec<String>>,

    // status/progress
    status: Arc<Mutex<String>>,
    running: bool,
}

impl App {
    pub fn new(params: Params) -> Self {
        let teams = match teams::load() {
            Ok(v) if !v.is_empty() => v,
            _ => (0u32..32).map(|id| (id, format!("Team {}", id))).collect(),
        };
        let selected = teams.iter().map(|(id, _)| *id).collect();

        Self {
            params,
            teams,
            selected,
            last_clicked: None,
            fmt: UiFormat::Csv,
            out_path: format!("{}/{}", DEFAULT_OUT_DIR, DEFAULT_MERGED_FILENAME),
            headers: None,
            rows: Vec::new(),
            status: Arc::new(Mutex::new("Idle".into())),
            running: false,
        }
    }

    fn apply_selection(&mut self) {
        if self.selected.is_empty() {
            self.params.all = true;
            self.params.one_team = None;
            self.params.ids_filter = None;
        } else if self.selected.len() == 1 {
            self.params.all = false;
            self.params.one_team = Some(self.selected[0]);
            self.params.ids_filter = None;
        } else {
            self.params.all = true; // still scrape all, but filter active
            self.params.one_team = None;
            let mut filt = self.selected.clone();
            filt.sort_unstable();
            filt.dedup();
            self.params.ids_filter = Some(filt);
        }
    }

    fn recollect(&mut self, ctx: &egui::Context) {
        if self.running { return; }
        self.running = true;
        *self.status.lock().unwrap() = "Refreshing…".to_string();

        // Update params in place
        if self.selected.is_empty() {
            self.params.all = true;
            self.params.one_team = None;
            self.params.ids_filter = None;
        } else if self.selected.len() == 1 {
            self.params.all = false;
            self.params.one_team = Some(self.selected[0]);
            self.params.ids_filter = None;
        } else {
            self.params.all = true;
            self.params.one_team = None;
            let mut filt = self.selected.clone();
            filt.sort_unstable();
            filt.dedup();
            self.params.ids_filter = Some(filt);
        }

        let params_clone = self.params.clone();
        let status_arc = self.status.clone();
        let ctx2 = ctx.clone();

        thread::spawn(move || {
            let ds = runner::collect_players(&params_clone);
            let msg = match ds {
                Ok(ds) => format!("Ready: {} rows{}", ds.rows.len(), if ds.headers.is_some() { " + headers" } else { "" }),
                Err(e) => format!("Error: {}", e),
            };
            *status_arc.lock().unwrap() = msg;
            ctx2.request_repaint();
        });
    }


    fn sync_collect(&mut self) {
        // Adjust params based on current UI selections
        if self.selected.is_empty() {
            self.params.all = true;
            self.params.one_team = None;
            self.params.ids_filter = None;
        } else if self.selected.len() == 1 {
            self.params.all = false;
            self.params.one_team = Some(self.selected[0]);
            self.params.ids_filter = None;
        } else {
            self.params.all = true;
            self.params.one_team = None;
            let mut filt = self.selected.clone();
            filt.sort_unstable();
            filt.dedup();
            self.params.ids_filter = Some(filt);
        }

        match runner::collect_players(&self.params) {
            Ok(ds) => {
                self.headers = ds.headers;
                self.rows = ds.rows;
                *self.status.lock().unwrap() = "Ready".into();
            }
            Err(e) => {
                *self.status.lock().unwrap() = format!("Error: {}", e);
            }
        }
        self.running = false;
    }

    
    fn current_delim(&self) -> Delim {
        match self.fmt { UiFormat::Csv => Delim::Csv, UiFormat::Tsv => Delim::Tsv }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Tabs (future expansion)
        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.params.page, PageKind::Players, "Players");
            });
        });

        // Left: team multiselect with All/None
        egui::SidePanel::left("teams").resizable(false).show(ctx, |ui| {
            ui.heading("Teams");
            ui.horizontal(|ui| {
                if ui.button("All").clicked() {
                    self.selected = self.teams.iter().map(|(id, _)| *id).collect();
                    self.sync_collect();
                }
                if ui.button("None").clicked() {
                    self.selected.clear();
                    self.sync_collect();
                }
            });
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {

                let mut needs_sync = false;

                for (idx, (id, name)) in self.teams.iter().enumerate() {
                    let is_selected = self.selected.contains(id);
                    let resp = ui.selectable_label(is_selected, name);
                    if resp.clicked() && !self.running {
                        let input = ui.input(|i| i.clone());
                        if input.modifiers.ctrl {
                            if is_selected { self.selected.retain(|x| x != id); }
                            else { self.selected.push(*id); }
                            self.last_clicked = Some(idx);
                        } else if input.modifiers.shift {
                            if let Some(last) = self.last_clicked {
                                let (lo, hi) = if last <= idx {(last, idx)} else {(idx, last)};
                                self.selected.clear();
                                for j in lo..=hi { self.selected.push(self.teams[j].0); }
                            }
                        } else {
                            self.selected.clear();
                            self.selected.push(*id);
                            self.last_clicked = Some(idx);
                        }
                        
                        needs_sync = true;
                    }
                }
                if needs_sync {
                    self.sync_collect();
                }
            });
        });

        // Center: options + display + copy/export
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Players");
            ui.separator();

            // Options (affect data immediately)
            let mut changed = false;
            changed |= ui.checkbox(&mut self.params.include_headers, "Include headers").changed();
            changed |= ui.checkbox(&mut self.params.keep_hash, "Keep hash in player numbers").changed();

            // Format selector (CSV/TSV) — affects Copy/Export only
            ui.horizontal(|ui| {
                ui.label("Format:");
                ui.selectable_value(&mut self.fmt, UiFormat::Csv, "CSV");
                ui.selectable_value(&mut self.fmt, UiFormat::Tsv, "TSV");
            });

            ui.separator();

            // Export options
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.params.per_team, "Export per-team files");
                ui.label("Output:");
                ui.text_edit_singleline(&mut self.out_path);
            });

            // Status
            let status = self.status.lock().unwrap().clone();
            ui.label(format!("Status: {}", status));

            ui.horizontal(|ui| {
                if ui.button("Copy").clicked() {
                    // Merge to text in selected format and copy
                    let txt = rows_to_string(&self.rows, &self.headers, &self.current_delim());
                    ctx.copy_text(txt);
                    *self.status.lock().unwrap() = "Copied to clipboard.".into();
                }

                if ui.button("Export").clicked() {
                    let res = if self.params.per_team {
                        self.params.out = Some(PathBuf::from(&self.out_path));
                        self.params.per_team = true;
                        runner::run(&self.params, None)
                    } else {
                        // merged export path stays special-case
                        let path = PathBuf::from(&self.out_path);
                        if let Some(parent) = path.parent() {
                            if !parent.as_os_str().is_empty() { let _ = std::fs::create_dir_all(parent); }
                        }
                        let txt = rows_to_string(&self.rows, &self.headers, &self.current_delim());
                        std::fs::write(&path, txt)
                            .map(|_| runner::RunSummary { files_written: vec![path] })
                            .map_err(|e| e.into())
                    };

                    match res {
                        Ok(sum) => { /* same status reporting as before */ }
                        Err(e) => *self.status.lock().unwrap() = format!("Export error: {}", e),
                    }
                }

            });

            ui.separator();

            // Live table preview (egui_extras 0.32 API)
            let cols = self.headers.as_ref().map(|h| h.len())
                .or_else(|| self.rows.get(0).map(|r| r.len()))
                .unwrap_or(0);

            let mut table = TableBuilder::new(ui)
                .striped(true)
                .column(Column::auto().resizable(true).at_least(60.0));

            // Add remaining columns (we already added 1 above)
            for _ in 1..cols {
                table = table.column(Column::auto());
            }

            table
                .header(20.0, |mut header| {
                    if let Some(hs) = &self.headers {
                        for h in hs {
                            header.col(|ui| { ui.label(h); });
                        }
                    } else {
                        for i in 0..cols {
                            header.col(|ui| { ui.label(format!("Col {}", i + 1)); });
                        }
                    }
                })
                .body(|mut body| {
                    body.rows(18.0, self.rows.len(), |mut row| {
                        let row_idx = row.index();
                        if let Some(data) = self.rows.get(row_idx) {
                            for cell in data {
                                row.col(|ui| { ui.label(cell); });
                            }
                        }
                    });
                });

            // If any options changed, recollect immediately (synchronous for simplicity)
            if changed { self.sync_collect(); }
        });
    }
}

/* ---------- Progress adapter ---------- */
struct GuiProgress { status: Arc<Mutex<String>> }
impl runner::Progress for GuiProgress {
    fn begin(&mut self, total: usize) { *self.status.lock().unwrap() = format!("Starting… {} team(s)", total); }
    fn log(&mut self, msg: &str) { *self.status.lock().unwrap() = msg.to_string(); }
    fn item_done(&mut self, team_id: u32, path: &std::path::Path) {
        *self.status.lock().unwrap() = format!("Done team {} → {}", team_id, path.display());
    }
    fn update_status(&mut self, msg: &str) { *self.status.lock().unwrap() = msg.to_string(); }
}