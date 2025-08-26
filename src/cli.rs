// src/cli.rs
use std::{env, path::PathBuf};
use std::str::FromStr;

use crate::{ 
    file,
    scrape,
};
use crate::config::{
    state::AppState,
    options::{ PageKind::*, TeamSelector, ExportType, ExportFormat },
};

pub enum Mode {
    Cli(AppState),
    Gui(AppState),
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {

    let mut app_state = AppState::default();
    parse_cli(&mut app_state)?;
    
    let scrape = &app_state.options.scrape;
    let options = &app_state.options;

    // 1) SCRAPE
    let mut progress = CliProgress::default();
    let ds = scrape::run(scrape, Some(&mut progress))?;

    // 2) Cache the dataset (best-effort)
    let _ = crate::store::save_dataset(&scrape.page, &crate::store::DataSet {
        headers: ds.headers.clone(),
        rows: ds.rows.clone(),
    });

    // 3) Export according to ExportOptions
    let export = &app_state.options.export;

    // Per-team only makes sense for pages that can be split into team-wise data
    // For now just Players
    // Later also Career and Season stats, maybe even Game Results and Injuries
    // So potentially all except the Teams list
    let (effective_export_type, team_col) = match scrape.page {
        Players => (export.export_type, Some(3usize)),
        _ if matches!(export.export_type, ExportType::PerTeam) => {
            eprintln!("Per-team export is only supported for the Players page; writing a single file instead.");
            (ExportType::SingleFile, None)
        }
        _ => (export.export_type, None),
    };

    let written: Vec<PathBuf> = match effective_export_type {
        ExportType::SingleFile => {
            file::write_export_single(options, &ds.headers, &ds.rows)
                .map(|p| vec![p])?
        }
        ExportType::PerTeam => {
            // safe: only reached for Players due to guard above
            file::write_export_per_team(options, &ds.headers, &ds.rows, team_col.unwrap())?
        }
    };

    if written.is_empty() {
        eprintln!("Nothing to export.");
    } else if let Some(last) = written.last() {
        eprintln!("Exported {} file(s). Last: {}", written.len(), last.display());
    } else {
        eprintln!("Export done.");
    }

    Ok(())
}


fn parse_cli(app_state: &mut AppState) -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);

    // IMPORTANT: mutate the real structs, not copies
    let export = &mut app_state.options.export;
    let scrape = &mut app_state.options.scrape;

    while let Some(a) = args.next() {
        match a.as_str() {
            "-h" | "--help" => {
                eprintln!(include_str!("cli_help.txt"));
                std::process::exit(0);
            }

            "-l" | "--list-teams" => {
                for (id, name) in crate::scrape::list_teams() {
                    println!("{:2}  {}", id, name);
                }
                std::process::exit(0);
            }

            "-p" | "--page" => {
                let v = args.next().ok_or("Missing value for --page")?;
                scrape.page = match v.to_ascii_lowercase().as_str() {
                    "players" => Players,
                    "teams"   => Teams,
                    other => return Err(format!("Unknown page: {}", other).into()),
                };
            }

            "-t" | "--team" => {
                let v: u32 = args.next().ok_or("Missing team id")?.parse()?;
                if v >= 32 { return Err("Team ID out of range (0-31)".into()); }
                scrape.teams.add(v);
            }

            "-i" | "--ids" => {
                let v = args.next().ok_or("Missing value for --ids")?;
                let list = parse_ids_list(&v)?;
                scrape.teams.extend(list);
            }

            "-o" | "--out" => {
                let path = args.next().ok_or("Missing output path")?;
                export.set_path(&path);
            }

            "-f" | "--format" => {
                let v = args.next().ok_or("Missing value for --format")?;
                export.format = ExportFormat::from_str(&v)?;
            }

            "-#" | "--nohash" => { export.keep_hash = false; }
            "-x" | "--drop-headers" => { export.include_headers = false; }
            "-m" | "--multi" | "--per-team" => { export.export_type = ExportType::PerTeam; }

            _ => return Err(format!("Unknown arg: {}", a).into()),
        }
    }

    // Sort and dedup
    scrape.teams.normalize();

    Ok(())
}

fn parse_ids_list(s: &str) -> Result<Vec<u32>, Box<dyn std::error::Error>> {
    let mut out = Vec::new();
    for part in s.split(',') {
        let part = part.trim();
        if part.is_empty() { continue; }
        if let Some(dash) = part.find('-') {
            let a: u32 = part[..dash].trim().parse()?;
            let b: u32 = part[dash + 1..].trim().parse()?;
            if a > b { return Err(format!("Invalid range: {}", part).into()); }
            for v in a..=b {
                if v < 32 { out.push(v); }
            }
        } else {
            let v: u32 = part.parse()?;
            if v < 32 { out.push(v); }
        }
    }
    out.sort_unstable();
    out.dedup();
    Ok(out)
}

/* ---------- CLI progress ---------- */

#[derive(Default)]
struct CliProgress {
    done: usize,
    total: usize,
}

impl crate::progress::Progress for CliProgress {
    fn begin(&mut self, total: usize) {
        self.total = total;
        eprintln!("Fetching… {} team(s)", total);
    }
    fn log(&mut self, msg: &str) {
        eprintln!("{}", msg);
    }
    fn item_done(&mut self, _team_id: u32) {
        self.done += 1;
        eprintln!("Fetched {}/{}", self.done, self.total);
    }
    fn finish(&mut self) {}
}
