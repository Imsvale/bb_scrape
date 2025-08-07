// /src/main.rs
// Minimal, std-only scraper for Brutalball team rosters.
// Usage:
//   cargo run --release -- -t 20
//   cargo run --release -- --all -o players_all.csv
//
// Output: NO HEADERS. Each row: Name, #Number, Race, Team, <attributes…>
// Assumptions (by design):
// - table has class="teamroster"
// - player rows have class="playerrow" or "playerrow1"
// - first cell is "Name #Number Race"

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
    let cli = parse_cli()?;

    let team_ids: Vec<u32> = if cli.all {
        (0..32).collect()
    } else {
        vec![cli.one_team.unwrap()]
    };

    let mut out = BufWriter::new(File::create(&cli.out)?);

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

fn parse_cli() -> Result<Cli, Box<dyn std::error::Error>> {
    let mut all = false;
    let mut one_team = None;
    let mut out = String::new();

    let mut args = env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--all" => all = true,
            "-t" | "--team" => {
                let v: u32 = args.next().ok_or("Missing team id")?.parse()?;
                if v >= 32 { return Err("Team id out of range (0..31)".into()); }
                one_team = Some(v);
            }
            "-o" | "--out" => out = args.next().ok_or("Missing output file")?,
            "-h" | "--help" => {
                eprintln!("Usage: --all | -t <id> [-o <output.csv>]");
                std::process::exit(0);
            }
            _ => return Err(format!("Unknown arg: {}", a).into()),
        }
    }
    if !all && one_team.is_none() {
        return Err("Specify --all or -t <id>".into());
    }
    if out.is_empty() {
        out = if all { "players_all.csv".into() } else { "players.csv".into() };
    }
    Ok(Cli { all, one_team, out })
}