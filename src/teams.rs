// src/teams.rs
use std::{error::Error};
use crate::config::options::{PageKind, ScrapeOptions, TeamSelector};
use crate::{scrape, store};

fn dataset_to_pairs(ds: &scrape::DataSet) -> Vec<(u32, String)> {
    ds.rows.iter().filter_map(|r| {
        let id = r.get(0).and_then(|s| s.parse::<u32>().ok())?;
        let name = r.get(1).cloned().unwrap_or_default();
        Some((id, name))
    }).collect()
}

/// Load cached teams if present; otherwise scrape and cache.
pub fn load() -> Result<Vec<(u32, String)>, Box<dyn Error>> {
    if let Ok(ds) = store::load_dataset(&PageKind::Teams) {
        if !ds.rows.is_empty() {
            return Ok(ds.rows
                .iter()
                .filter_map(
                    |r| {
                        let id = r.get(0)?.parse::<u32>().ok()?;
                        Some((id, r.get(1).cloned().unwrap_or_default()))
                    }
                )
                .collect()
            );
        }
    }
    refresh()
}

/// Force refresh from the site and update cache.
pub fn refresh() -> Result<Vec<(u32, String)>, Box<dyn Error>> {
    let scrape_opts = ScrapeOptions { page: PageKind::Teams, teams: TeamSelector::All };
    let ds = scrape::run(&scrape_opts, None)?;
    // persist in the same raw “dataset” format you use elsewhere
    store::save_dataset(&PageKind::Teams, &store::Dataset { headers: ds.headers.clone(), rows: ds.rows.clone() })?;
    Ok(dataset_to_pairs(&ds))
}
