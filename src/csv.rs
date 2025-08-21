// src/csv.rs
use std::io::{self, Write};

/// Output delimiter
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Delim {
    Csv, // comma
    Tsv, // tab
}

/* ---------------- Parsing ---------------- */

/// Minimal CSV/TSV parser (quotes + CRLF tolerant). std-only.
pub fn parse_rows(text: &str, delim: Delim) -> Vec<Vec<String>> {
    let sep = match delim { Delim::Csv => ',', Delim::Tsv => '\t' };
    let mut rows = Vec::new();
    let mut field = String::new();
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
                row.push(field.clone());
                field.clear();
            }
            '\n' | '\r' if !in_quotes => {
                if ch == '\r' && matches!(chars.peek(), Some('\n')) { chars.next(); }
                row.push(field.clone());
                field.clear();
                if !row.is_empty() && !(row.len() == 1 && row[0].is_empty()) {
                    rows.push(std::mem::take(&mut row));
                } else {
                    row.clear();
                }
            }
            _ => field.push(ch),
        }
    }
    // tolerate unterminated quote by flushing the field/row
    if !in_quotes {
        row.push(field);
        if !row.is_empty() { rows.push(row); }
    }
    rows
}

/// Heuristic: if the first cell is "Name" (Players page), treat first row as header.
/// TODO: Questionable purpose
pub fn detect_headers(mut rows: Vec<Vec<String>>) -> (Option<Vec<String>>, Vec<Vec<String>>) {
    if rows.is_empty() { return (None, rows); }
    let first = &rows[0];
    if !first.is_empty() && first[0].eq_ignore_ascii_case("name") {
        let header = rows.remove(0);
        return (Some(header), rows);
    }
    (None, rows)
}

/* ---------------- Writing ---------------- */

fn needs_quotes(field: &str, delim: Delim) -> bool {
    match delim {
        Delim::Csv => field.contains(',') || field.contains('"') || field.contains('\n') || field.contains('\r'),
        Delim::Tsv => field.contains('\t') || field.contains('"') || field.contains('\n') || field.contains('\r'),
    }
}

/// Write a single CSV/TSV row to any writer.
pub fn write_row<W: Write>(mut w: W, row: &[String], delim: Delim) -> io::Result<()> {
    let sep = match delim { Delim::Csv => ',', Delim::Tsv => '\t' };
    let mut first = true;
    for cell in row {
        if !first { write!(w, "{}", sep)?; } else { first = false; }
        if needs_quotes(cell, delim) {
            let escaped = cell.replace('"', "\"\"");
            write!(w, "\"{}\"", escaped)?;
        } else {
            write!(w, "{}", cell)?;
        }
    }
    writeln!(w)
}

/* ---------------- Export-time transforms (no mutation of base) ---------------- */

/// Transform the Number column (index 1) according to `keep_hash` for *export only*.
fn map_keep_hash(cell: &str, keep_hash: bool) -> String {
    if keep_hash {
        if cell.starts_with('#') { cell.to_string() }
        else if cell.is_empty()  { String::new() }
        else { format!("#{}", cell) }
    } else {
        cell.trim_start_matches('#').to_string()
    }
}

/// Build one output row from a base row, applying export-time toggles.
pub fn build_export_row(base_row: &[String], keep_hash: bool) -> Vec<String> {
    if base_row.len() <= 1 {
        return base_row.to_owned();
    }
    let mut out = base_row.to_owned();
    out[1] = map_keep_hash(&out[1], keep_hash);
    out
}

/// Create a full export string (Copy/Export) from base data and toggles.
/// - `headers`: base headers (if any)
/// - `rows`: base rows (assumed to have '#' in Number column)
/// - `include_headers`: whether to emit a header line
/// - `keep_hash`: whether to keep '#' in Number column for export
/// - `delim`: CSV or TSV
pub fn to_export_string(
    headers: &Option<Vec<String>>,
    rows: &[Vec<String>], // <-- Fix'd
    include_headers: bool,
    keep_hash: bool,
    delim: Delim,
) -> String {
    let mut buf: Vec<u8> = Vec::new();

    if include_headers {
        if let Some(h) = headers {
            let _ = write_row(&mut buf, h, delim);
        }
    }
    for r in rows {
        let mapped = build_export_row(r, keep_hash);
        let _ = write_row(&mut buf, &mapped, delim);
    }

    match String::from_utf8(buf) {
        Ok(s) => s,
        Err(e) => String::from_utf8_lossy(&e.into_bytes()).into_owned(),
    }
}

/* ---------------- Convenience: stringify rows as-is (no transforms) ---------------- */

/// If you still need to stringify rows as-is for preview/debug.
pub fn rows_to_string(rows: &[Vec<String>], headers: &Option<Vec<String>>, delim: Delim) -> String {
    let mut buf: Vec<u8> = Vec::new();

    if let Some(h) = headers {
        let _ = write_row(&mut buf, h, delim);
    }
    for r in rows {
        let _ = write_row(&mut buf, r, delim);
    }

    match String::from_utf8(buf) {
        Ok(s) => s,
        Err(e) => String::from_utf8_lossy(&e.into_bytes()).into_owned(),
    }
}
