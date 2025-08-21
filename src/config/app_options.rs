// src/config/app_options.rs
use super::consts::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppOptions {
    pub scraper: ScraperOptions,
    pub export: ExportOptions,
}

impl Default for AppOptions {
    fn default() -> Self {
        Self {
            scraper: ScraperOptions::default(),
            export: ExportOptions::default(),      
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageKind {
    Players,
    // TODO: Other pages
    // SeasonStats,
    // CareerStats,
    // Injuries,
    // GameResults,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TeamSelector {
    All,
    Ids(Vec<u32>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScraperOptions {
    pub page: PageKind,
    pub teams: TeamSelector,
}

impl Default for ScraperOptions {
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
    MultiFile,
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
    pub fn ext(self) -> String {
        match self {
            ExportFormat::Csv => s!("csv"),
            ExportFormat::Tsv => s!("tsv"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputPath {
    dir: String,
    stem: String,
    ext: String,
}

impl Default for OutputPath {
    fn default() -> Self {
        Self {
            dir: join!(DEFAULT_OUT_DIR, "/", DEFAULT_PLAYERS_SUBDIR),
            stem: s!(DEFAULT_FILE),
            ext: s!("csv"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportOptions {
    pub format: ExportFormat,
    pub export_type: ExportType,
    pub out_path: OutputPath,
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
    pub fn set_format(&mut self, fmt: ExportFormat) {

        self.out_path.ext = match fmt {
            ExportFormat::Csv => s!("csv"),
            ExportFormat::Tsv => s!("tsv"),
            // ExportFormat::Json => "json",
            // ExportFormat::Toml => "toml",
        };

        self.format = fmt;
    }

    fn refresh_path(&mut self) {

    }
}
