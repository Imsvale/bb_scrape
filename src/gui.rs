// src/gui.rs
use std::error::Error;
use crate::{
    params::{Params, PageKind},
    runner, 
    runner::Progress,
    teams,
};
use eframe::egui;

use std::sync::{Arc, Mutex};
use std::thread;

pub fn run() -> Result<(), Box<dyn Error>> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Brutalball Scraper",
        options,
        Box::new(|_cc| Ok(Box::new(App::default()))),
    )?;
    Ok(())
}

pub struct App {
    // data
    teams: Vec<(u32, String)>,
    selected: Vec<u32>,            // selected team IDs
    last_clicked: Option<usize>,   // for Shift selection range

    // params
    page: PageKind,
    include_headers: bool,
    keep_hash: bool,
    per_team: bool,

    // status/progress
    status: Arc<Mutex<String>>,
    running: bool,
}

impl Default for App {
    fn default() -> Self {
        // Attempt to load cached/scraped team names
        let teams = match teams::load() {
            Ok(v) if !v.is_empty() => v,
            Ok(_) | Err(_) => (0u32..32).map(|id| (id, format!("Team {}", id))).collect(),
        };

        Self {
            teams,
            selected: Vec::new(),
            last_clicked: None,

            page: PageKind::Players,
            include_headers: false,
            keep_hash: false,
            per_team: false,

            status: Arc::new(Mutex::new("Idle".to_string())),
            running: false,
        }
    }
}

/* ---------- GUI progress adapter ---------- */

struct GuiProgress {
    status: Arc<Mutex<String>>,
}
impl runner::Progress for GuiProgress {
    fn begin(&mut self, total: usize) {
        *self.status.lock().unwrap() = format!("Starting… {} team(s)", total);
    }
    fn log(&mut self, msg: &str) {
        *self.status.lock().unwrap() = msg.to_string();
    }
    fn item_done(&mut self, team_id: u32, path: &std::path::Path) {
        *self.status.lock().unwrap() =
            format!("Done team {} → {}", team_id, path.display());
    }
    fn update_status(&mut self, msg: &str) {
        *self.status.lock().unwrap() = msg.to_string();
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Tabs header (for now, only Players)
        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.page, PageKind::Players, "Players");
                // future: more tabs here
            });
        });

        // Left: multi-select teams
        egui::SidePanel::left("teams").resizable(false).show(ctx, |ui| {
            ui.heading("Teams");
            ui.separator();

            // helper: show either "All" hint or selected count
            let sel_note = if self.selected.is_empty() {
                "(none selected → all teams)"
            } else {
                "(selected)"
            };
            ui.label(sel_note);

            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                for (idx, (id, name)) in self.teams.iter().enumerate() {
                    let is_selected = self.selected.contains(id);
                    let resp = ui.selectable_label(is_selected, name);

                    if resp.clicked() && !self.running {
                        let input = ui.input(|i| i.clone());
                        if input.modifiers.ctrl {
                            // Toggle one
                            if is_selected {
                                self.selected.retain(|x| x != id);
                            } else {
                                self.selected.push(*id);
                            }
                            self.last_clicked = Some(idx);
                        } else if input.modifiers.shift {
                            // Range select
                            if let Some(last) = self.last_clicked {
                                let (lo, hi) = if last <= idx {(last, idx)} else {(idx, last)};
                                self.selected.clear();
                                for j in lo..=hi {
                                    self.selected.push(self.teams[j].0);
                                }
                            }
                        } else {
                            // Single select
                            self.selected.clear();
                            self.selected.push(*id);
                            self.last_clicked = Some(idx);
                        }
                    }
                }
            });
        });

        // Center: options + run
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Scrape Options");
            ui.separator();

            ui.checkbox(&mut self.include_headers, "Include headers");
            ui.checkbox(&mut self.keep_hash, "Keep hash in player numbers");
            ui.checkbox(&mut self.per_team, "Output per-team files (vs merged)");

            ui.separator();

            let status = self.status.lock().unwrap().clone();
            ui.label(format!("Status: {}", status));

            ui.add_space(8.0);

            let mut disabled = self.running;
            if disabled {
                let _ = ui.add_enabled_ui(false, |ui| {
                    if ui.button("Running…").clicked() {}
                });
            } else {
                if ui.button("Run Scrape").clicked() {
                    self.running = true;
                    *self.status.lock().unwrap() = "Starting…".to_string();

                    // Build Params from current GUI state
                    let mut p = Params {
                        page: self.page,
                        all: false,
                        one_team: None,
                        out: None,                   // runner will use defaults (out/players.csv or out/)
                        keep_hash: self.keep_hash,
                        include_headers: self.include_headers,
                        list_teams: false,
                        ids_filter: None,
                        per_team: self.per_team,
                    };

                    // all teams if none selected; otherwise filter
                    if self.selected.is_empty() {
                        p.all = true;
                    } else if self.selected.len() == 1 {
                        p.all = false;
                        p.one_team = Some(self.selected[0]);
                    } else {
                        p.all = true; // we’ll pass a filter
                        let mut filt = self.selected.clone();
                        filt.sort_unstable();
                        filt.dedup();
                        p.ids_filter = Some(filt);
                    }

                    // Launch runner in background thread
                    let status_arc = self.status.clone();
                    let ctx_clone = ctx.clone(); // to request repaint from worker
                    thread::spawn(move || {
                        let mut gp = GuiProgress { status: status_arc };
                        let res = runner::run(&p, Some(&mut gp));
                        // update final status
                        let msg = match res {
                            Ok(summary) => {
                                if summary.files_written.len() == 1 {
                                    format!("Done: {}", summary.files_written[0].display())
                                } else {
                                    format!("Done: {} files written", summary.files_written.len())
                                }
                            }
                            Err(e) => format!("Error: {}", e),
                        };
                        // push status
                        if let Some(m) = gp.status.lock().ok() {
                            drop(m); // unlock first
                        }
                        let _ = gp.update_status(&msg); // reuse trait method
                        // trigger redraw
                        ctx_clone.request_repaint();
                    });
                }
            }

            // Show a stop-gap “running” indicator
            if self.running {
                // Poll via a tiny label refresh; you could add a spinner or progress bar later.
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
            }
        });

        // Simple heuristic: if status starts with "Done" or "Error", mark not running
        {
            let s = self.status.lock().unwrap().clone();
            if s.starts_with("Done") || s.starts_with("Error") {
                self.running = false;
            }
        }
    }
}
