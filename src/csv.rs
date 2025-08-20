// src/csv.rs

use std::io::{Result, Write};

/// Write a single CSV row to any writer.
/// Quotes fields that contain comma, quote, or newline; doubles inner quotes.
pub fn write_row<W: Write>(out: &mut W, fields: &[String]) -> Result<()> {
    let mut first = true;
    for f in fields {
        if !first {
            write!(out, ",")?;
        }
        if f.contains(',') || f.contains('"') || f.contains('\n') {
            let esc = f.replace('"', "\"\"");
            write!(out, "\"{}\"", esc)?;
        } else {
            write!(out, "{}", f)?;
        }
        first = false;
    }
    writeln!(out)?;
    Ok(())
}
