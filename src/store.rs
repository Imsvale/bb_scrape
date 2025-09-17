// src/store.rs
use std::{
    fs::{ self, File },
    io::{ BufWriter, Result },
    path::PathBuf,
};

use crate::file::{parse_rows, write_row};
use crate::config::options::PageKind::{self, *};
use crate::config::consts::{STORE_DIR, STORE_SEP};

/// Load cached dataset for a given page (if present).
/// Assumes first row is headers when present.
pub fn load_dataset(kind: &PageKind) -> Result<DataSet> {
    let path = store_path(kind);
    let text = fs::read_to_string(&path)?;
    let mut rows = parse_rows(&text, STORE_SEP);

    let headers = if !rows.is_empty() {
        Some(rows.remove(0))
    } else {
        None
    };

    Ok(DataSet { headers, rows })
}

/// Persist a canonical dataset for a given page.
/// Always writes headers first (if present), then rows.
pub fn save_dataset(kind: &PageKind, ds: &DataSet) -> Result<PathBuf> {
    let dir = store_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }

    let path = store_path(kind);
    let file = File::create(&path)?;
    let mut w = BufWriter::new(file);

    if let Some(h) = &ds.headers {
        write_row(&mut w, h, STORE_SEP)?;
    }
    for r in &ds.rows {
        write_row(&mut w, r, STORE_SEP)?;
    }
    // BufWriter drops will flush; explicit flush not required

    Ok(path)
}

fn store_dir() -> PathBuf {
    PathBuf::from(STORE_DIR)
}

fn store_path(kind: &PageKind) -> PathBuf {
    store_dir().join(page_filename(kind))
}

fn page_filename(kind: &PageKind) -> &'static str {
    match kind {
        Teams         => "teams",
        Players       => "players",
        SeasonStats   => "season_stats",
        CareerStats   => "career_stats",
        Injuries      => "injuries",
        GameResults   => "game_results",
    }
}

// ---- Season persistence ----

pub fn season_path() -> PathBuf { store_dir().join("season") }

/// Save current season number to `.store/season`.
pub fn save_season(season: u32) -> Result<PathBuf> {
    let dir = store_dir();
    if !dir.exists() { std::fs::create_dir_all(&dir)?; }
    let p = season_path();
    std::fs::write(&p, season.to_string())?;
    Ok(p)
}

/// Load season from `.store/season` if present.
pub fn load_season() -> Result<Option<u32>> {
    let p = season_path();
    if !p.exists() { return Ok(None); }
    let s = std::fs::read_to_string(p)?;
    Ok(s.trim().parse::<u32>().ok())
}

#[derive(Clone, Debug)]
pub struct DataSet {
    pub headers: Option<Vec<String>>,
    pub rows: Vec<Vec<String>>,
}

// ---- New common helpers on DataSet ----

use crate::gui::pages::Page;

impl DataSet {
    pub fn header_index(&self, name: &str) -> Option<usize> {
        self.headers.as_ref()?.iter()
            .position(|s| s.eq_ignore_ascii_case(name))
    }

    pub fn indexes_filtered_by_selection(
        &self,
        page: &dyn Page,
        selected_team_ids: &[u32],
        teams: &[(u32, String)],
    ) -> Vec<usize> {
        // Reuse your existing page filter but return indexes to avoid cloning.
        let mut ix = Vec::with_capacity(self.rows.len());
        for (i, row) in self.rows.iter().enumerate() {
            // Quick path by sharing the predicate:
            // If you want max reuse, expose page.filter_predicate(...) and call it here.
            // For now, keep it simple:
            let tmp = vec![row.clone()];
            if !page.filter_rows_for_selection(selected_team_ids, teams, &tmp).is_empty() {
                ix.push(i);
            }
        }
        ix
    }

    /// Returns headers or the page's default/fallback headers if none present.
    pub fn headers_or_defaults(&self, page: &dyn Page) -> Option<Vec<String>> {
        match &self.headers {
            Some(h) => Some(h.clone()),
            None => page
                .default_headers()
                .map(|hs| hs.iter().map(|s| s.to_string()).collect()),
        }
    }

    /// Apply the page's team selection filter to rows.
    pub fn rows_filtered_by_selection(
        &self,
        page: &dyn Page,
        selected_team_ids: &[u32],
        teams: &[(u32, String)],
    ) -> Vec<Vec<String>> {
        page.filter_rows_for_selection(selected_team_ids, teams, &self.rows)
    }

    /// Convenience counters.
    pub fn row_count(&self) -> usize { self.rows.len() }
    pub fn header_count(&self) -> usize { self.headers.as_ref().map(|h| h.len()).unwrap_or(0) }
}
