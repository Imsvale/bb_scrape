// src/gui/app.rs
use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, Mutex}, thread,
};

use eframe::egui;

use crate::{    
    store,
    get_teams,
    config::{
        state::{AppState, GuiState},
        options::{ TeamSelector, PageKind::{ self, * }}}
};

use super::{
    components::*,
    pages::Page,
    router,
};

use crate::data::{RawData, Selection, SelectionView};
use super::actions::scrape::ScrapeOutcome;

pub fn run(options: eframe::NativeOptions) -> Result<(), Box<dyn Error>> {
    eframe::run_native(
        "Brutalball Scraper",
        options,
        Box::new(|_cc| Ok(Box::new(App::new(AppState::default())))),
    )?;
    Ok(())
}

pub struct App {
    // Single source of truth (UI thread only)
    pub state: AppState,

    // Teams & selection UI (selection lives inside state.gui)
    pub teams: Vec<(u32, String)>,
    pub last_clicked: Option<usize>,

    // Output text field UX (we map this <-> ExportOptions)
    pub out_path_text: String,
    pub out_path_dirty: bool,

    // Display/reference for current page
    pub headers: Option<Vec<String>>,
    pub row_ix: Arc<Vec<usize>>,

    // Status/progress (workers write here)
    pub status: Arc<Mutex<String>>,
    pub running: bool,
    pub scrape_handle: Option<thread::JoinHandle<ScrapeOutcome>>,

    // Per-page canonical data + cached views
    pub raw_data: HashMap<PageKind, RawData>,

    /// Cache of row indices per (page, selection key).
    /// Invalidation: bump state.teams_version on team list changes.
    /// Clear per-page on scrape merge (see Export button handler).
    pub row_ix_cache: HashMap<(PageKind, u32), Arc<Vec<usize>>>,

    // Column order per page (visual only). Values are source column indexes.
    pub col_order: HashMap<PageKind, Vec<usize>>,

    // Column widths per page, keyed by source column index (f32 px-ish)
    pub col_widths: HashMap<PageKind, Vec<f32>>,

    // Transient UI state for column drag & drop
    // Source column index (into the underlying dataset order)
    pub dragging_source_col: Option<usize>,
    // Preview target display index while dragging
    pub dragging_preview_to: Option<usize>,
    // Ghost visuals while dragging: capture pointer offset and width
    pub dragging_ghost_offset_x: f32,
    pub dragging_ghost_width: f32,
}

