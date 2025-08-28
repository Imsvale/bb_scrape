// src/file.rs

use std::{
    error::Error,
    fs,
    io::{self, Write},
    mem::take,
    path::{Path, PathBuf},
    collections::HashMap,
};

use crate::config::options::{ AppOptions, PageKind::Players };
use crate::core::sanitize;

/* ---------- parsing (for .store) ---------- */

/// Minimal CSV/TSV parser (quotes + CRLF tolerant). std-only.
pub fn parse_rows(text: &str, sep: char) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut field = s!();
    let mut row = Vec::new();
    let mut in_quotes = false;
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quotes {
                    if matches!(chars.peek(), Some('"')) {
                        chars.next(); // double-quote escape
                        field.push('"');
                    } else {
                        in_quotes = false;
                    }
                } else {
                    in_quotes = true;
                }
            }
            c if c == sep && !in_quotes => {
                // move the field without cloning
                row.push(take(&mut field));
            }
            '\n' | '\r' if !in_quotes => {
                if ch == '\r' && matches!(chars.peek(), Some('\n')) { chars.next(); }
                row.push(take(&mut field));
                if !row.is_empty() && !(row.len() == 1 && row[0].is_empty()) {
                    rows.push(take(&mut row));
                } else {
                    row.clear();
                }
            }
            _ => field.push(ch),
        }
    }

    // Flush the trailing field/row — but ignore a trailing blank line.
    if in_quotes {
        // Unterminated quotes: behave like “end of line” to avoid data loss.
        // (Same behavior as before; we still don’t create a spurious empty row.)
    }
    row.push(field);
    if !row.is_empty() && !(row.len() == 1 && row[0].is_empty()) {
        rows.push(row);
    }

    rows
}

/* ---------- low-level delimited writers ---------- */

fn needs_quotes(field: &str, sep: char) -> bool {
    field.contains(sep) || field.contains('"') || field.contains('\n') || field.contains('\r')
}

/// Write a single CSV/TSV row to any writer.
pub fn write_row<W: Write>(mut w: W, row: &[String], sep: char) -> io::Result<()> {
    let mut first = true;
    for cell in row {
        if !first { write!(w, "{}", sep)?; } else { first = false; }
        if needs_quotes(cell, sep) {
            let escaped = cell.replace('"', "\"\"");
            write!(w, "\"{}\"", escaped)?;
        } else {
            write!(w, "{}", cell)?;
        }
    }
    writeln!(w)
}

/// Minimal writer for borrowed cells. Mirrors `write_row` quoting rules.
fn write_row_strs<W: Write>(mut w: W, row: &[&str], sep: char) -> io::Result<()> {
    let mut first = true;
    for cell in row {
        if !first { write!(w, "{}", sep)?; } else { first = false; }
        // If `needs_quotes` isn't public, inline the same logic here:
        let needs_q = cell.contains(sep) || cell.contains('"') || cell.contains('\n') || cell.contains('\r');
        if needs_q {
            let escaped = cell.replace('"', "\"\"");
            write!(w, "\"{}\"", escaped)?;
        } else {
            write!(w, "{}", cell)?;
        }
    }
    writeln!(w)
}

/* ---------- export gate (CSV/TSV today) ---------- */

/// One gate for all export callers:
/// - Players: optionally strip hash in col #1, then encode raw
/// - Others: pass-through (no hash logic)
pub fn to_export_string(
    o: &AppOptions,
    headers: &Option<Vec<String>>,
    rows: &[Vec<String>],
) -> String {

    let e = &o.export;
    let page = &o.scrape.page;

    let include_headers = e.include_headers;
    let sep = e.delimiter().unwrap();
    let mut buf: Vec<u8> = Vec::new();

    if include_headers {
        if let Some(h) = headers {
            // If you prefer, you can also use write_row_strs with borrowed cells:
            let _ = write_row_strs(&mut buf, &h.iter().map(|s| s.as_str()).collect::<Vec<_>>(), sep);
        }
    }

    let strip_players_hash = matches!(page, Players) && !e.keep_hash;

    // Reuse a tiny scratch buffer per row to avoid allocations in the hot path
    let mut scratch: Vec<&str> = Vec::new();

    for r in rows {
        scratch.clear();
        scratch.reserve_exact(r.len());

        if strip_players_hash && r.len() > 1 {
            for (i, cell) in r.iter().enumerate() {
                // zero-copy: borrow a subslice for col 1
                let s = if i == 1 { cell.strip_prefix('#').unwrap_or(cell) } else { cell.as_str() };
                scratch.push(s);
            }
            let _ = write_row_strs(&mut buf, &scratch, sep);
        } else {
            for cell in r { scratch.push(cell.as_str()); }
            let _ = write_row_strs(&mut buf, &scratch, sep);
        }
    }

    String::from_utf8(buf).unwrap_or_default()
}

/* ---------- high-level writers ---------- */

/// Write a single export file based on ExportOptions (path, headers policy, delimiter, etc.).
/// Returns the final path written to.
pub fn write_export_single(
    options: &AppOptions,
    headers: &Option<Vec<String>>,
    rows: &[Vec<String>],
) -> Result<PathBuf, Box<dyn Error>> {
    let export = &options.export;
    let path = export.out_path();

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            ensure_directory(parent)?;
        }
    }

    let contents = to_export_string(
        options,
        headers,
        rows,
    );

    fs::write(&path, contents)?;
    Ok(path)
}

/// Write multiple team files into the directory implied by `export.out_path()`
/// (which must be a directory when `export.export_type == PerTeam`).
/// `team_col` is the column index of the "Team" field in `rows` (Players = 3).
pub fn write_export_per_team(
    options: &AppOptions,
    headers: &Option<Vec<String>>,
    rows: &[Vec<String>],
    team_col: usize,
) -> Result<Vec<PathBuf>, Box<dyn Error>> {

    let export = &options.export;

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
        let base_stem = sanitize::sanitize_team_filename(&team_name, 0);
        let path = resolve_team_filename(&outdir, &base_stem, &mut seen, ext);

        let contents = to_export_string(
            options,
            headers,
            &team_rows,
        );

        fs::write(&path, contents)?;
        written.push(path);
    }

    Ok(written)
}

/* ---------- path utils ---------- */

pub fn ensure_directory(dir: &Path) -> Result<(), Box<dyn Error>> {
    if dir.exists() && !dir.is_dir() {
        return Err(format!("Path exists but is not a directory: {}", dir.display()).into());
    }
    if !dir.exists() { fs::create_dir_all(dir)?; }
    Ok(())
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
