// /src/csv.rs
use std::fs::File;
use std::io::{BufWriter, Write};

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
