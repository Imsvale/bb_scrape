// src/cli.rs
use std::{env, path::PathBuf};
use crate::config::state::AppState;
use crate::config::options::PageKind;

pub enum Mode {
    Cli(AppState),
    Gui(AppState),
}

// Decide CLI vs GUI
pub fn detect_mode() -> Result<Mode, Box<dyn std::error::Error>> {

    let mut app_state = AppState::default();

    if std::env::args().len() == 1 {
        // only program name
        return Ok(Mode::Gui(app_state));
    }
    parse_cli(&mut app_state)?;
    Ok(Mode::Cli(app_state))
}

pub fn run(app_state: AppState) -> Result<(), Box<dyn std::error::Error>> {
    crate::runner::run(&app_state.options, None).map(|_| ())
}

fn parse_cli(app_state: &mut AppState) -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let mut export = app_state.options.export;
    let mut scrape = app_state.options.scrape;
    while let Some(a) = args.next() {
        match a.as_str() 
        {
            "--page" => {
                let v = args.next().ok_or("Missing value for --page")?;
                scrape.page = match v.to_ascii_lowercase().as_str() {
                    "players" => PageKind::Players,
                    other => return Err(format!("Unknown page: {}", other).into()),
                };}
            "--list-teams" => app_state.options.list_teams = true,
            "-t" | "--team" => {
                let v: u32 = args.next().ok_or("Missing team id")?.parse()?;
                if v >= 32 { return Err("Team id out of range (0..31)".into()); }
                scrape.one_team = Some(v);
                scrape.all = false; }            // override default
            "--ids" => {
                let v = args.next().ok_or("Missing value for --ids")?;
                scrape.ids_filter = Some(parse_ids_list(&v)?);}
            "-o" | "--out" => export.out_path = Some(PathBuf::from(args.next().ok_or("Missing output path")?)),
            "--format" => {
                let v = args.next().ok_or("Missing value for --format")?;
                app_state.format = match v.to_ascii_lowercase().as_str() {
                    "csv" => Delim::Csv,
                    "tsv" => Delim::Tsv,
                    other => return Err(format!("Unknown format: {}", other).into()),
                };}
            "--keephash" => app_state.keep_hash = true,
            "--include-headers" => app_state.include_headers = true,
            "--single" => app_state.single_file = true,
            "-h" | "--help" => {
                eprintln!(include_str!("cli_help.txt"));
                std::process::exit(0);
            }
            _ => return Err(format!("Unknown arg: {}", a).into()),
        }
    }

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
