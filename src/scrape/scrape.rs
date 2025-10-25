// src/scrape.rs
use std::{
    error::Error, thread, time::Duration,
    sync::{ mpsc, Arc, atomic::{ AtomicUsize, Ordering }}
};

use crate::{
    config::options::{PageKind::*, ScrapeOptions, TeamSelector},
    config::consts::{ WORKERS, REQUEST_PAUSE_MS, JITTER_MS },

    progress::Progress, 
    store::{ self, DataSet },
    get_teams, 
};

use super::*;

fn resolve_ids(sel: &TeamSelector) -> Vec<u32> {
    match sel {
        TeamSelector::All     => (0..32).collect(),
        TeamSelector::One(id) => vec![*id],
        TeamSelector::Ids(v)  => v.clone(),
    }
}

pub fn list_teams() -> Vec<(u32, String)> {
    match get_teams::load() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Warning: could not load team list: {}", e);
            (0u32..32).map(|id| (id, format!("Team {}", id))).collect()
        }
    }
}

pub fn collect_teams(mut progress: Option<&mut dyn Progress>)
    -> Result<DataSet, Box<dyn Error>>
{
    if let Some(p) = progress.as_deref_mut() {
        p.log("Refreshing teams…");
    }
    let bundle = teams::fetch()?;
    Ok(DataSet { headers: bundle.headers, rows: bundle.rows })
}

/// Collect players into memory according to selection.
/// Always returns canonical base data: headers present; numbers with '#'.
pub fn collect_players(
    scrape: &ScrapeOptions,
    mut progress: Option<&mut dyn Progress>,
) -> Result<DataSet, Box<dyn Error>> {

    if let Ok(bundle) = teams::fetch() {
        // cache, but ignore any IO error (best-effort)
        let _ = store::save_dataset(
            &Teams, 
            &DataSet { headers: bundle.headers, rows: bundle.rows }
        );
    }

    let ids = resolve_ids(&scrape.teams);

    // Load team names for progress reporting
    let team_names: std::collections::HashMap<u32, String> = list_teams()
        .into_iter()
        .collect();

    if let Some(p) = progress.as_deref_mut() {
        p.begin(ids.len());
    }

    // Concurrency
    type FetchOk = (u32, players::RosterBundle);
    type FetchErr = (u32, String);
    
    let ids_arc = Arc::new(ids.clone());
    let counter = Arc::new(AtomicUsize::new(0));
    let (res_tx, res_rx) = mpsc::channel::<Result<FetchOk, FetchErr>>();

    let workers = WORKERS.min(ids.len()).max(1);

    // Spawn workers

    for _ in 0..workers {
        let ids = Arc::clone(&ids_arc);
        let idx = Arc::clone(&counter);
        let tx = res_tx.clone();

        thread::spawn(
            move || {
                loop {
                    let i = idx.fetch_add(1, Ordering::Relaxed);
                    if i >= ids.len() {
                        break;
                    }
                    let team_id = ids[i];
                    let result = match players::fetch_and_extract(team_id) {
                        Ok(bundle) => Ok((team_id, bundle)),
                        Err(e) => Err((team_id, e.to_string())),
                    };
                    let _ = tx.send(result);
                    let jitter = (team_id as u64) % JITTER_MS;
                    thread::sleep(Duration::from_millis(REQUEST_PAUSE_MS + jitter)); // be polite
                }
            }
        );
    }
    drop(res_tx); // main thread is sole receiver now

    // Aggregate results
    let mut headers: Option<Vec<String>> = None;
    let mut per_team: Vec<(u32, Vec<Vec<String>>)> = Vec::new();

    for _ in 0..ids_arc.len() {
        match res_rx.recv() {
            Ok(Ok((id, bundle))) => {
                if headers.is_none() {
                    headers = bundle.headers.clone();
                }
                per_team.push((id, bundle.rows));
                if let Some(p) = progress.as_deref_mut() {
                    let team_name = team_names.get(&id)
                        .map(|s| s.as_str())
                        .unwrap_or("Unknown Team");
                    p.item_done(id, team_name);
                }
            }
            Ok(Err((id, msg))) => {
                if let Some(p) = progress.as_deref_mut() {
                    let team_name = team_names.get(&id)
                        .map(|s| s.as_str())
                        .unwrap_or("Unknown Team");
                    p.item_failed(id, team_name);

                    loge!("Team {id}: {msg}");
                }
            }
            Err(_) => break, // workers ended early; bail gracefully
        }
    }

    if let Some(p) = progress.as_deref_mut() {
        p.finish();
    }

    // Sort
    per_team.sort_by_key(|(id, _)| *id);
    let mut rows: Vec<Vec<String>> = Vec::new();
    for (_, mut team_rows) in per_team {
        rows.append(&mut team_rows);
    }

    Ok(DataSet { headers, rows })
}

pub fn collect_game_results(_progress: Option<&mut dyn Progress>,) -> Result<DataSet, Box<dyn Error>> {
    let bundle = scrape::game_results::fetch()?;
    Ok(DataSet { headers: bundle.headers, rows: bundle.rows })
}
