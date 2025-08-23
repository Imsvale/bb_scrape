// src/file.rs

use std::{
    fs::{self, File, OpenOptions},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    collections::HashMap,
};

use crate::csv::{write_row, to_export_string};
use crate::config::options::ExportOptions;

/// Write a single export file based on ExportOptions (path, headers policy, delimiter, etc.).
/// Returns the final path written to.
pub fn write_export_single(
    export: &ExportOptions,
    headers: &Option<Vec<String>>,
    rows: &[Vec<String>],
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = export.out_path();

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            ensure_directory(parent)?;
        }
    }

    let contents = to_export_string(
        headers,
        rows,
        export.include_headers,
        export.keep_hash,
        export.delimiter().unwrap(),
    );

    std::fs::write(&path, contents)?;
    Ok(path)
}

/// Write multiple team files into the directory implied by `export.out_path()`
/// (which must be a directory when `export.export_type == PerTeam`).
/// `team_col` is the column index of the "Team" field in `rows` (Players = 3).
pub fn write_export_per_team(
    export: &ExportOptions,
    headers: &Option<Vec<String>>,
    rows: &[Vec<String>],
    team_col: usize,
) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    // Resolve target directory and ensure it exists
    let outdir = export.out_path();
    ensure_directory(&outdir)?;

    // Group rows by team name from the given column
    let mut by_team: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    for r in rows {
        if let Some(team) = r.get(team_col) {
            by_team.entry(team.clone()).or_default().push(r.clone());
        }
    }

    // Dedup stems and write each file
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut written = Vec::with_capacity(by_team.len());
    let ext = export.format.ext();

    for (team_name, team_rows) in by_team {
        let base_stem = sanitize_team_filename(&team_name, 0);
        let path = resolve_team_filename(&outdir, &base_stem, &mut seen, ext);

        let contents = to_export_string(
            headers,
            &team_rows,
            export.include_headers,
            export.keep_hash,
            export.delimiter().unwrap(),
        );

        std::fs::write(&path, contents)?;
        written.push(path);
    }

    Ok(written)
}

/// Ensure parent dir exists; create/truncate file; optionally write header.
pub fn write_rows_start(
    path: &Path,
    headers: Option<&[String]>,
    sep: char,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            ensure_directory(parent)?;
        }
    }
    let file = File::create(path)?; // truncate/overwrite
    let mut out = BufWriter::new(file);
    if let Some(h) = headers {
        write_row(&mut out, h, sep)?;
    }
    out.flush()?;
    Ok(())
}

/// Append multiple rows to an existing CSV/TSV file (must be created already).
pub fn append_rows(
    path: &Path, 
    rows: &[Vec<String>],
    sep: char,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = OpenOptions::new().append(true).open(path)?;
    let mut out = BufWriter::new(file);
    for row in rows {
        write_row(&mut out, row, sep)?;
    }
    out.flush()?;
    Ok(())
}

pub fn resolve_single_out_path(user_o: &str, default_filename: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if user_o.is_empty() { return Ok(PathBuf::from(default_filename)); }
    let p = PathBuf::from(normalize_separators(user_o));
    if looks_like_dir_hint(&p) || p.is_dir() {
        ensure_directory(&p)?; Ok(p.join(default_filename))
    } else {
        Ok(p)
    }
}

pub fn normalize_separators(p: &str) -> String {
    let sep = std::path::MAIN_SEPARATOR;
    p.chars().map(|c| if c=='/'||c=='\\' { sep } else { c }).collect()
}

pub fn normalize_dir_path(p: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let pb = PathBuf::from(normalize_separators(p));
    Ok(pb)
}

pub fn ensure_directory(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if dir.exists() && !dir.is_dir() {
        return Err(format!("Path exists but is not a directory: {}", dir.display()).into());
    }
    if !dir.exists() { fs::create_dir_all(dir)?; }
    Ok(())
}

pub fn looks_like_dir_hint(p: &Path) -> bool {
    let s = p.to_string_lossy();
    s.ends_with('/') || s.ends_with('\\')
}

pub fn sanitize_team_filename(name: &str, id: u32) -> String {
    crate::core::sanitize::sanitize_team_filename(name, id)
}

/// Duplicate handling **only within this run**
pub fn resolve_team_filename(
    dir: &Path,
    stem: &str,                        // already sanitized, no extension
    seen_names: &mut HashMap<String, usize>,
    ext: &str,                         // "csv" | "tsv" | ...
) -> PathBuf {
    // How many times have we seen this base?
    let count = seen_names.entry(stem.to_string()).or_insert(0);

    // First occurrence: "<stem>.ext"
    // Subsequent:       "<stem> (N).ext" with N starting at 2
    let filename = if *count == 0 {
        format!("{stem}.{ext}")
    } else {
        format!("{stem} ({}).{ext}", *count + 1)
    };

    *count += 1;
    dir.join(filename)
}


