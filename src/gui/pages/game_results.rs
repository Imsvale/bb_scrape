use std::error::Error;
use std::collections::HashSet;
use eframe::egui;

use crate::config::options::PageKind;
use crate::progress::Progress;
use crate::store::DataSet;
use crate::scrape;

use super::{AppCtx, Page};

pub struct GameResultsPage;

pub static PAGE: GameResultsPage = GameResultsPage;

const HEADERS: [&str; 7] = [
    "Season","Week","Home team","Home result","Away result","Away team","Match id"
];

impl Page for GameResultsPage {
    fn label(&self) -> &'static str { "Game Results" }
    fn kind(&self) -> PageKind { PageKind::GameResults }

    fn default_headers(&self) -> Option<&'static [&'static str]> {
        Some(&HEADERS)
    }

    fn preferred_column_widths(&self) -> Option<&'static [usize]> {
        // Season, Week, Home Team, Home, Away, Away Team, Match id
        Some(&[52, 44, 200, 72, 72, 200, 92])
    }

    fn draw_controls(&self, ui: &mut egui::Ui, ctx: &mut AppCtx) {
        // Page-specific toggles
        ui.horizontal(|ui| {
            ui.label("Columns:");
            ui.checkbox(&mut ctx.app_state.gui.game_results_show_match_id, "Include match id");
        });
    }

    fn scrape(
        &self,
        ctx: &AppCtx,
        progress: Option<&mut dyn Progress>,
    ) -> Result<DataSet, Box<dyn Error>> {
        // Use the top-level router to run the correct scraper
        let ds = scrape::run(&ctx.app_state.options.scrape, progress)?;
        Ok(ds)
    }

    fn key_column(&self) -> Option<usize> { Some(6) }

    fn merge(&self, into: &mut DataSet, mut new: DataSet) {
        const KEY: usize = 6;

        // Prefer fresh headers when provided.
        if new.headers.is_some() {
            into.headers = new.headers.take();
        }

        use std::collections::HashMap;
        // Build a map of existing rows keyed by match id.
        let mut by_id: HashMap<String, Vec<String>> =
            HashMap::with_capacity(into.rows.len().saturating_add(new.rows.len()));

        for r in std::mem::take(&mut into.rows) {
            if let Some(k) = r.get(KEY).cloned() {
                by_id.insert(k, r);
            }
        }

        // Upsert new rows by the same key.
        for r in new.rows {
            if let Some(k) = r.get(KEY).cloned() {
                by_id.insert(k, r); // replace if exists, insert if not
            }
        }

        // Rebuild rows in a deterministic order: (Season asc, Week asc).
        let mut rows: Vec<Vec<String>> = by_id.into_values().collect();
        rows.sort_by(|a, b| {
            let sa = a.get(0).and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
            let sb = b.get(0).and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
            let wa = a.get(1).and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
            let wb = b.get(1).and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
            (sa, wa).cmp(&(sb, wb))
        });

        into.rows = rows;
    }

    fn filter_rows_for_selection(
        &self,
        selected_ids: &[u32],
        teams: &[(u32, String)],
        rows: &Vec<Vec<String>>,
    ) -> Vec<Vec<String>> {
        if selected_ids.is_empty() || selected_ids.len() == teams.len() {
            return rows.clone();
        }

        // Build selected name set
        let sel: HashSet<&str> = selected_ids.iter()
            .filter_map(|id| teams.iter().find(|(tid, _)| tid == id))
            .map(|(_, name)| name.as_str())
            .collect();

        // Columns: 0 Season, 1 Week, 2 Home team, 3 Home, 4 Away, 5 Away team, 6 Match id
        let filtered: Vec<Vec<String>> = rows.iter()
            .filter(|r| {
                r.get(2).map(|s| sel.contains(s.as_str())).unwrap_or(false)
                || r.get(5).map(|s| sel.contains(s.as_str())).unwrap_or(false)
            })
            .cloned()
            .collect();

        filtered
    }

    fn view_for_display(
        &self,
        ctx: &AppCtx,
        headers: &Option<Vec<String>>,
        rows: &Vec<Vec<String>>,
    ) -> (Option<Vec<String>>, Vec<Vec<String>>) {
        if ctx.app_state.gui.game_results_show_match_id {
            return (headers.clone(), rows.clone());
        }
        let new_headers = headers.as_ref().map(|hs| {
            let mut h = hs.clone();
            if !h.is_empty() { h.pop(); }
            h
        });
        let new_rows = rows.iter().map(|r| {
            let mut c = r.clone();
            if !c.is_empty() { c.pop(); }
            c
        }).collect();
        (new_headers, new_rows)
    }

    fn view_for_export(
        
        &self,
        ctx: &AppCtx,
        headers: &Option<Vec<String>>,
        rows: &Vec<Vec<String>>,
    ) -> (Option<Vec<String>>, Vec<Vec<String>>) {
        if ctx.app_state.gui.game_results_show_match_id {
            return (headers.clone(), rows.clone());
        }
        // Drop the last column from headers + rows if present
        let new_headers = headers.as_ref().map(|hs| {
            let mut h = hs.clone();
            if !h.is_empty() { h.pop(); }
            h
        });
        let new_rows = rows.iter().map(|r| {
            let mut c = r.clone();
            if !c.is_empty() { c.pop(); }
            c
        }).collect();
        (new_headers, new_rows)
    }
}