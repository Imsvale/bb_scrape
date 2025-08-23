// src/config/options.rs
use std::ffi::OsString;
use std::path::{ Path, PathBuf };
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageKind {
    Teams,
    Players,
    SeasonStats, 
    CareerStats, 
    GameResults,
    Injuries,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TeamSelector {
    All,
    One(u32),
    Ids(Vec<u32>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScrapeOptions {
    pub page: PageKind,
    pub teams: TeamSelector,
}

impl Default for ScrapeOptions {
    fn default() -> Self {
        Self {
            page: PageKind::Players,
            teams: TeamSelector::All,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportType {
    SingleFile, 
    PerTeam,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Tsv,
    // Json,
    // Toml,
}

impl ExportFormat {
    pub fn ext(&self) -> &'static str {
        match self { 
            ExportFormat::Csv => "csv", 
            ExportFormat::Tsv => "tsv",
            // ExportFormat::Json => "json",
            // ExportFormat::Toml => "toml",
         }
    }
    pub fn delimiter(&self) -> Option<char> {
        match self { 
            ExportFormat::Csv => Some(','),
            ExportFormat::Tsv => Some('\t'),
            // ExportFormat::Json | ExportFormat::Toml => None,
         }
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
            format: ExportFormat::Csv,
            export_type: ExportType::SingleFile,
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
            ExportType::SingleFile => {
                // Build "<stem>.<ext>" in OsString to avoid UTF-8 loss
                let mut file_name: OsString = self.out_path.file_stem.clone();
                file_name.push(".");
                file_name.push(self.format.ext()); // ext is &str (ASCII), fine to push
                path.push(PathBuf::from(&file_name))
            }
            ExportType::PerTeam => { /* directory only */},
        }
        path
    }

    /// Parse a user-provided path string into {dir, stem}.
    /// In SingleFile, ignores any pasted extension (format controls it).
    pub fn set_path(&mut self, text: &str) {
        let s = text.trim();

        match self.export_type {
            ExportType::SingleFile => {
                let p = Path::new(s);
                if let Some(parent) = p.parent() {
                    // If there's no parent (e.g. "all"), leave dir as-is
                    if !parent.as_os_str().is_empty() {
                        self.out_path.dir = parent.to_path_buf();
                    }
                }
                if let Some(stem) = p.file_stem() {
                    self.out_path.file_stem = stem.to_os_string();
                }
            }
            ExportType::PerTeam => {
                if !s.is_empty() {
                    self.out_path.dir = PathBuf::from(s);
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