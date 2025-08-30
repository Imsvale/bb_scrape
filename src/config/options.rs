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
                // Extension precedence: user-chosen > format default
                let ext_str = self.out_path.file_ext
                    .as_ref()
                    .and_then(|e| e.to_str())
                    .unwrap_or(self.format.ext());
                if !ext_str.is_empty() {
                    file_name.push(".");
                    file_name.push(ext_str);
                }
                path.push(PathBuf::from(&file_name))
            }
            PerTeam => { /* directory only */ }
        }
        path
    }

    /// Parse a user-provided path string into {dir, stem, ext?}.
    /// In SingleFile, now **respects** a pasted extension (stores it).
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
                // Respect user-provided extension if present; otherwise leave as-is
                self.out_path.file_ext = p.extension().map(|e| e.to_os_string());
            }
            PerTeam => {
                if !s.is_empty() {
                    self.out_path.dir = normalize_dir_like(Path::new(s));
                }
            }
        }
    }

    pub fn delimiter(&self) -> Option<char> { self.format.delimiter() }

    /// Default DIR for a page (public, so UI can reason about defaults).
    pub fn default_dir_for(kind: PageKind) -> PathBuf {
        let sub = match kind {
            PageKind::Players     => DEFAULT_PLAYERS_SUBDIR,
            PageKind::GameResults => DEFAULT_RESULTS_SUBDIR,
            _ => DEFAULT_PLAYERS_SUBDIR, // extend as needed
        };
        PathBuf::from(DEFAULT_OUT_DIR).join(sub)
    }

    /// Set only the DIR to the page-default. Keeps filename/ext as-is.
    pub fn set_default_dir_for_page(&mut self, kind: PageKind) {
        self.out_path.dir = Self::default_dir_for(kind);
    }

    /// Helper: compute a path string using a provided DIR and an arbitrary filename.
    pub fn join_dir_and_filename<D: AsRef<Path>, F: AsRef<Path>>(dir: D, filename: F) -> PathBuf {
        let mut p = dir.as_ref().to_path_buf();
        p.push(filename.as_ref());
        p
    }

    pub fn current_dir(&self) -> &Path {
        &self.out_path.dir
    }

    pub fn is_current_dir_default_for(&self, kind: PageKind) -> bool {
        fn norm(p: &Path) -> PathBuf { p.components().collect() }
        norm(self.current_dir()) == norm(&Self::default_dir_for(kind))
    }

    pub fn is_fully_default_for(&self, kind: PageKind) -> bool {
        if self.export_type != ExportType::SingleFile {
            return false;
        }
        let dir_is_default = {
            // Normalize both sides to components for platform separators
            let def = Self::default_dir_for(kind);
            def.components().eq(self.out_path.dir.components())
        };
        let stem_is_default = self.out_path.file_stem == OsString::from(DEFAULT_FILE);
        let uses_default_ext = self.out_path.file_ext.is_none();

        dir_is_default && stem_is_default && uses_default_ext
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputPath {
    dir: PathBuf,
    file_stem: OsString,            // without extension
    file_ext: Option<OsString>,     // user-chosen extension (e.g., "txt"); if None, use format.ext()
}

impl Default for OutputPath {
    fn default() -> Self {
        Self {
            dir: PathBuf::from(DEFAULT_OUT_DIR).join(DEFAULT_PLAYERS_SUBDIR),
            file_stem: OsString::from(DEFAULT_FILE),
            file_ext: None, // no extension chosen yet → format decides
        }
    }
}