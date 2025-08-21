// src/cli.rs
use std::{env, path::PathBuf};

use crate::csv::Delim;
use crate::params::{ PageKind, Params };

pub enum Mode {
    Cli(Params),
    Gui(Params),
}

// Decide CLI vs GUI
pub fn detect_mode() -> Result<Mode, Box<dyn std::error::Error>> {

    let mut params = Params::new();

    if std::env::args().len() == 1 {
        // only program name
        return Ok(Mode::Gui(params));
    }
    parse_cli(&mut params)?;
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

fn parse_cli(params: &mut Params) -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() 
        {
            "--page" => {
                let v = args.next().ok_or("Missing value for --page")?;
                params.page = match v.to_ascii_lowercase().as_str() {
                    "players" => PageKind::Players,
                    other => return Err(format!("Unknown page: {}", other).into()),
                };}
            "--list-teams" => params.list_teams = true,
            "--all" | "-a" => params.all = true,
            "-t" | "--team" => {
                let v: u32 = args.next().ok_or("Missing team id")?.parse()?;
                if v >= 32 { return Err("Team id out of range (0..31)".into()); }
                params.one_team = Some(v);
                params.all = false; }            // override default
            "--ids" => {
                let v = args.next().ok_or("Missing value for --ids")?;
                params.ids_filter = Some(parse_ids_list(&v)?);}
            "-o" | "--out" => params.out = Some(PathBuf::from(args.next().ok_or("Missing output path")?)),
            "--format" => {
                let v = args.next().ok_or("Missing value for --format")?;
                params.format = match v.to_ascii_lowercase().as_str() {
                    "csv" => Delim::Csv,
                    "tsv" => Delim::Tsv,
                    other => return Err(format!("Unknown format: {}", other).into()),
                };}
            "--keephash" => params.keep_hash = true,
            "--include-headers" => params.include_headers = true,
            "--single" => params.single_file = true,
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
