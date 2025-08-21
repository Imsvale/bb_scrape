// src/runner.rs
#![allow(unused)]
use std::collections::HashMap;
use std::error::Error;
use std::path::{Path, PathBuf};

use crate::params::PLAYERS_SUBDIR;
use crate::{
    file,
    params::{Params, PageKind, DEFAULT_OUT_DIR, DEFAULT_SINGLE_FILE},
    specs,
    store,
};

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

/// Top-level runner: dispatch on page kind and run.
/// `progress` can be None (no UI updates) or Some(&mut impl Progress).
pub fn run(
    params: &Params,
    progress: Option<&mut dyn Progress>,
) -> Result<RunSummary, Box<dyn Error>> {
    match params.page {
        PageKind::Players => get_players(params, progress),
        // PageKind::SeasonStats => todo!(),
        // PageKind::CareerStats => todo!(),
        // PageKind::Season => todo!(),
        // PageKind::Injuries => todo!(),
    }
}

// Collect players into memory according to selection/filter.
// Does NOT write files. Honors include_headers/keep_hash; merges rows.
pub fn collect_players(params: &Params) -> Result<DataSet, Box<dyn std::error::Error>> {
    // Build ID list
    let mut ids: Vec<u32> = if params.all {
        // GUI: default “all teams selected”
        // CLI: same if no one_team & no ids_filter
        (0..32).collect()
    } else {
        vec![params.one_team.expect("one_team is required when not 'all'")]
    };

    if let Some(filter) = &params.ids_filter {
        let mut f = filter.clone();
        f.sort_unstable();
        ids.retain(|id| f.binary_search(id).is_ok());
    }

    let mut merged_headers: Option<Vec<String>> = None;
    let mut rows: Vec<Vec<String>> = Vec::new();

    for (i, id) in ids.iter().copied().enumerate() {
        let bundle = specs::players::fetch_and_extract(id, params.keep_hash, params.include_headers)?;
        // take headers only once (if requested and available)
        if merged_headers.is_none() && params.include_headers {
            merged_headers = bundle.headers.clone();
        }
        rows.extend(bundle.rows);
    }

    Ok(DataSet { headers: merged_headers, rows })
}

/* ---------------- Players implementation ---------------- */
fn get_players(
    params: &Params,
    mut progress: Option<&mut dyn Progress>,
) -> Result<RunSummary, Box<dyn Error>> {
    let mut ids: Vec<u32> = if params.all {
        (0..32).collect()
    } else {
        vec![params.one_team.expect("one_team required when not --all")]
    };

    if let Some(filter) = &params.ids_filter {
        // Intersect with filter. Filter is assumed sorted; if not, sort first.
        let mut f = filter.clone();
        f.sort_unstable();
        ids.retain(|id| f.binary_search(id).is_ok());
    }

    if ids.is_empty() {
        if let Some(p) = progress.as_deref_mut() {
            p.log("No team IDs to process (after filtering).");
        }
        return Ok(RunSummary { files_written: Vec::new() });
    }

    if let Some(p) = progress.as_deref_mut() {
        p.begin(ids.len());
    }

    let mut written = Vec::with_capacity(ids.len());

    let delim = params.format;

    if !params.single_file {
        // ---------- PER-TEAM FILES ----------
        // -o must be a directory (create if missing). If omitted → "./out"
        let outdir = params
            .out
            .clone()
            .unwrap_or_else(|| PathBuf::from(DEFAULT_OUT_DIR));
        // only enforce dir semantics in per-team mode
        let outdir = file::normalize_dir_path(outdir.to_string_lossy().as_ref())?;
        file::ensure_directory(&outdir)?;

        let mut seen: HashMap<String, usize> = HashMap::new();
        let mut persisted_headers = false;

        for id in ids {
            let bundle = specs::players::fetch_and_extract(id, params.keep_hash, params.include_headers)?;
            let stem = file::sanitize_team_filename(&bundle.team_name, id);
            let path = file::resolve_team_filename(&outdir, &stem, &mut seen, delim);

            if !persisted_headers {
                if let Some(h) = &bundle.headers {
                    let _ = store::save_players_headers(h);
                    persisted_headers = true;
                }
            }

            file::write_rows_start(&path, bundle.headers.as_deref(), delim)?;
            file::append_rows(&path, &bundle.rows, delim)?;
            if let Some(p) = progress.as_deref_mut() {
                p.item_done(id, &path);
            }
            written.push(path);
        }
    } else {
        // ---------- MERGED SINGLE FILE ----------
        let out_hint = params.out
            .clone()
            .unwrap_or_else(|| PathBuf::from(DEFAULT_OUT_DIR).join(PLAYERS_SUBDIR).join(DEFAULT_SINGLE_FILE));

        let resolved = if out_hint.is_dir() || crate::file::looks_like_dir_hint(&out_hint) {
            file::ensure_directory(&out_hint)?;
            out_hint.join(DEFAULT_SINGLE_FILE)
        } else {
            if let Some(parent) = out_hint.parent() {
                if !parent.as_os_str().is_empty() {
                    file::ensure_directory(parent)?;
                }
            }
            out_hint
        };

        // We will write the header ONCE, from the first successful team bundle.
        let mut wrote_header = false;

        for (i, id) in ids.iter().copied().enumerate() {
            let bundle = specs::players::fetch_and_extract(id, params.keep_hash, params.include_headers)?;
            if !wrote_header {
                // use site headers if present and include_headers==true
                file::write_rows_start(&resolved, bundle.headers.as_deref(), delim)?;
                wrote_header = true;

                if let Some(h) = &bundle.headers {
                    let _ = store::save_players_headers(h);
                }
            } else {
                // ensure file exists already (write_rows_start creates/truncates only once)
                // then append rows for subsequent teams
                // (no-op here; just fall through)
            }
            file::append_rows(&resolved, &bundle.rows, delim)?;

            if let Some(p) = progress.as_deref_mut() {
                p.item_done(id, &resolved);
            }
            if i == 0 {
                written.push(resolved.clone());
            }
        }
    }

    Ok(RunSummary { files_written: written })
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
