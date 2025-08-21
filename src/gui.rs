// src/gui.rs
#![allow(unused)]
use std::{
    error::Error,
    path::PathBuf, 
    sync::{ Arc, Mutex }, 
    thread,
};

use eframe::{egui};
use egui::{RichText, Layout, Align, TextWrapMode, Direction};
use egui_extras::{TableBuilder, Column};

use crate::{
    csv::{
        rows_to_string, to_export_string, Delim },
    params::{
        PageKind, Params, DEFAULT_SINGLE_FILE, DEFAULT_OUT_DIR, PLAYERS_SUBDIR },
    runner::{self, Progress},
    store,
    teams,
};

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
    out_path_dirty: bool,

    // in-memory data
    headers: Option<Vec<String>>,
    rows: Vec<Vec<String>>,

    // status/progress
    status: Arc<Mutex<String>>,
    running: bool,
}

impl App {
    pub fn new(params: Params) -> Self {
        let mut headers = None;
        let mut rows = Vec::new();
        let mut status = "Idle".to_string();

        if let Ok(ds) = store::load_players_local() {
            headers = ds.headers;
            rows = ds.rows;
            status = "Loaded local data".to_string();
        }
        // prune stray empty rows just in case
        rows.retain(|r| !r.is_empty() && !(r.len() == 1 && r[0].trim().is_empty()));

        let teams = match teams::load() {
            Ok(v) if !v.is_empty() => v,
            _ => (0u32..32).map(|id| (id, format!("Team {}", id))).collect(),
        };
        let selected = teams.iter().map(|(id, _)| *id).collect();

        let out_path = if params.single_file {
            format!("{}/{}/{}", DEFAULT_OUT_DIR, PLAYERS_SUBDIR, DEFAULT_SINGLE_FILE)
        } else {
            format!("{}/{}/", DEFAULT_OUT_DIR, PLAYERS_SUBDIR)
        };

        Self {
            params,
            teams,
            selected,
            last_clicked: None,
            fmt: UiFormat::Csv,
            out_path,
            out_path_dirty: false,
            headers,
            rows,
            status: Arc::new(Mutex::new(status)),
            running: false,
        }
    }

    fn is_blank_row(r: &[String]) -> bool { r.iter().all(|s| s.trim().is_empty()) }


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

                if let Some(ref h) = self.headers {
                    let _ = store::save_players_headers(h);
                }

