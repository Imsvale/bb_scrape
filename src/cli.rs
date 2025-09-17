// src/cli.rs
use std::{env, path::PathBuf};
use std::str::FromStr;
use std::error::Error;

use crate::{ 
    file,
    scrape,
};
use crate::{
    store::{ self, DataSet },
    progress::Progress,
    config::{
        state::AppState, 
        options::{ 
            ExportType::*, 
            ExportFormat, 
            PageKind::{ self, * }
        },
    },
};

pub enum Mode {
    Cli(AppState),
    Gui(AppState),
}

pub fn run() -> Result<(), Box<dyn Error>> {

    let mut app_state = AppState::default();
    parse_cli(&mut app_state)?;
    
    let page = app_state.options.scrape.page;
    let options = &mut app_state.options;

    // Ensure default DIR mirrors page (preserve filename/ext if user didn't change DIR)
    // Only flip when current dir is one of the page defaults.
    if options.export.is_current_dir_default_for(PageKind::Players)
        || options.export.is_current_dir_default_for(PageKind::GameResults)
        || options.export.is_current_dir_default_for(PageKind::Teams)
    {
        options.export.set_default_dir_for_page(page);
    }
    // Special-case Teams: default filename should be "teams" instead of "all"
    if matches!(page, PageKind::Teams)
        && options.export.is_fully_default_for(PageKind::Teams)
    {
        // Only change the stem when still fully-default
        options.export.set_path(crate::config::consts::DEFAULT_TEAMS_FILE);
    }
    // Special-case Injuries: default filename should be "injuries"
    if matches!(page, PageKind::Injuries)
        && options.export.is_fully_default_for(PageKind::Injuries)
    {
        options.export.set_path("injuries");
    }

    // 1) SCRAPE
    let mut cp = CliProgress::default();

    let mut ds = match page {
        Players => scrape::collect_players(&options.scrape, Some(&mut cp))?,
        Teams => scrape::collect_teams(Some(&mut cp))?,
        GameResults => {
            let ds = scrape::collect_game_results(Some(&mut cp))?;
            if let Some(first) = ds.rows.get(0).and_then(|r| r.get(0)) {
                if let Ok(season) = first.trim().parse::<u32>() { let _ = store::save_season(season); }
            }
            ds
        },
        SeasonStats => todo!("CLI: SeasonStats scraper not implemented yet"),
        CareerStats => todo!("CLI: CareerStats scraper not implemented yet"),
        Injuries => scrape::collect_injuries(Some(&mut cp))?,
    };

    // Align with GUI: if headers are missing, inject page defaults so exports include headers.
    inject_headers_for_cli(page, &mut ds);

    // 2) Cache the dataset (best-effort)
    let _ = store::save_dataset(&page, &DataSet {
        headers: ds.headers.clone(),
        rows: ds.rows.clone(),
    });

    // 3) Export according to ExportOptions
    let export = &mut options.export;

    // Page-agnostic skip optional: players=#, results=match id
    if export.skip_optional {
        if matches!(page, PageKind::Players) {
            export.keep_hash = false;
        }
    }

    // Per-team only makes sense for pages that can be split into team-wise data
    // For now just Players
    // Later also Career and Season stats, maybe even Game Results and Injuries
    // So potentially all except the Teams list
    let (effective_export_type, team_col) = match page {
        Players => (export.export_type, Some(3usize)),
        GameResults => (export.export_type, None), // use two-column variant
        _ => (export.export_type, None),
    };

    // Adjust headers/rows for Game Results when skipping optional match id
    let (mut headers_to_write, mut rows_to_write) = (ds.headers.clone(), ds.rows.clone());
    if matches!(page, PageKind::GameResults) && export.skip_optional {
        if let Some(h) = &mut headers_to_write { if !h.is_empty() { h.pop(); } }
        for r in &mut rows_to_write { if !r.is_empty() { r.pop(); } }
    }

    let written: Vec<PathBuf> = match effective_export_type {
        SingleFile => {
            file::write_export_single(options, &headers_to_write, &rows_to_write)
                .map(|p| vec![p])?
        }
        PerTeam => {
            match page {
                PageKind::Players => file::write_export_per_team(options, &headers_to_write, &rows_to_write, team_col.unwrap())?,
                PageKind::GameResults => file::write_export_per_team_results(options, &headers_to_write, &rows_to_write, 2, 5)?,
                PageKind::Injuries => file::write_export_per_team_results(options, &headers_to_write, &rows_to_write, 2, 8)?,
                _ => file::write_export_per_team(options, &headers_to_write, &rows_to_write, team_col.unwrap_or(0))?,
            }
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


fn parse_cli(app_state: &mut AppState) -> Result<(), Box<dyn Error>> {
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
                for (id, name) in scrape::list_teams() {
                    println!("{:2}  {}", id, name);
                }
                std::process::exit(0);
            }

            "-p" | "--page" => {
                let v = args.next().ok_or("Missing value for --page")?;
                scrape.page = PageKind::from_str(&v)?;
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

            "-s" | "--skip-optional" => { export.skip_optional = true; }
            "-x" | "--drop-headers" => { export.include_headers = false; }
            "-m" | "--multi" | "--per-team" => { export.export_type = PerTeam; }

            _ => return Err(format!("Unknown arg: {}", a).into()),
        }
    }

    // Sort and dedup
    scrape.teams.normalize();

    Ok(())
}

fn parse_ids_list(s: &str) -> Result<Vec<u32>, Box<dyn Error>> {
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

/// Fill headers from page defaults when the scraper returns None, mirroring the GUI behavior.
fn inject_headers_for_cli(kind: PageKind, ds: &mut DataSet) {
    if ds.headers.is_some() { return; }
    let page = crate::gui::router::page_for(&kind);
    if let Some(hs) = page.default_headers() {
        ds.headers = Some(hs.iter().map(|s| s.to_string()).collect());
    }
}

/* ---------- CLI progress ---------- */

#[derive(Default)]
struct CliProgress {
    done: usize,
    total: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::options::ExportType;

    #[test]
    fn ids_parser_handles_ranges_and_values() {
        let v = parse_ids_list("1, 3-5, 7").unwrap();
        assert_eq!(v, vec![1,3,4,5,7]);

        // out-of-range values are ignored (>=32)
        let v2 = parse_ids_list("0, 31, 32, 40").unwrap();
        assert_eq!(v2, vec![0,31]);

        // duplicates are removed and sorted
        let v3 = parse_ids_list("5, 3-5, 4").unwrap();
        assert_eq!(v3, vec![3,4,5]);
    }

    // Keep the export gating logic equivalent to run() for testability.
    fn effective_export_for(page: PageKind, requested: ExportType) -> (ExportType, Option<usize>) {
        match page {
            Players => (requested, Some(3usize)),
            _ if matches!(requested, PerTeam) => (SingleFile, None),
            _ => (requested, None),
        }
    }

    #[test]
    fn per_team_only_for_players() {
        // Players keeps per-team and returns team_col=3
        let (ty, col) = effective_export_for(PageKind::Players, PerTeam);
        assert!(matches!(ty, PerTeam));
        assert_eq!(col, Some(3));

        // Non-Players downgrades to SingleFile
        let (ty2, col2) = effective_export_for(PageKind::GameResults, PerTeam);
        assert!(matches!(ty2, SingleFile));
        assert_eq!(col2, None);
    }

    #[test]
    fn inject_headers_uses_page_defaults() {
        let mut ds = DataSet { headers: None, rows: vec![vec!["x".into()]] };
        inject_headers_for_cli(PageKind::GameResults, &mut ds);
        assert!(ds.headers.is_some());
        assert_eq!(ds.headers.as_ref().unwrap().get(0).map(|s| s.as_str()), Some("S"));
    }
}

impl Progress for CliProgress {
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
