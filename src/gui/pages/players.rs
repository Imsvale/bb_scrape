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
        progress: Option<&mut dyn Progress>,
    ) -> Result<DataSet, Box<dyn Error>> {
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
}
