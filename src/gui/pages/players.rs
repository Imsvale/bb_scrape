// src/gui/pages/players.rs
use eframe::egui;

use crate::{
    config::options::PageKind,
    progress::Progress,
    scrape,
    store::DataSet,
};

use super::{ AppCtx, Page };

pub struct PlayersPage;
pub static PAGE: PlayersPage = PlayersPage;

impl Page for PlayersPage {
    fn kind(&self) -> PageKind { PageKind::Players }
    fn label(&self) -> &'static str { "Players" }

    fn draw_controls(&self, ui: &mut egui::Ui, ctx: &mut AppCtx) {
        // Players-only toggle: Keep '#'
        ui.checkbox(
            &mut ctx.app_state.options.export.keep_hash,
            "Keep # in player number",
        );
    }

    fn scrape(
        &self,
        ctx: &AppCtx,
        progress: Option<&mut dyn Progress>,
    ) -> Result<DataSet, Box<dyn std::error::Error>> {
        let ds = scrape::collect_players(&ctx.app_state.options.scrape, progress)?;
        Ok(ds)
    }

    fn merge(&self, into: &mut DataSet, mut new: DataSet) {
        const TEAM_COL: usize = 3;

        // If the scrape gave us headers, accept them.
        if new.headers.is_some() {
            into.headers = new.headers.take();
        }

        // Which teams did this scrape cover?
        use std::collections::HashSet;
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
