// src/gui/pages/game_results.rs
use std::error::Error;
use std::collections::{HashMap, HashSet};
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
    "S","W","Home","H","A","Away","Match id"
];

impl Page for GameResultsPage {
    fn title(&self) -> &'static str { "Game Results" }
    fn kind(&self) -> PageKind { PageKind::GameResults }

    fn default_headers(&self) -> Option<&'static [&'static str]> {
        Some(&HEADERS)
    }

    // Non-numeric: 2 Home team, 5 Away team. All other columns are numeric.
    fn non_numeric_columns(&self) -> &'static [usize] { &[2, 5] }

    fn preferred_column_widths(&self) -> Option<&'static [usize]> {
        // Season, Week, Home Team, Home, Away, Away Team, Match id
        Some(&[20, 20, 170, 20, 20, 170, 50])
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
        mut progress: Option<&mut dyn Progress>,
    ) -> Result<DataSet, Box<dyn Error>> {
        if let Some(p) = progress.as_deref_mut() {
            p.begin(0);
        }
        scrape::collect_game_results(progress) // → Result<DataSet>
    }

    

    /// Game Results: the scrape is whole-season, so accept it atomically.
    fn merge(&self, into: &mut DataSet, new: DataSet) {
        // We already validated `new` in actions::scrape before calling merge.
        *into = new;
    }


    fn filter_row_indices_for_selection(
        &self,
        selected_ids: &[u32],
        teams: &[(u32, String)],
        rows: &Vec<Vec<String>>,
    ) -> Option<Vec<usize>> {

        // Game Results can compute indices in O(n), so we *always* return Some(...).
        // Reserve `None` only for pages that *cannot* provide indices (force fallback).

        // 1) Fast paths that keep caller simple and avoid any fallback logic:

        // (a) Nothing selected → empty projection.
        if selected_ids.is_empty() {
            return Some(Vec::new());
        }

        // (b) All teams selected → identity projection (all row indices).
        if selected_ids.len() == teams.len() {
            return Some((0..rows.len()).collect());
        }

        // 2) Partial selection → build a set of selected team *names*.
        //
        // Page rows store names (not ids), so we map ids → names using the UI's
        // canonical (id, name) list. Using &str avoids allocating new Strings.
        let sel: HashSet<&str> = selected_ids
            .iter()
            .filter_map(|id| teams.iter().find(|(tid, _)| tid == id))
            .map(|(_, name)| name.as_str())
            .collect();

        // 3) Columns in this page shape:
        //    0 Season, 1 Week, 2 Home team, 3 Home result, 4 Away result, 5 Away team, 6 Match id
        //
        // Keep any row where either the Home team or the Away team is in the selection.
        let ix = rows.iter().enumerate()
            .filter(|(_, r)| {
                r.get(2).map(|s| sel.contains(s.as_str())).unwrap_or(false) ||
                r.get(5).map(|s| sel.contains(s.as_str())).unwrap_or(false)
            })
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

    fn validate_scrape(
        &self,
        _state: &AppState,
        teams: &[(u32, String)],
        new: &DataSet,
    ) -> Result<(), String> {
        
        let n = teams.len();
        if n == 0 || n > 32 {
            return Err(format!("Validator expects 1..=32 teams; got {}", n));
        }
        
        // We assume 32 teams (fits in u32). If it ever changes, bump here.
        let full_mask: u32 = if n == 32 { u32::MAX } else { (1u32 << n) - 1 };

        // Map canonical team name -> bit (use the UI’s teams list as ground truth)
        let mut bit_of: HashMap<&str, u32> = HashMap::with_capacity(teams.len());
        for (idx, (_, name)) in teams.iter().enumerate() {
            bit_of.insert(name.as_str(), 1u32 << idx);
        }

        // Per (season, week) mask of teams seen
        let mut week_mask: HashMap<(String, String), u32> = HashMap::new();

        // Duplicate detection
        let mut seen_match_id: HashSet<&str> = HashSet::new();
        // Unordered “game signature” per (S,W): (min(team), max(team))
        let mut seen_game: HashSet<(String, String, String, String)> = HashSet::new();

        for r in &new.rows {
            if r.len() < 7 {
                return Err("Row has fewer than 7 columns (S,W,Home team,Home,Away,Away team,Match id)".into());
            }
            let s     = r[0].trim().to_string();
            let w     = r[1].trim().to_string();
            let home  = r[2].trim();
            let away  = r[5].trim();
            let mid   = r[6].trim();

            if home.is_empty() || away.is_empty() {
                return Err(format!("Empty team name in S={} W={}", s, w));
            }
            if home == away {
                return Err(format!("Home==Away in S={}, W={} ({})", s, w, home));
            }

            // Duplicate game (by team pair) within the same week, independent of match id
            let (a, b) = if home <= away { (home.to_string(), away.to_string()) }
                         else            { (away.to_string(), home.to_string()) };
            if !seen_game.insert((s.clone(), w.clone(), a, b)) {
                return Err(format!("Duplicate game by teams in S={}, W={}", s, w));
            }

            // Duplicate match id (only if present; future games often blank)
            if !mid.is_empty() && !seen_match_id.insert(mid) {
                return Err(format!("Duplicate match id {} in S={}, W={}", mid, s, w));
            }

            // Bitmask: each team exactly once per week
            let entry = week_mask.entry((s, w)).or_insert(0u32);

            let hb = *bit_of.get(home)
                .ok_or_else(|| format!("Unknown team name '{}' in results", home))?;
            if (*entry & hb) != 0 {
                return Err(format!("Team '{}' appears twice in the same week", home));
            }
            *entry |= hb;

            let ab = *bit_of.get(away)
                .ok_or_else(|| format!("Unknown team name '{}' in results", away))?;
            if (*entry & ab) != 0 {
                return Err(format!("Team '{}' appears twice in the same week", away));
            }
            *entry |= ab;
        }

        // Every week must have exactly all teams
        for ((s, w), mask) in week_mask {
            if mask != full_mask {
                return Err(format!(
                    "Incomplete/extra teams in S={}, W={} (got mask {:032b}, expected {:032b})",
                    s, w, mask, full_mask
                ));
            }
        }

        Ok(())
    }
}
