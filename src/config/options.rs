// src/config/app_options.rs
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageKind {
    Players,
    // TODO: SeasonStats, CareerStats, Injuries, GameResults,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExportType {
    SingleFile, 
    PerTeam,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Tsv,
    // TODO: Other formats?
    // Json,
    // Toml,
}

impl ExportFormat {
    pub fn ext(&self) -> &'static str {
        match self { ExportFormat::Csv => "csv", ExportFormat::Tsv => "tsv" }
    }
    pub fn delim(&self) -> char {
        match self { ExportFormat::Csv => ',', ExportFormat::Tsv => '\t' }
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
            include_headers: false,
            keep_hash: false,
        }
    }
}

impl ExportOptions {
    pub fn out_path(&self) -> PathBuf {
        let mut path = self.out_path.dir.clone();

        match self.export_type {
            ExportType::SingleFile => {
                // join "<stem>.<ext>" – keep using your macro
                let stem = self
                    .out_path
                    .file_stem
                    .to_string_lossy();
                let ext = self.format.ext();
                path.push(join!(stem, ".", ext));
            },
            ExportType::PerTeam => { /* directory only */},
        }
        path
    }

    /// Parse GUI text into dir + stem. Ignores pasted extension; format controls it.
    pub fn set_path(&mut self, text: &str) {
        let s = text.trim();

        match self.export_type {
            ExportType::SingleFile => {
                let p = Path::new(s);
                if let Some(parent) = p.parent() {
                    self.out_path.dir = parent.to_path_buf();
                }
                if let Some(stem) = p.file_stem() {
                    self.out_path.file_stem = stem.to_os_string();
                }
                // Ignore pasted extension; format controls it.
            }
            ExportType::PerTeam => {
                self.out_path.dir = PathBuf::from(s);
            }
        }
    }

    pub fn delim(&self) -> char {
        match self.format {
            ExportFormat::Csv => ',',
            ExportFormat::Tsv => '\t',
        }
    }
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