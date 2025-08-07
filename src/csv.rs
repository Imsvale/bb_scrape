// /src/csv.rs
// Minimal CSV writer for rows of String fields.
// Only quotes fields when needed, and escapes quotes by doubling them.

use std::fs::File;
use std::io::{BufWriter, Write};

/// Write a single CSV row to the output.
///
/// Fields are joined by commas.
/// Quotes are added if a field contains a comma, double-quote, or newline.
/// Double-quotes inside a field are escaped by doubling them.
pub fn write_csv_row(out: &mut BufWriter<File>, fields: &[String]) -> std::io::Result<()> {
    let mut first = true;
    for field in fields {
        if !first {
            write!(out, ",")?;
        }
        let needs_quote = field.contains(',') || field.contains('"') || field.contains('\n');
        if needs_quote {
            let escaped = field.replace('"', "\"\"");
            write!(out, "\"{}\"", escaped)?;
        } else {
            write!(out, "{}", field)?;
        }
        first = false;
    }
    writeln!(out)?;
    Ok(())
}
