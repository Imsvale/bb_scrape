// src/runner.rs
#![allow(unused)]
use std::collections::HashMap;
use std::error::Error;
use std::path::{Path, PathBuf};

use crate::{
    specs,
    params::{Params, PageKind, DEFAULT_OUT_DIR, DEFAULT_MERGED_FILENAME},
    file::{
        append_rows, ensure_directory, normalize_dir_path,
        resolve_team_filename, sanitize_team_filename, write_rows_start,
    }
};

/// Optional progress sink for GUI/CLI.
/// Implement this in the frontend (GUI: update labels/progress bar; CLI: print lines).
pub trait Progress {
    fn begin(&mut self, _total: usize) {}
    fn log(&mut self, _msg: &str) {}
    fn item_done(&mut self, _team_id: u32, _path: &Path) {}
    fn update_status(&mut self, msg: &str) {}
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

    if params.all && params.per_team {
        // ---------- PER-TEAM FILES ----------
        // -o must be a directory (create if missing). If omitted → "./out"
        let outdir = params
            .out
            .clone()
            .unwrap_or_else(|| PathBuf::from(DEFAULT_OUT_DIR));
        // only enforce dir semantics in per-team mode
        let outdir = normalize_dir_path(outdir.to_string_lossy().as_ref())?;
        ensure_directory(&outdir)?;

        let mut seen: HashMap<String, usize> = HashMap::new();
        for id in ids {
            let bundle = specs::players::fetch_and_extract(id, params.keep_hash, params.include_headers)?;
            let stem = sanitize_team_filename(&bundle.team_name, id);
            let path = resolve_team_filename(&outdir, &stem, &mut seen);

            write_rows_start(&path, bundle.headers.as_deref())?;
            append_rows(&path, &bundle.rows)?;
            if let Some(p) = progress.as_deref_mut() {
                p.item_done(id, &path);
            }
            written.push(path);
        }
    } else {
        // ---------- MERGED SINGLE FILE ----------
        let out_hint = params.out
            .clone()
            .unwrap_or_else(|| PathBuf::from(DEFAULT_OUT_DIR).join(DEFAULT_MERGED_FILENAME));

        let resolved = if out_hint.is_dir() || crate::file::looks_like_dir_hint(&out_hint) {
            ensure_directory(&out_hint)?;
            out_hint.join(DEFAULT_MERGED_FILENAME)
        } else {
            if let Some(parent) = out_hint.parent() {
                if !parent.as_os_str().is_empty() {
                    ensure_directory(parent)?;
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
                write_rows_start(&resolved, bundle.headers.as_deref())?;
                wrote_header = true;
            } else {
                // ensure file exists already (write_rows_start creates/truncates only once)
                // then append rows for subsequent teams
                // (no-op here; just fall through)
            }
            append_rows(&resolved, &bundle.rows)?;

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
