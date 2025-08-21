// src/store.rs

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::csv::{parse_rows, detect_headers, Delim};
use crate::file::sanitize_team_filename; // you already have this
use crate::params::{DEFAULT_OUT_DIR, PLAYERS_SUBDIR, DEFAULT_MERGED_FILENAME};

pub struct Dataset {
    pub headers: Option<Vec<String>>,
    pub rows: Vec<Vec<String>>,
}

/// Load Players dataset from local store:
/// 1) load merged out/players/players.csv if present
/// 2) for each per-team CSV newer than merged, override that team’s rows
pub fn load_players_local() -> Result<Dataset, Box<dyn std::error::Error>> {
    let dir = PathBuf::from(DEFAULT_OUT_DIR).join(PLAYERS_SUBDIR);
    let merged_path = dir.join(DEFAULT_MERGED_FILENAME);

    let mut headers: Option<Vec<String>> = None;
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut merged_mtime: Option<SystemTime> = None;

    // 1) Load merged if exists
    if merged_path.exists() {
        let text = fs::read_to_string(&merged_path)?;
        let parsed = parse_rows(&text, &Delim::Csv);
        let (h, r) = detect_headers(parsed);
        headers = h;
        rows = r;
        merged_mtime = fs::metadata(&merged_path).ok().and_then(|m| m.modified().ok());
    }

    // Build index of existing rows by team name (col 3)
    let mut by_team: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    for row in rows.drain(..) {
        if row.len() > 3 {
            by_team.entry(row[3].clone()).or_default().push(row);
        }
    }

    // 2) Scan per-team files and overlay newer
    if dir.exists() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() { continue; }
            if path.file_name().and_then(|s| s.to_str()) == Some(DEFAULT_MERGED_FILENAME) {
                continue;
            }
            if path.extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase() != "csv" {
                continue;
            }

            let team_file_mtime = fs::metadata(&path).ok().and_then(|m| m.modified().ok());
            let newer_than_merged = match (team_file_mtime, merged_mtime) {
                (Some(tf), Some(mg)) => tf > mg,
                (Some(_), None)      => true,
                _                    => false,
            };

            if !newer_than_merged { continue; }

            // Guess team name from file stem (reverse-sanitize heuristic)
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            // We’ll trust the CSV instead of the filename (more robust)
            let text = fs::read_to_string(&path)?;
            let parsed = parse_rows(&text, &Delim::Csv);
            let (h2, rows2) = detect_headers(parsed);
            if headers.is_none() { headers = h2; } // adopt a header if merged had none

            // derive team name from first row’s col 3 if present
            if let Some(first) = rows2.get(0) {
                if first.len() > 3 {
                    let t = first[3].clone();
                    by_team.insert(t, rows2);
                    continue;
                }
            }
            // fallback: use filename stem, but we don’t know exact original spacing/case
            // Put rows under the stem as-is
            by_team.insert(stem.to_string(), rows2);
        }
    }

    // Reassemble final rows
    let mut final_rows: Vec<Vec<String>> = Vec::new();
    for (_team, mut rlist) in by_team {
        final_rows.append(&mut rlist);
    }

    Ok(Dataset { headers, rows: final_rows })
}
