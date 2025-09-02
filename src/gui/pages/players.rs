// src/gui/pages/players.rs
use eframe::egui;
use std::error::Error;
use std::collections::HashSet;

use crate::{
    config::options::PageKind::{ self, * },
    config::state::AppState,
    progress::Progress,
    scrape,
    store::DataSet,
};

use super::{ Page };

pub struct PlayersPage;
pub static PAGE: PlayersPage = PlayersPage;

impl Page for PlayersPage {
    fn kind(&self) -> PageKind { Players }
    fn title(&self) -> &'static str { "Players" }

    // Non-numeric: 0 Name, 2 Race, 3 Team. Column 1 (Number) and 4..end are numeric.
    fn non_numeric_columns(&self) -> &'static [usize] { &[0, 2, 3] }

    fn draw_controls(&self, ui: &mut egui::Ui, state: &mut AppState) -> bool {
        // Players-only toggle: Keep '#'
        let mut changed = false;
        changed |= ui.checkbox(
            &mut state.options.export.keep_hash,
            "Keep # in player number")
            .changed();
        changed
    }

    fn scrape(
        &self,
        state: &AppState,
        mut progress: Option<&mut dyn Progress>,
    ) -> Result<DataSet, Box<dyn Error>> {

        if let Some(p) = progress.as_deref_mut() {
            p.begin(1);
        }
        
        let ds = scrape::collect_players(&state.options.scrape, progress)?;
        Ok(ds)
    }

    fn merge(&self, into: &mut DataSet, mut new: DataSet) {
        const TEAM_COL: usize = 3;

        // If the scrape gave us headers, accept them.
        if new.headers.is_some() {
            into.headers = new.headers.take();
        }

        // Which teams did this scrape cover?
        
        let mut scraped_teams: HashSet<String> = HashSet::new();
        for r in &new.rows {
            if let Some(t) = r.get(TEAM_COL) {
                scraped_teams.insert(t.clone());
            }
        }

        // Drop any existing rows for those teams.
        if !scraped_teams.is_empty() {
            into.rows.retain(|r| {
                let keep = r
                    .get(TEAM_COL)
                    .map(|t| !scraped_teams.contains(t))
                    .unwrap_or(true);
                keep
            });
        }

        // Append the freshly scraped rows.
        into.rows.extend(new.rows.into_iter());
    }

    fn filter_row_indices_for_selection(
        &self,
        selected_ids: &[u32],
        teams: &[(u32, String)],
        rows: &Vec<Vec<String>>,
    ) -> Option<Vec<usize>> {
        const TEAM_COL: usize = 3;

        let selected_names: HashSet<&str> = selected_ids.iter()
            .filter_map(|id| teams.iter().find(|(tid, _)| tid == id))
            .map(|(_, n)| n.as_str())
            .collect();

        let ix = rows.iter().enumerate()
            .filter(|(_, r)| r.get(TEAM_COL)
                .map(|t| selected_names.contains(t.as_str()))
                .unwrap_or(false))
            .map(|(i, _)| i)
            .collect();

        Some(ix)
    }

    fn filter_rows_for_selection(
        &self,
        selected_ids: &[u32],
        teams: &[(u32, String)],
        rows: &Vec<Vec<String>>,
    ) -> Vec<Vec<String>> {

        if selected_ids.len() == teams.len() {
            return rows.clone();
        }

        if selected_ids.is_empty() {
            return Vec::new();
        }

        const TEAM_COL: usize = 3;

        use std::collections::HashSet;
        let selected_names: HashSet<&str> = selected_ids.iter()
            .filter_map(|id| teams.iter().find(|(tid, _)| tid == id))
            .map(|(_, name)| name.as_str())
            .collect();

        rows.iter()
            .filter(|r| r.get(TEAM_COL)
                .map(|t| selected_names.contains(t.as_str()))
                .unwrap_or(false))
            .cloned()
            .collect()
    }
}
