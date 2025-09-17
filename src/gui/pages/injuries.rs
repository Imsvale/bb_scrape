// src/gui/pages/injuries.rs
use std::error::Error;
use std::collections::HashSet;

use crate::{
    config::options::PageKind,
    config::state::AppState,
    progress::Progress,
    scrape,
    store::DataSet,
};

pub struct InjuriesPage;
pub static PAGE: InjuriesPage = InjuriesPage;

const HEADERS: [&str; 12] = [
    "S","W","Victim Team","Victim","DUR","SR0","SR1","Type","Offender Team","Offender","BRU","Bounty"
];

impl super::Page for InjuriesPage {
    fn title(&self) -> &'static str { "Injuries" }
    fn kind(&self) -> PageKind { PageKind::Injuries }

    fn default_headers(&self) -> Option<&'static [&'static str]> { Some(&HEADERS) }

    // Non-numeric columns for alignment: teams, names, type, bounty
    fn non_numeric_columns(&self) -> &'static [usize] { &[2,3,7,8,9,11] }

    fn preferred_column_widths(&self) -> Option<&'static [usize]> {
        Some(&[20, 20, 160, 160, 30, 30, 30, 140, 160, 160, 30, 120])
    }

    fn scrape(&self, _state: &AppState, mut progress: Option<&mut dyn Progress>) -> Result<DataSet, Box<dyn Error>> {
        if let Some(p) = progress.as_deref_mut() { p.begin(0); }
        scrape::collect_injuries(progress)
    }

    fn filter_row_indices_for_selection(
        &self,
        selected_ids: &[u32],
        teams: &[(u32, String)],
        rows: &Vec<Vec<String>>,
    ) -> Option<Vec<usize>> {
        if selected_ids.is_empty() { return Some(Vec::new()); }
        if selected_ids.len() == teams.len() { return Some((0..rows.len()).collect()); }
        let sel: HashSet<&str> = selected_ids
            .iter()
            .filter_map(|id| teams.iter().find(|(tid, _)| tid == id))
            .map(|(_, name)| name.as_str())
            .collect();
        let ix = rows.iter().enumerate().filter(|(_, r)| {
            r.get(2).map(|s| sel.contains(s.as_str())).unwrap_or(false) ||
            r.get(8).map(|s| sel.contains(s.as_str())).unwrap_or(false)
        }).map(|(i, _)| i).collect();
        Some(ix)
    }

    fn filter_rows_for_selection(
        &self,
        selected_ids: &[u32],
        teams: &[(u32, String)],
        rows: &Vec<Vec<String>>,
    ) -> Vec<Vec<String>> {
        if selected_ids.is_empty() || selected_ids.len() == teams.len() { return rows.clone(); }
        let sel: HashSet<&str> = selected_ids
            .iter()
            .filter_map(|id| teams.iter().find(|(tid, _)| tid == id))
            .map(|(_, name)| name.as_str()).collect();
        rows.iter().filter(|r| {
            r.get(2).map(|s| sel.contains(s.as_str())).unwrap_or(false) ||
            r.get(8).map(|s| sel.contains(s.as_str())).unwrap_or(false)
        }).cloned().collect()
    }
}

