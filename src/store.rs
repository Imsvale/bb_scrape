// src/store.rs
use std::{
    fs::{self, File},
    io::{self, BufWriter},
    path::PathBuf,
};

use crate::csv::{parse_rows, write_row};
use crate::config::consts::DEFAULT_OUT_DIR;
use crate::config::options::PageKind;

#[derive(Clone, Debug)]
pub struct Dataset {
    pub headers: Option<Vec<String>>,
    pub rows: Vec<Vec<String>>,
}

// We keep store files separate from export files:
//   <DEFAULT_OUT_DIR>/data/<page>.csv
const STORE_SUBDIR: &str = "data";
const STORE_SEP: char = ','; // raw cache always stored as CSV

fn store_dir() -> PathBuf {
    PathBuf::from(DEFAULT_OUT_DIR).join(STORE_SUBDIR)
}

fn page_filename(kind: &PageKind) -> &'static str {
    match kind {
        PageKind::Players => "players.csv",
        PageKind::SeasonStats => "season_stats.csv",
        PageKind::CareerStats => "career_stats.csv",
        PageKind::Injuries    => "injuries.csv",
        PageKind::GameResults => "game_results.csv",
    }
}

fn store_path(kind: &PageKind) -> PathBuf {
    store_dir().join(page_filename(kind))
}

/// Persist a canonical dataset for a given page.
/// Always writes headers first (if present), then rows.
pub fn save_dataset(kind: &PageKind, ds: &Dataset) -> io::Result<PathBuf> {
    let dir = store_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }

    let path = store_path(kind);
    let file = File::create(&path)?;
    let mut w = BufWriter::new(file);

    if let Some(h) = &ds.headers {
        write_row(&mut w, h, STORE_SEP)?;
    }
    for r in &ds.rows {
        write_row(&mut w, r, STORE_SEP)?;
    }
    // BufWriter drops will flush; explicit flush not required

    Ok(path)
}

/// Load cached dataset for a given page (if present).
/// Assumes first row is headers when present.
pub fn load_dataset(kind: &PageKind) -> io::Result<Dataset> {
    let path = store_path(kind);
    let text = fs::read_to_string(&path)?;
    let mut rows = parse_rows(&text, STORE_SEP);

    let headers = if !rows.is_empty() {
        Some(rows.remove(0))
    } else {
        None
    };

    Ok(Dataset { headers, rows })
}