impl App {
    pub fn new(mut state: AppState) -> Self {
        // Teams list (fallback)
        let teams = match get_teams::load() {
            Ok(v) if !v.is_empty() => v,
            _ => (0u32..32).map(|id| (id, format!("Team {}", id))).collect(),
        };

        // Default selection: all
        state.gui = GuiState {
            selected_team_ids: teams.iter().map(|(id, _)| *id).collect(),
            ..GuiState::default()
        };

        let mut status = s!("Idle");

        // Initial out path text
        let out_path_text = state.options.export.out_path().to_string_lossy().into();

        // Canonical cache
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
                            k, ds.row_count(), ds.header_count());
                        raw_data.insert(k, RawData::new(k, ds));
                        status = s!("Loaded local data");
                    } else {
                        loge!("Cache: Invalid shape for {:?}, ignoring", k);
                    }
                }
                Err(e) => logd!("Cache: Missing {:?} ({})", k, e),
            }
        }

        logf!("Init: teams={}, default page={:?}", teams.len(), Players);

        // Initialize row index cache
        let mut row_ix_cache: 
            HashMap<(PageKind, u32), 
            Arc<Vec<usize>>> = HashMap::new();

        // Initial view for Players
        let initial_kind = Players;
        let page = router::page_for(&initial_kind);

        let (headers, row_ix) = if let Some(raw) = raw_data.get(&initial_kind) {

            // Headers from raw or page defaults
            let headers = match &raw.dataset().headers {
                Some(h) => Some(h.clone()),
                None => page
                    .default_headers()
                    .map(|hs| hs.iter().map(|s| s!(*s)).collect()),
            };

            // Indices via SelectionView with Selection
            let sel  = Selection { ids: &state.gui.selected_team_ids, teams: &teams };
            let key  = sel.to_key_mask();
            let view = SelectionView::from_raw(page, raw, sel);
            let row_ix = Arc::new(view.row_ix);
            // Tiny cache warm-up!
            row_ix_cache.insert((initial_kind, key), Arc::clone(&row_ix));
            (headers, row_ix)
        } else {
            let headers = page
                .default_headers()
                .map(|hs| hs.iter().map(|s| s!(*s)).collect());
            (headers, Arc::new(Vec::new()))
        };

        let mut app = Self {
            state,
            teams,
            last_clicked: None,
            out_path_text,
            out_path_dirty: false,
            headers,
            row_ix,
            status: Arc::new(Mutex::new(status)),
            running: false,
            scrape_handle: None,
            raw_data,
            row_ix_cache,
            col_order: HashMap::new(),
            col_widths: HashMap::new(),
            dragging_source_col: None,
            dragging_preview_to: None,
            dragging_ghost_offset_x: 0.0,
            dragging_ghost_width: 0.0,
        };

        // Load cached season if available, otherwise infer from cached Game Results
        if let Ok(Some(season)) = crate::store::load_season() {
            app.state.season = Some(season);
            logd!("Init: loaded cached season {}", season);
        } else {
            use crate::config::options::PageKind;
            if let Some(raw) = app.raw_data.get(&PageKind::GameResults) {
                if let Some(first) = raw.dataset().rows.get(0).and_then(|r| r.get(0)) {
                    if let Ok(season) = first.trim().parse::<u32>() {
                        app.state.season = Some(season);
                        let _ = crate::store::save_season(season);
                        logd!("Init: inferred season {} from cached Game Results", season);
                    }
                }
            }
        }

        app
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
        self.status(format!("Selection: {} team(s)", n));
    }

    /// Mirror GUI selection → options.scrape.teams
    pub fn sync_gui_selection_into_scrape(&mut self) {
        
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

    /// Update the team list. If any (id,name) pair differs from the current set,
    /// clear the selection-index cache and rebuild the view; otherwise leave it alone.
    pub fn set_teams(&mut self, new_teams: Vec<(u32, String)>) {
        if self.teams == new_teams {
            logd!("Teams: unchanged — keeping selection cache");
            return;
        }

        self.teams = new_teams;
        logf!("Teams: changed — clearing selection cache");

        // Optional: clamp selection to the new team ids (defensive)
        // Build a tiny lookup to keep only valid ids
        {
            use std::collections::HashSet;
            let valid: HashSet<u32> = self.teams.iter().map(|(id, _)| *id).collect();
            self.state.gui.selected_team_ids.retain(|id| valid.contains(id));
            if self.state.gui.selected_team_ids.is_empty() {
                // keep UX friendly: if selection became empty due to changes, select all
                self.state.gui.selected_team_ids = self.teams.iter().map(|(id, _)| *id).collect();
            }
            // Mirror selection into scrape options
            self.sync_gui_selection_into_scrape();
        }

        // Clear all cached index views; page data didn’t change, but name-based filters might.
        self.row_ix_cache.clear();

        // Rebuild current display using the updated names
        self.rebuild_view();
    }

    /// Recompute headers/rows for the current page from canonical raw_data,
    /// applying the current GUI team selection.
    /// Uses a row-index cache if present.
    pub fn rebuild_view(&mut self) {
        let kind = self.current_page_kind();
        let page = self.current_page();

        if let Some(raw) = self.raw_data.get(&kind) {

            // Prefer page-provided defaults when available; otherwise use dataset headers.
            self.headers = page
                .default_headers()
                .map(|hs| hs.iter().map(|s| s.to_string()).collect())
                .or_else(|| raw.dataset().headers.clone());

            let sel  = Selection { ids: &self.state.gui.selected_team_ids, teams: &self.teams };
            let mask = sel.to_key_mask();
            let key  = (kind, mask);

            if let Some(ix) = self.row_ix_cache.get(&key) {
                // Cache hit → build from indices
                self.row_ix = ix.clone();
            } else {
                // Cache miss → compute via fast path/fallback, then store indices
                let view = SelectionView::from_raw(page, raw, sel);
                let arc_ix = Arc::new(view.row_ix);
                self.row_ix_cache.insert(key, arc_ix.clone());
                self.row_ix = arc_ix;
            }
            // Ensure column order is initialized or resized to current cols
            let cols = self.headers.as_ref()
                .map(|h| h.len())
                .or_else(|| raw.dataset().rows.get(0).map(|r| r.len()))
                .unwrap_or_else(|| page.default_headers().map(|h| h.len()).unwrap_or(0));
            let ord = self.col_order.entry(kind).or_insert_with(|| (0..cols).collect());
            if ord.len() != cols {
                *ord = (0..cols).collect();
            }
        } else {
            self.headers = page
                .default_headers()
                .map(|hs| hs.iter().map(|s| s!(*s)).collect());
            self.row_ix = Arc::new(Vec::new());
            // Reset column order for empty dataset / defaults
            let cols = self.headers.as_ref().map(|h| h.len()).unwrap_or(0);
            if cols > 0 {
                self.col_order.insert(kind, (0..cols).collect());
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        crate::gui::actions::scrape::poll(self);

        if self.running {
            // Repaint while spinner animates; throttle a bit to save CPU
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
        }

        egui::SidePanel::left("teams")
            .resizable(false)
            .min_width(self.state.gui.team_panel_width)
            .max_width(self.state.gui.team_panel_width)
            .default_width(self.state.gui.team_panel_width)
            .show(ctx, |ui| {
                team_panel::draw(ui, self);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            tabs::draw(ui, self);

            ui.separator();

            action_buttons::draw(ui, self);

            ui.separator();

            data_table::draw(ui, self);
        });
    }
}
