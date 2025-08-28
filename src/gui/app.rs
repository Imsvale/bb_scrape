// src/gui/app.rs
use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, Mutex},
};

use eframe::egui;

use crate::{    
    store,
    teams,
    config::{
        state::{AppState, GuiState},
        options::{ PageKind::{ self, * }}}
};

use super::{
    pages::Page,
    router,
};

use crate::data::{RawData, FilteredData};

pub fn run(options: eframe::NativeOptions) -> Result<(), Box<dyn Error>> {
    eframe::run_native(
        "Brutalball Scraper",
        options,
        Box::new(|_cc| Ok(Box::new(App::new(AppState::default())))),
    )?;
    Ok(())
}

pub struct App {
    // single source of truth (UI thread only)
    pub state: AppState,

    // teams & selection UI (selection lives inside state.gui)
    pub teams: Vec<(u32, String)>,
    pub last_clicked: Option<usize>,

    // output text field UX (we map this <-> ExportOptions)
    pub out_path_text: String,
    pub out_path_dirty: bool,

    // in-memory display for CURRENT page
    pub headers: Option<Vec<String>>,
    pub rows: Vec<Vec<String>>,

    // status/progress (workers write here)
    pub status: Arc<Mutex<String>>,
    pub running: bool,

    // per-page canonical data + cached views
    pub raw_data: HashMap<PageKind, RawData>,
}

impl App {
    pub fn new(mut state: AppState) -> Self {
        // Teams list (fallback)
        let teams = match teams::load() {
            Ok(v) if !v.is_empty() => v,
            _ => (0u32..32).map(|id| (id, format!("Team {}", id))).collect(),
        };

        // Default selection: all
        state.gui = GuiState {
            selected_team_ids: teams.iter().map(|(id, _)| *id).collect(),
            ..GuiState::default()
        };

        let mut status = s!("Idle");

        // initial out path text
        let out_path_text = state.options.export.out_path().to_string_lossy().into();

        // canonical cache(s) from disk
        let mut raw_data: HashMap<PageKind, RawData> = HashMap::new();

        // Load cache for all pages
        for p in router::all_pages() {
            let k = p.kind();

            match store::load_dataset(&k) {
                Ok(ds) => {
                    if ds.rows.is_empty() {
                        logd!("Cache: {:?} is empty, skipping", k);
                        continue
                    }
                    if p.validate_cache(&ds) {
                        logf!("Cache: Loaded {:?} (rows={}, headers={})",
                            k, ds.row_count(),
                            ds.header_count()
                        );
                        raw_data.insert(k, RawData::new(k, ds));
                        status = s!("Loaded local data");
                    } else {
                        loge!("Cache: Invalid shape for {:?}, ignoring", k);
                    }
                }
                Err(e) => {
                    logd!("Cache: Missing {:?} ({})", k, e);
                }
            }
        }

        logf!("Init: teams={}, default page={:?}", teams.len(), Players);

        // initial view for Players
        let initial_kind = Players;
        let page = router::page_for(&initial_kind);

        let (headers, rows) = if let Some(raw) = raw_data.get(&initial_kind) {
            let fd = crate::data::FilteredData::from_raw(
                page,
                raw,
                &state.gui.selected_team_ids,
                &teams,
            );
            (fd.headers_owned(), fd.to_owned_rows())
        } else {
            let headers = page
                .default_headers()
                .map(|hs| hs.iter().map(|s| s!(*s)).collect());
            (headers, Vec::new())
        };

        Self {
            state,
            teams,
            last_clicked: None,
            out_path_text,
            out_path_dirty: false,
            headers,
            rows,
            status: Arc::new(Mutex::new(status)),
            running: false,
            raw_data,
        }
    }

    /* ---------- tiny helpers ---------- */

    #[inline]
    pub fn current_index(&self) -> usize { self.state.gui.current_page_index }

    #[inline]
    pub fn set_current_index(&mut self, idx: usize) { self.state.gui.current_page_index = idx; }

    #[inline]
    pub fn current_page_kind(&self) -> PageKind { router::all_pages()[self.current_index()].kind() }

    #[inline]
    pub fn current_page(&self) -> &'static dyn Page { router::all_pages()[self.current_index()] }

    #[inline]
    pub fn status<T: Into<String>>(&self, msg: T) {
        *self.status.lock().unwrap() = msg.into();
    }

    #[inline]
    pub fn set_selection_message(&self) {
        let n = self.state.gui.selected_team_ids.len();
        self.status(format!("Selection: {} team(s) — not scraped yet", n));
    }

    /// Mirror GUI selection → options.scrape.teams
    pub fn sync_gui_selection_into_scrape(&mut self) {
        use crate::config::options::TeamSelector;
        let teams_total = self.teams.len();
        let sel = &self.state.gui.selected_team_ids;

        self.state.options.scrape.teams = if sel.is_empty() {
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
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        eframe::egui::SidePanel::left("teams")
            .resizable(false)
            .show(ctx, |ui| {
                crate::gui::components::team_panel::draw(ui, self);
            });

        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            crate::gui::components::tabs::draw(ui, self);

            ui.separator();

            crate::gui::components::export_bar::draw(ui, self);

            ui.separator();

            crate::gui::components::data_table::draw(ui, self);
        });
    }
}
