// src/csv.rs
use std::io::{self, Write};

#[derive(Clone)]
pub enum Delim {
    Csv, // comma
    Tsv, // tab
}

pub fn parse_rows(text: &str, delim: &Delim) -> Vec<Vec<String>> {
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
                row.push(field.clone()); field.clear();
            }
            '\n' | '\r' if !in_quotes => {
                if ch == '\r' && matches!(chars.peek(), Some('\n')) { chars.next(); }
                row.push(field.clone()); field.clear();
                if !row.is_empty() && !(row.len() == 1 && row[0].is_empty()) {
                    rows.push(std::mem::take(&mut row));
                } else {
                    row.clear();
                }
            }
            _ => field.push(ch),
        }
    }
    if in_quotes { /* tolerate trailing quote mismatch by finishing the field */ }
    if !in_quotes {
        row.push(field);
        if !row.is_empty() { rows.push(row); }
    }
    rows
}

pub fn detect_headers(mut rows: Vec<Vec<String>>) -> (Option<Vec<String>>, Vec<Vec<String>>) {
    if rows.is_empty() { return (None, rows); }
    let first = &rows[0];
    // Simple heuristic for Players page
    if !first.is_empty() && first[0].eq_ignore_ascii_case("name") {
        let header = rows.remove(0);
        return (Some(header), rows);
    }
    (None, rows)
}

fn needs_quotes(field: &str, delim: &Delim) -> bool {
    match delim {
        Delim::Csv => field.contains(',') || field.contains('"') || field.contains('\n') || field.contains('\r'),
        Delim::Tsv => field.contains('\t') || field.contains('"') || field.contains('\n') || field.contains('\r'),
    }
}

pub fn write_row<W: Write>(mut w: W, row: &[String], delim: &Delim) -> io::Result<()> {
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

pub fn rows_to_string(rows: &[Vec<String>], headers: &Option<Vec<String>>, delim: &Delim) -> String {
    // Build into a Vec<u8> (implements io::Write), then convert to String.
    let mut buf: Vec<u8> = Vec::new();

    if let Some(h) = headers {
        // Writing to Vec<u8> is infallible; unwrap() is fine.
        write_row(&mut buf, h, delim).unwrap();
    }
    for r in rows {
        write_row(&mut buf, r, delim).unwrap();
    }

    match String::from_utf8(buf) {
        Ok(s) => s,
        Err(e) => String::from_utf8_lossy(&e.into_bytes()).into_owned(),
    }
}
