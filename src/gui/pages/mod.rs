// src/gui/pages/mod.rs
use eframe::egui;
use std::sync::MutexGuard;

use crate::{
    config::{ 
        options::{ PageKind }, 
        state::{ AppState }},
    progress,
    store,
};

pub mod players;
pub mod game_results;

/// Light-weight context pages can use to interact with the app.
/// No long-held locks; page methods run quickly and return.
pub struct AppCtx<'a> {
    pub egui_ctx: &'a egui::Context,

    // Hold the mutex lock while the page works
    pub app_state: MutexGuard<'a, AppState>,

    // In-memory preview table
    pub headers: &'a mut Option<Vec<String>>,
    pub rows: &'a mut Vec<Vec<String>>,

    // Team list for convenience
    pub teams: &'a [(u32, String)],

    // Status callback (boxed so we can move it in)
    pub set_status: Box<dyn FnMut(String) + 'a>,
}

/// Optional column hints if you later want per-page sizing.
#[derive(Default, Clone, Copy)]
pub struct ColumnHints;

pub trait Page: Send + Sync + 'static {
    fn label(&self) -> &'static str;
    fn kind(&self) -> PageKind;

    /// Optional: default/fallback headers if the page fabricates a table.
    /// By default: none. Pages like Game Results can override.
    fn default_headers(&self) -> Option<&'static [&'static str]> {
        None
    }

    /// Optional: per-page column widths (in px-ish)
    fn preferred_column_widths(&self) -> Option<&'static [usize]> { None }

    /// Draw page-specific controls above the table.
    fn draw_controls(&self, _ui: &mut egui::Ui, _ctx: &mut AppCtx) {}

    /// Execute the page's scrape. Should update ctx.headers/rows and save to store.
    fn scrape(
        &self,
        _ctx: &AppCtx,
        progress: Option<&mut dyn progress::Progress>,
    ) -> Result<crate::store::DataSet, Box<dyn std::error::Error>>;

    /// Return the column index of a unique key if the page has one
    /// (e.g. Some(6) for Game Results = "Match id"); otherwise None.
    fn key_column(&self) -> Option<usize> { None }

    /// Merge freshly scraped `new` rows into `into` (canonical cache).
    /// Default behavior: replace everything.
    fn merge(&self, into: &mut store::DataSet, new: store::DataSet) {
        *into = new;
    }

    /// Optional: Filter rows by current selection (IDs/names differ per page)
    fn filter_rows_for_selection(
        &self,
        _selected_ids: &[u32],
        _teams: &[(u32, String)],
        rows: &Vec<Vec<String>>,
    ) -> Vec<Vec<String>> {
        rows.clone()
    }

    /// Optional: adapt headers/rows for on-screen display (e.g. hide columns).
    fn view_for_display(
        &self,
        _ctx: &AppCtx,
        headers: &Option<Vec<String>>,
        rows: &Vec<Vec<String>>,
    ) -> (Option<Vec<String>>, Vec<Vec<String>>) {
        (headers.clone(), rows.clone())
    }

    /// Optional: transform headers/rows for export/copy (e.g. hide columns)
    fn view_for_export(
        &self,
        _ctx: &AppCtx,
        headers: &Option<Vec<String>>,
        rows: &Vec<Vec<String>>,
    ) -> (Option<Vec<String>>, Vec<Vec<String>>) {
        // default: pass-through
        (headers.clone(), rows.clone())
    }

    /// Called when the tab becomes active (e.g., load cached dataset).
    fn on_enter(&self, ctx: &mut AppCtx) {
        // Default: try to load cached dataset for this page.
        if let Ok(ds) = store::load_dataset(&self.kind()) {
            *ctx.headers = ds.headers;
            *ctx.rows = ds.rows;
            (ctx.set_status)("Loaded local data".to_string());
        } else {
            (ctx.set_status)("Idle".to_string());
        }
    }
}
