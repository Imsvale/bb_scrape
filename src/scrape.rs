// src/scrape.rs
use std::error::Error;

use crate::{
    config::options::{PageKind, ScrapeOptions, TeamSelector},
    progress::Progress,
    specs::players,
};

pub struct DataSet {
    pub headers: Option<Vec<String>>,
    pub rows: Vec<Vec<String>>,
}

/// Top-level: dispatch on page kind and collect data (no IO).
pub fn run(
    scrape: &ScrapeOptions,
    progress: Option<&mut dyn Progress>,
) -> Result<DataSet, Box<dyn Error>> {
    match scrape.page {
        PageKind::Players => collect_players(scrape, progress),
        // PageKind::SeasonStats => todo!(),
        // PageKind::CareerStats => todo!(),
        // PageKind::Injuries    => todo!(),
        // PageKind::GameResults => todo!(),
    }
}

fn resolve_ids(sel: &TeamSelector) -> Vec<u32> {
    match sel {
        TeamSelector::All     => (0..32).collect(),
        TeamSelector::One(id) => vec![*id],
        TeamSelector::Ids(v)  => v.clone(),
    }
}

/// Collect players into memory according to selection.
/// Always returns canonical base data: headers present; numbers with '#'.
pub fn collect_players(
    scrape: &ScrapeOptions,
    mut progress: Option<&mut dyn Progress>,
) -> Result<DataSet, Box<dyn Error>> {
    let ids = resolve_ids(&scrape.teams);

    if let Some(p) = progress.as_deref_mut() {
        p.begin(ids.len());
        p.log("Fetching rosters…");
    }

    let mut merged_headers: Option<Vec<String>> = None;
    let mut rows: Vec<Vec<String>> = Vec::new();

    for id in ids {
        let bundle = players::fetch_and_extract(id)?;
        if merged_headers.is_none() {
            merged_headers = bundle.headers.clone();
        }
        rows.extend(bundle.rows);

        if let Some(p) = progress.as_deref_mut() {
            p.item_done(id);
        }
    }

    if let Some(p) = progress.as_deref_mut() {
        p.finish();
    }

    Ok(DataSet { headers: merged_headers, rows })
}

/* ---------------- Team-list helper (GUI/CLI can call) ---------------- */

pub fn list_teams() -> Vec<(u32, String)> {
    match crate::teams::load() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Warning: could not load team list: {}", e);
            (0u32..32).map(|id| (id, format!("Team {}", id))).collect()
        }
    }
}
