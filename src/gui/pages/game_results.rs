use std::error::Error;
use std::collections::HashSet;
use eframe::egui;

use crate::config::options::PageKind;
use crate::config::state::AppState;
use crate::progress::Progress;
use crate::store::DataSet;
use crate::scrape;

use super::Page;

pub struct GameResultsPage;

pub static PAGE: GameResultsPage = GameResultsPage;

const HEADERS: [&str; 7] = [
    "S","W","Home team","Home","Away","Away team","Match id"
];

impl Page for GameResultsPage {
    fn title(&self) -> &'static str { "Game Results" }
    fn kind(&self) -> PageKind { PageKind::GameResults }

    fn default_headers(&self) -> Option<&'static [&'static str]> {
        Some(&HEADERS)
    }

    fn preferred_column_widths(&self) -> Option<&'static [usize]> {
        // Season, Week, Home Team, Home, Away, Away Team, Match id
        Some(&[25, 25, 200, 30, 30, 200, 92])
    }

    fn draw_controls(&self, ui: &mut egui::Ui, state: &mut AppState) -> bool {
        // Page-specific toggles
        let mut changed = false;
        ui.horizontal(|ui| {
            changed |= ui.checkbox(
                &mut state.gui.game_results_show_match_id, 
                "Include match id")
                .changed();
        });
        changed
    }

    fn scrape(
        &self,
        _state: &AppState,
        progress: Option<&mut dyn Progress>,
    ) -> Result<DataSet, Box<dyn Error>> {
        // Use the top-level router to run the correct scraper
        let ds = scrape::collect_game_results(progress)?;
        Ok(ds)
    }

    fn key_column(&self) -> Option<usize> { Some(6) }

    fn merge(&self, into: &mut DataSet, new: DataSet) {
        const KEY: usize = 6;

        // 1) Headers: adopt if ours are missing
        if into.headers.is_none() && new.headers.is_some() {
            into.headers = new.headers;
        }

        // 2) Build indexes on existing rows:
        use std::collections::HashMap;

        // by ID (only for rows that already have an ID)
        let mut by_id: HashMap<String, usize> = HashMap::new();

        // provisional index: (season, week, home, away) -> row index
        // only for rows that DON'T have an ID yet
        let mut provisional: HashMap<(String, String, String, String), usize> = HashMap::new();

        for (i, r) in into.rows.iter().enumerate() {
            let id = r.get(KEY).map(|s| s.as_str()).unwrap_or("");
            if !id.is_empty() {
                by_id.insert(id.to_string(), i);
            } else {
                // only index rows lacking an id
                let k = (
                    r.get(0).cloned().unwrap_or_default(), // season
                    r.get(1).cloned().unwrap_or_default(), // week
                    r.get(2).cloned().unwrap_or_default(), // home
                    r.get(5).cloned().unwrap_or_default(), // away
                );
                provisional.insert(k, i);
            }
        }

        // 3) Integrate new rows
        for r in new.rows {
            let id = r.get(KEY).map(|s| s.as_str()).unwrap_or("");

            if !id.is_empty() {
                // prefer ID upsert
                if let Some(&idx) = by_id.get(id) {
                    into.rows[idx] = r;
                    continue;
                }
                // otherwise try promoting a provisional match
                let k = (
                    r.get(0).cloned().unwrap_or_default(),
                    r.get(1).cloned().unwrap_or_default(),
                    r.get(2).cloned().unwrap_or_default(),
                    r.get(5).cloned().unwrap_or_default(),
                );
                if let Some(&idx) = provisional.get(&k) {
                    into.rows[idx] = r;
                    // row now has an ID; our provisional index is stale, but
                    // we don't need it after the merge pass, so we don't rebalance it.
                    continue;
                }
                // brand new game with ID
                by_id.insert(id.to_string(), into.rows.len());
                into.rows.push(r);
            } else {
                // No ID yet: use/replace provisional row if present; else append
                let k = (
                    r.get(0).cloned().unwrap_or_default(),
                    r.get(1).cloned().unwrap_or_default(),
                    r.get(2).cloned().unwrap_or_default(),
                    r.get(5).cloned().unwrap_or_default(),
                );
                if let Some(&idx) = provisional.get(&k) {
                    into.rows[idx] = r;
                } else {
                    provisional.insert(k, into.rows.len());
                    into.rows.push(r);
                }
            }
        }
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

    fn view_for_export(
        
        &self,
        state: &AppState,
        headers: &Option<Vec<String>>,
        rows: &Vec<Vec<String>>,
    ) -> (Option<Vec<String>>, Vec<Vec<String>>) {
        if state.gui.game_results_show_match_id {
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

    fn validate_cache(&self, ds: &DataSet) -> bool {
        // Allow blank. Otherwise require exactly 7 columns in headers+rows.
        let hdr_ok = ds.headers.as_ref().map(|h| h.len() == 7).unwrap_or(true);
        let rows_ok = ds.rows.iter().all(|r| r.len() == 7);
        hdr_ok && rows_ok
    }

}