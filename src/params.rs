// src/params.rs
use std::path::PathBuf;
use crate::csv::Delim;

pub const DEFAULT_OUT_DIR: &str = "out";
pub const PLAYERS_SUBDIR: &str = "players";
pub const DEFAULT_MERGED_FILENAME: &str = "players.csv";
pub const HOST: &str = "dozerverse.com";
pub const PREFIX: &str = "/brutalball/";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageKind {
    Players,
    // SeasonStats,
    // CareerStats,
    // Season,
    // Injuries,
}

#[derive(Clone)]
pub struct Params {
    pub page: PageKind,              // players, (later: season stats, injuries, etc.)
    pub all: bool,                   // scrape all teams
    pub one_team: Option<u32>,       // scrape one team
    pub out: Option<PathBuf>,        // output path (dir for per-team, file for merged)
    pub keep_hash: bool,             // keep hash in filenames (for diff/debug)
    pub include_headers: bool,       // include headers row in CSV
    pub list_teams: bool,            // list teams then exit
    pub ids_filter: Option<Vec<u32>>,// filter subset of team IDs
    pub per_team: bool,              // write one file per team vs merged single
    pub format: Delim,
}

impl Params {
    pub fn new() -> Self {
        Self {
            page: PageKind::Players,
            all: true,
            one_team: None,
            out: Some(PathBuf::from(DEFAULT_OUT_DIR).join(DEFAULT_MERGED_FILENAME)),
            keep_hash: false,
            include_headers: false,
            list_teams: false,
            ids_filter: None,
            per_team: false,
            format: Delim::Csv,
        }
    }
}

