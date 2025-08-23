// src/runner.rs
use std::collections::HashMap;
use std::error::Error;
use std::path::{Path, PathBuf};

use crate::config::options::TeamSelector;
use crate::{
    config::{
        options::{ AppOptions, ScrapeOptions, ExportOptions, ExportType, PageKind },
        state::{ AppState },
    },
    file,
    specs,
    store,
};

/// Top-level runner: dispatch on page kind and run.
/// `progress` can be None (no UI updates) or Some(&mut impl Progress).
pub fn run(
    scrape: &ScrapeOptions,
    export: &ExportOptions,
    progress: Option<&mut dyn Progress>,
) -> Result<RunSummary, Box<dyn Error>> {
    match scrape.page {
        PageKind::Players => get_players(scrape, export, progress),
        // PageKind::SeasonStats => todo!(),
        // PageKind::CareerStats => todo!(),
        // PageKind::Season => todo!(),
        // PageKind::Injuries => todo!(),
    }
}

/// Optional progress sink for GUI/CLI.
/// Implement this in the frontend (GUI: update labels/progress bar; CLI: print lines).
pub trait Progress {
    fn begin(&mut self, _total: usize) {}
    fn log(&mut self, _msg: &str) {}
    fn item_done(&mut self, _team_id: u32, _path: &Path) {}
    fn update_status(&mut self, msg: &str) {}
}

pub struct DataSet {
    pub headers: Option<Vec<String>>,
    pub rows: Vec<Vec<String>>,
}

/// A no-op progress sink you can pass when you don't care.
pub struct NullProgress;
impl Progress for NullProgress {}

/// Summary of what was produced.
pub struct RunSummary {
    pub files_written: Vec<PathBuf>,
}

fn resolve_ids(sel: &TeamSelector) -> Vec<u32> {
    match sel {
        TeamSelector::All       => (0..32).collect(),
        TeamSelector::One(id)   => vec![*id],
        TeamSelector::Ids(v)    => v.clone(),
    }
}

// Collect players into memory according to selection/filter.
// Does NOT write files. Honors include_headers/keep_hash; merges rows.
pub fn collect_players(
    scrape: &ScrapeOptions,
    export: &ExportOptions,
) -> Result<DataSet, Box<dyn std::error::Error>> {
    let ids = resolve_ids(&scrape.teams);

    let mut merged_headers: Option<Vec<String>> = None;
    let mut rows: Vec<Vec<String>> = Vec::new();

    for id in ids {
        let bundle = specs::players::fetch_and_extract( // TODO: Investigate use of export options
            id,
            export.keep_hash,
            export.include_headers,
        )?;
        if merged_headers.is_none() && export.include_headers {
            merged_headers = bundle.headers.clone();
        }
        rows.extend(bundle.rows);
    }

    Ok(DataSet { headers: merged_headers, rows })
}

/* ---------------- Players implementation ---------------- */
fn get_players(
    scrape: &ScrapeOptions,
    export: &ExportOptions,
    mut progress: Option<&mut dyn Progress>,
) -> Result<RunSummary, Box<dyn Error>> {
    let mut ids = resolve_ids(&scrape.teams);

    if ids.is_empty() {
        if let Some(p) = progress.as_deref_mut() {
            p.log("No team IDs to process.");
        }
        return Ok(RunSummary { files_written: Vec::new() });
    }

    if let Some(p) = progress.as_deref_mut() {
        p.begin(ids.len());
    }

    match export.export_type {
        ExportType::PerTeam => {
            let outdir = export.out_path(); // directory
            if !outdir.as_os_str().is_empty() {
                std::fs::create_dir_all(&outdir)?;
            }

            // Resolve duplicate stems like "Lions", "Lions-2", ...

            let mut seen: HashMap<String, usize> = HashMap::new();
            let mut files = Vec::with_capacity(ids.len());

            for id in ids.drain(..) {
                let bundle = specs::players::fetch_and_extract(
                    id,
                    export.keep_hash,
                    export.include_headers,
                )?;

                // Prefer explicit team name from the bundle if available
                let raw_stem = if !bundle.team_name.is_empty() {
                    bundle.team_name.clone()
                } else {
                    format!("team_{id:02}")
                };

                let stem = file::sanitize_team_filename(&raw_stem, id);

                let stem = {
                    let c = seen.entry(stem).and_modify(|n| *n += 1).or_insert(0);
                    if *c == 0 { seen.keys().last().unwrap().clone() }
                    else { format!("{}--{}", seen.keys().last().unwrap(), *c + 1) }
                };

                let file_name = format!("{}.{}", stem, export.format.ext());
                let path = outdir.join(file_name);

                let txt = to_export_string(
                    &bundle.headers,
                    &bundle.rows,
                    export.include_headers,
                    export.keep_hash,
                    export.delim(),
                );
                std::fs::write(&path, txt)?;

                if let Some(p) = progress.as_deref_mut() {
                    p.item_done(id, &path);
                }
                files.push(path);

                if let Some(h) = &bundle.headers {
                    let _ = store::save_players_headers(h);
                }
            }
            Ok(RunSummary { files_written: files })
        }

        ExportType::SingleFile => {
            let path = export.out_path(); // full file path
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)?;
                }
            }

            let ds = collect_players(scrape, export)?;
            let txt = to_export_string(
                &ds.headers,
                &ds.rows,
                export.include_headers,
                export.keep_hash,
                export.delim(),
            );

            std::fs::write(&path, txt)?;
            Ok(RunSummary { files_written: vec![path] })
        }
    }
}

/* ---------------- Team-list helper (GUI/CLI can call) ---------------- */

/// Fetch all team IDs and names.
/// CLI and GUI should call this instead of hardcoding.
pub fn list_teams() -> Vec<(u32, String)> {
    match crate::teams::load() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Warning: could not load team list: {}", e);
            // fallback stub (always 32 teams, numbered)
            (0u32..32).map(|id| (id, format!("Team {}", id))).collect()
        }
    }
}
