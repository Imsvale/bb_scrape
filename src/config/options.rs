// src/config/options.rs
use std::{
    path::{ Path, PathBuf },
    ffi::OsString,
    str,
    fmt,
};

use super::consts::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppOptions {
    pub scrape: ScrapeOptions,
    pub export: ExportOptions,
}

impl Default for AppOptions {
    fn default() -> Self {
        Self {
            scrape: ScrapeOptions::default(),
            export: ExportOptions::default(),      
        }
    }
}

/// Something about PageKind representing the specific page on the website
/// Each page has its own scrape Spec with details on how to extract the desired information
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum PageKind {
    Teams,
    Players,
    SeasonStats, 
    CareerStats, 
    GameResults,
    Injuries,
}

use PageKind::*;

impl str::FromStr for PageKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "teams"         => Ok(Teams),
            "players"       => Ok(Players),
            "seasonstats"   | "season_stats"   | "season-stats"   => Ok(SeasonStats),
            "careerstats"   | "career_stats"   | "career-stats"   => Ok(CareerStats),
            "gameresults"   | "game_results"   | "game-results"   => Ok(GameResults),
            "injuries"      => Ok(Injuries),
            other => Err(format!("Unknown page: {}", other)),
        }
    }
}

impl fmt::Display for PageKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Teams        => "teams",
            Players      => "players",
            SeasonStats  => "season-stats",
            CareerStats  => "career-stats",
            GameResults  => "game-results",
            Injuries     => "injuries",
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TeamSelector {
    All,
    One(u32),
    Ids(Vec<u32>),
}

use TeamSelector::*;

impl TeamSelector {
    pub fn add(&mut self, v: u32) {
        match self {
            All => *self = One(v),
            One(prev) => {
                let p = *prev;
                *self = Ids(vec![p, v]);
            }
            Ids(list) => list.push(v),
        }
    }

    pub fn extend<I: IntoIterator<Item = u32>>(&mut self, iter: I) {
        for v in iter {
            self.add(v);
        }
    }

    pub fn normalize(&mut self) {
        if let Ids(list) = self {
            list.sort_unstable();
            list.dedup();
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScrapeOptions {
    pub page: PageKind,
    pub teams: TeamSelector,
}

impl Default for ScrapeOptions {
    fn default() -> Self {
        Self {
            page: Players,
            teams: All,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportType {
    SingleFile, 
    PerTeam,
}

use ExportType::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Tsv,
    // Json,
    // Toml,
}

use ExportFormat::*;

impl ExportFormat {

    pub fn ext(&self) -> &'static str {
        match self { 
            Csv => "csv", 
            Tsv => "tsv",
            // Json => "json",
            // Toml => "toml",
         }
    }
    pub fn delimiter(&self) -> Option<char> {
        match self { 
            Csv => Some(','),
            Tsv => Some('\t'),
            // Json | Toml => None,
         }
    }
}

impl str::FromStr for ExportFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "csv" => Ok(Csv),
            "tsv" => Ok(Tsv),
            other => Err(format!("Unknown format: {}", other)),
        }
    }
}

impl fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(
            match self {
                Csv => "csv",
                Tsv => "tsv",
            }
        )
    }
}


#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportOptions {
    pub format: ExportFormat,
    pub export_type: ExportType,
    out_path: OutputPath,
    pub include_headers: bool, 
    pub keep_hash: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            format: Csv,
            export_type: SingleFile,
            out_path: OutputPath::default(),
            include_headers: true,
            keep_hash: true,
        }
    }
}

impl ExportOptions {
    pub fn out_path(&self) -> PathBuf {
        let mut path = self.out_path.dir.clone();

        match self.export_type {
            SingleFile => {
                // Build "<stem>.<ext>" in OsString to avoid UTF-8 loss
                let mut file_name: OsString = self.out_path.file_stem.clone();
                file_name.push(".");
                file_name.push(self.format.ext()); // ext is &str (ASCII), fine to push
                path.push(PathBuf::from(&file_name))
            }
            PerTeam => { /* directory only */},
        }
        path
    }

    /// Parse a user-provided path string into {dir, stem}.
    /// In SingleFile, ignores any pasted extension (format controls it).
    pub fn set_path(&mut self, text: &str) {

        fn normalize_dir_like(p: &Path) -> PathBuf {
            // Rebuild the path from components → uses platform separator
            p.components().collect()
        }

        let s = text.trim();

        match self.export_type {
            SingleFile => {
                let p = Path::new(s);
                if let Some(parent) = p.parent() {
                    // If there's no parent (e.g. "all"), leave dir as-is
                    if !parent.as_os_str().is_empty() {
                        self.out_path.dir = normalize_dir_like(parent);
                    }
                }
                if let Some(stem) = p.file_stem() {
                    self.out_path.file_stem = stem.to_os_string();
                }
            }
            PerTeam => {
                if !s.is_empty() {
                    self.out_path.dir = normalize_dir_like(Path::new(s));
                }
            }
        }
    }

    pub fn delimiter(&self) -> Option<char> { self.format.delimiter() }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputPath {
    dir: PathBuf,
    file_stem: OsString, // without extension
}

impl Default for OutputPath {
    fn default() -> Self {
        Self {
            dir: PathBuf::from(DEFAULT_OUT_DIR).join(DEFAULT_PLAYERS_SUBDIR),
            file_stem: OsString::from(DEFAULT_FILE),
        }
    }
}