// /src/main.rs
// Orchestrates the CLI parsing, per-team scraping, and CSV output.

use std::{env, fs::File, io::{BufWriter, Write}};
mod net;
mod html;
mod roster;
mod csv;

struct Cli {
    all: bool,
    one_team: Option<u32>,
    out: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI args
    let cli = parse_cli()?;

    // Decide which teams to scrape
    let team_ids: Vec<u32> = if cli.all {
        (0..32).collect()
    } else {
        vec![cli.one_team.unwrap()]
    };

    // Prepare output file
    let mut out = BufWriter::new(File::create(&cli.out)?);

    // For each team: fetch HTML, extract player rows, write CSV
    for tid in team_ids {
        let path = format!("/brutalball/team.php?i={}", tid);
        let html = net::http_get("dozerverse.com", 80, &path)?;
        let rows = roster::extract_player_rows(&html, tid)?;

        for row in rows {
            csv::write_csv_row(&mut out, &row)?;
        }
    }

    out.flush()?;
    println!("Wrote {}", cli.out);
    Ok(())
}

/// Minimal CLI parser. Supports:
/// -t / --team <id>      Scrape a single team by ID (0..31)
/// -o / --out <file>     Output CSV path
/// -h / --help           Show help text
fn parse_cli() -> Result<Cli, Box<dyn std::error::Error>> {
    let mut all = true;
    let mut one_team = None;
    let mut out = String::new();

    let mut args = env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "-t" | "--team" => {
                all = false;
                let v: u32 = args.next().ok_or("Missing team id")?.parse()?;
                if v >= 32 { return Err("Team id out of range (0..31)".into()); }
                one_team = Some(v);
            }
            "-o" | "--out" => out = args.next().ok_or("Missing output file")?,
            "-h" | "--help" => {
                eprintln!("Usage: [ -t <id> ] [-o <output.csv>]");
                std::process::exit(0);
            }
            _ => return Err(format!("Unknown argument: {}", a).into()),
        }
    }
    if out.is_empty() {
        out = "players.csv".into();
    }
    Ok(Cli { all, one_team, out })
}
