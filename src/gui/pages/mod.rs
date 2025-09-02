// src/gui/pages/mod.rs
use eframe::egui;
use std::error::Error;

use crate::{
    config::{ 
        options::{ PageKind }, 
        state::{ AppState }},
    progress::Progress,
    store::DataSet,
};

pub mod players;
pub mod game_results;

/// Optional column hints if you later want per-page sizing.
#[derive(Default, Debug, Clone, Copy)]
pub struct ColumnHints;

pub trait Page: Send + Sync + 'static {
    fn title(&self) -> &'static str;
    fn kind(&self) -> PageKind;

    /// Optional: default/fallback headers if the page fabricates a table.
    /// By default: none. Pages like Game Results can override.
    fn default_headers(&self) -> Option<&'static [&'static str]> {
        None
    }

    /// Optional: per-page column widths (in px-ish)
    fn preferred_column_widths(&self) -> Option<&'static [usize]> { None }

    /// Static list of non-numeric column indices for alignment purposes.
    /// Default: none (treat all columns as numeric).
    fn non_numeric_columns(&self) -> &'static [usize] { &[] }

    /// Draw page-specific controls above the table. 
    /// Return true if any control changed, so the app can rebuild the view.
    fn draw_controls(&self, _ui: &mut egui::Ui, _state: &mut AppState) -> bool { false }

    /// Execute the page's scrape.
    fn scrape(
        &self,
        _state: &AppState,
        progress: Option<&mut dyn Progress>,
    ) -> Result<DataSet, Box<dyn Error>>;

    /// Merge freshly scraped `new` rows into `into` (canonical cache).
    /// Default behavior: replace everything.
    fn merge(&self, into: &mut DataSet, new: DataSet) { *into = new; }

    /// Filter rows by current selection
    fn filter_rows_for_selection(
        &self,
        _selected_ids: &[u32],
        _teams: &[(u32, String)],
        rows: &Vec<Vec<String>>,
    ) -> Vec<Vec<String>> { rows.clone() }

    /// Filter row *indices* by current selection
    fn filter_row_indices_for_selection(
        &self,
        _selected_ids: &[u32],
        _teams: &[(u32, String)],
        _rows: &Vec<Vec<String>>,
    ) -> Option<Vec<usize>> { None }

    /// Optional: transform headers/rows for export/copy (e.g. hide columns)
    fn view_for_export(
        &self,
        _state: &AppState,
        headers: &Option<Vec<String>>,
        rows: &Vec<Vec<String>>,
    ) -> (Option<Vec<String>>, Vec<Vec<String>>) { (headers.clone(), rows.clone()) }

    fn validate_cache(&self, _ds: &DataSet) -> bool { true }

    /// Optional: validate a freshly scraped dataset before we accept it.
    /// Default = accept everything.
    fn validate_scrape(
        &self,
        _state: &AppState,
        _teams: &[(u32, String)],
        _new: &DataSet,
    ) -> Result<(), String> {
        Ok(())
    }

    /// Whether "per-team export" is applicable on this page.
    /// If false, the checkbox is grayed out.
    fn per_team_applicable(&self) -> bool { true }
}
