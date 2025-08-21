// src/csv.rs
use std::io::{self, Write};

#[derive(Clone)]
pub enum Delim {
    Csv, // comma
    Tsv, // tab
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