                *self.status.lock().unwrap() = "Ready".into();
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
                }
                if ui.button("None").clicked() {
                    self.selected.clear();
                }
            });
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {

                let mut needs_status = false;

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
                        
                        needs_status = true;
                    }
                }
                if needs_status {
                    *self.status.lock().unwrap() = "Selection changed — not scraped yet.".into();
                }
            });
        });

        // Center: options + display + copy/export
        egui::CentralPanel::default().show(ctx, |ui| {

            ui.heading("Players");

            ui.separator();

            // Format selector (CSV/TSV) — affects Copy/Export only
            let prev_fmt = self.fmt;
            ui.horizontal(|ui| {
                ui.label("Format:");
                ui.selectable_value(&mut self.fmt, UiFormat::Csv, "CSV");
                ui.selectable_value(&mut self.fmt, UiFormat::Tsv, "TSV");
            });

            if self.fmt != prev_fmt {
                // push into params so runner/export uses it everywhere
                self.params.format = match self.fmt {
                    UiFormat::Csv => Delim::Csv,
                    UiFormat::Tsv => Delim::Tsv,
               };

                // If the output path wasn't manually edited and we're in single-file mode,
                // flip the filename extension .csv <-> .tsv to match the new format.
                if !self.out_path_dirty && self.params.single_file {
                    if let Some(dot) = self.out_path.rfind('.') {
                        let (stem, ext) = self.out_path.split_at(dot);
                        match self.params.format {
                            Delim::Csv if ext.eq_ignore_ascii_case(".tsv") => {
                                self.out_path = format!("{stem}.csv");
                            }
                            Delim::Tsv if ext.eq_ignore_ascii_case(".csv") => {
                                self.out_path = format!("{stem}.tsv");
                            }
                            _ => {}
                        }
                    }
                }
            }

            ui.checkbox(&mut self.params.include_headers, "Include headers");
            ui.checkbox(&mut self.params.keep_hash, "Keep hash in player numbers");

            // Export options
            ui.horizontal(|ui| {
                let single_file_checkbox = ui.checkbox(&mut self.params.single_file, "Single file");
                ui.label("Output:");
                let resp = ui.text_edit_singleline(&mut self.out_path);
                if resp.changed() {
                    self.out_path_dirty = true;
                }

                let ext = match self.fmt {
                    UiFormat::Csv => "csv",
                    UiFormat::Tsv => "tsv"
                };

                let single_file_before = self.params.single_file;
                if single_file_checkbox.changed() {
                    if !self.out_path_dirty {
                        if self.params.single_file {
                            // switched to merged: use single file
                            self.out_path = format!("{DEFAULT_OUT_DIR}/{PLAYERS_SUBDIR}/{DEFAULT_SINGLE_FILE}.{ext}");
                        } else {
                            // switched to per-team: use directory
                            self.out_path = format!("{DEFAULT_OUT_DIR}/{PLAYERS_SUBDIR}/");
                        }
                    }
                }
            });

            ui.horizontal(|ui| {
                if ui.button("Copy").clicked() {
                    let txt = to_export_string(
                        &self.headers,
                        &self.rows,
                        self.params.include_headers,
                        self.params.keep_hash,
                        self.params.format, // CSV / TSV
                    );
                    ctx.copy_text(txt);
                    *self.status.lock().unwrap() = "Copied to clipboard".into();
                }

                if ui.button("Export").clicked() {
                    let res = if self.params.single_file {
                        // SINGLE-FILE EXPORT: File name as given, or default
                        let path = PathBuf::from(&self.out_path);
                        if let Some(parent) = path.parent() {
                            if !parent.as_os_str().is_empty() { let _ = std::fs::create_dir_all(parent); }
                        }
                        let txt = rows_to_string(&self.rows, &self.headers, self.params.format);
                        std::fs::write(&path, txt)
                            .map(|_| runner::RunSummary { files_written: vec![path] })
                            .map_err(|e| e.into())
                    } else {
                        // MULTI-FILE EXPORT: File names from team names
                        self.params.out = Some(PathBuf::from(&self.out_path));
                        self.params.format = match self.fmt { UiFormat::Csv => Delim::Csv, UiFormat::Tsv => Delim::Tsv };
                        runner::run(&self.params, None)
                    };

                    match res {
                        Ok(sum) => { /* same status reporting as before */ }
                        Err(e) => *self.status.lock().unwrap() = format!("Export error: {}", e),
                    }
                }

                let red = egui::Color32::from_rgb(220, 30, 30);
                let black = egui::Color32::BLACK;

                if ui.add(
                    egui::Button::new(egui::RichText::new("SCRAPE").color(black).strong())
                        .fill(red)
                ).clicked() {
                    self.sync_collect();
                }

                // Status
                let status = self.status.lock().unwrap().clone();
                ui.label(format!("Status: {}", status));
            });

            ui.separator();

            // Live table preview (egui_extras 0.32 API)
            let cols = self.headers.as_ref().map(|h| h.len())
                .or_else(|| self.rows.get(0).map(|r| r.len()))
                .unwrap_or(0);

            // Canonical columns (Name, Number, Race, Team) get wider baselines.
            // Numeric columns get compact baseline and are resizable by the user.
            let mut table = TableBuilder::new(ui)
                .striped(true)
                .min_scrolled_height(0.0)
                .column(Column::auto().resizable(true).at_least(180.0)) // Name
                .column(Column::auto().resizable(true).at_least(30.0))  // Number
                .column(Column::auto().resizable(true).at_least(120.0)) // Race
                .column(Column::auto().resizable(true).at_least(140.0));// Team

            // Remaining columns (usually numeric): narrower
            for _ in 4..cols {
                table = table.column(Column::auto().resizable(true).at_least(30.0));
            }

            table
                .header(24.0, |mut header| {
                    if let Some(hs) = &self.headers {
                        for h in hs {
                            header.col(|ui| {
                                ui.scope(|ui| {
                                    ui.style_mut().wrap_mode = Some(TextWrapMode::Extend); // no wrap
                                    ui.with_layout(
                                        Layout::left_to_right(Align::Center),
                                        // Bold header; let theme decide color for now
                                        |ui| {ui.label(RichText::new(h).strong()); }
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
                .body(|mut body| {
                    body.rows(20.0, self.rows.len(), |mut row| {
                        let row_idx = row.index();
                        if let Some(data) = self.rows.get(row_idx) {
                            for (ci, cell) in data.iter().enumerate() {
                                row.col(|ui| {
                                    ui.scope(|ui| {
                                        ui.style_mut().wrap_mode = Some(TextWrapMode::Extend); // no wrap

                                        let rt = RichText::new(cell);
                                        if ci == 0 {
                                            // Name: left align
                                            ui.label(rt);
                                        } else {
                                            // Others: center horizontally
                                            ui.with_layout(
                                                Layout::left_to_right(Align::Center),
                                                |ui| { ui.label(rt); }
                                            );
                                        }
                                    });
                                });
                            }
                        }
                    });
                });
        
        }); // End egui::CentralPanel::default().show
    }
}

fn apply_keep_hash(rows: &[Vec<String>], keep_hash: bool) -> Vec<Vec<String>> {
    // Copy-on-transform; we only touch column 1 if present.
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        if r.len() > 1 {
            let mut nr = r.clone();
            if keep_hash {
                if !nr[1].starts_with('#') && !nr[1].is_empty() {
                    nr[1] = format!("#{}", nr[1]);
                }
            } else {
                nr[1] = nr[1].trim_start_matches('#').to_string();
            }
            out.push(nr);
        } else {
            out.push(r.clone());
        }
    }
    out
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