// src/cli.rs
use std::{env, path::PathBuf};

use crate::params::{
    Params, 
    PageKind,
    DEFAULT_OUT_DIR,
    DEFAULT_MERGED_FILENAME,
};

pub enum Mode {
    Cli(Params),
    Gui,
}

// Decide CLI vs GUI
pub fn detect_mode() -> Result<Mode, Box<dyn std::error::Error>> {
    if std::env::args().len() == 1 {
        // only program name
        return Ok(Mode::Gui);
    }
    let params = parse_cli()?;
    Ok(Mode::Cli(params))
}

pub fn run(params: Params) -> Result<(), Box<dyn std::error::Error>> {
    if params.list_teams {
        for (id, name) in crate::runner::list_teams() {
            println!("{},{}", id, name);
        }
        return Ok(());
    }
    crate::runner::run(&params, None).map(|_| ())
}

fn parse_cli() -> Result<Params, Box<dyn std::error::Error>> {
    let mut page = PageKind::Players;
    let mut all = true;                   // default to all
    let mut one_team = None;
    let mut out: Option<PathBuf> = None;
    let mut keep_hash = false;
    let mut include_headers = false;
    let mut list_teams = false;
    let mut ids_filter: Option<Vec<u32>> = None;
    let mut per_team = false;             // default merged

    let mut args = env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--page" => {
                let v = args.next().ok_or("Missing value for --page")?;
                page = match v.to_ascii_lowercase().as_str() {
                    "players" => PageKind::Players,
                    other => return Err(format!("Unknown page: {}", other).into()),
                };
            }
            "--list-teams" => list_teams = true,
            "--all" | "-a" => all = true,
            "-t" | "--team" => {
                let v: u32 = args.next().ok_or("Missing team id")?.parse()?;
                if v >= 32 { return Err("Team id out of range (0..31)".into()); }
                one_team = Some(v);
                all = false;             // override default
            }
            "--ids" => {
                let v = args.next().ok_or("Missing value for --ids")?;
                ids_filter = Some(parse_ids_list(&v)?);
            }
            "-o" | "--out" => out = Some(PathBuf::from(args.next().ok_or("Missing output path")?)),
            "--keephash" => keep_hash = true,
            "--include-headers" => include_headers = true,
            "--per-team" => per_team = true,
            "-h" | "--help" => {
                eprintln!(include_str!("cli_help.txt"));
                std::process::exit(0);
            }
            _ => return Err(format!("Unknown arg: {}", a).into()),
        }
    }

    // Default output if not given
    if out.is_none() {
        if per_team {
            out = Some(PathBuf::from(DEFAULT_OUT_DIR)); // directory
        } else {
            out = Some(PathBuf::from(DEFAULT_OUT_DIR).join(DEFAULT_MERGED_FILENAME)); // single file
        }
    }

    Ok(Params {
        page,
        all,
        one_team,
        out,
        keep_hash,
        include_headers,
        list_teams,
        ids_filter,
        per_team,
    })
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
