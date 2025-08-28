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

    specs, 
    teams, 
};

/// Top-level: dispatch on page kind and collect data (no IO).
pub fn run(
    scrape: &ScrapeOptions,
    progress: Option<&mut dyn Progress>,
) -> Result<DataSet, Box<dyn Error>> {
    match scrape.page {
        // PageKind::Teams         => collect_teams(),
        Teams         => collect_teams(progress),
        Players       => collect_players(scrape, progress),
        SeasonStats   => todo!(),
        CareerStats   => todo!(),
        GameResults   => collect_game_results(),
        Injuries      => todo!(),
    }
}

fn resolve_ids(sel: &TeamSelector) -> Vec<u32> {
    match sel {
        TeamSelector::All     => (0..32).collect(),
        TeamSelector::One(id) => vec![*id],
        TeamSelector::Ids(v)  => v.clone(),
    }
}

fn collect_teams(mut progress: Option<&mut dyn Progress>)
    -> Result<DataSet, Box<dyn Error>>
{
    if let Some(p) = progress.as_deref_mut() {
        p.begin(1);
        p.log("Fetching teams...");
    }
    let bundle = specs::teams::fetch()?;
    if let Some(p) = progress.as_deref_mut() {
        p.item_done(999_999); // or add a non-team sentinel in the trait later
        p.finish();
    }
    Ok(DataSet { headers: bundle.headers, rows: bundle.rows })
}

/// Collect players into memory according to selection.
/// Always returns canonical base data: headers present; numbers with '#'.
pub fn collect_players(
    scrape: &ScrapeOptions,
    mut progress: Option<&mut dyn Progress>,
) -> Result<DataSet, Box<dyn Error>> {

    if let Ok(bundle) = specs::teams::fetch() {
        // cache, but ignore any IO error (best-effort)
        let _ = store::save_dataset(
            &Teams, 
            &DataSet { headers: bundle.headers, rows: bundle.rows }
        );
    }

    let ids = resolve_ids(&scrape.teams);

    if let Some(p) = progress.as_deref_mut() {
        p.begin(ids.len());
        p.log("Fetching rosters…");
    }

    // Concurrency
    type FetchOk = (u32, specs::players::RosterBundle);
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
                    let result = match specs::players::fetch_and_extract(team_id) {
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
                    p.item_done(id);
                }
            }
            Ok(Err((id, msg))) => {
                if let Some(p) = progress.as_deref_mut() {
                    p.log(&format!("Team {id}: {msg}"));
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

/* ---------------- Team-list helper (GUI/CLI can call) ---------------- */

pub fn list_teams() -> Vec<(u32, String)> {
    match teams::load() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Warning: could not load team list: {}", e);
            (0u32..32).map(|id| (id, format!("Team {}", id))).collect()
        }
    }
}

fn collect_game_results() -> Result<DataSet, Box<dyn Error>> {
    let bundle = specs::game_results::fetch()?;
    Ok(DataSet { headers: bundle.headers, rows: bundle.rows })
}
