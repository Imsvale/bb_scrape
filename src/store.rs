// src/store.rs
use std::{
    fs::{ self, File },
    io::{ BufWriter, Result },
    path::PathBuf,
};

use crate::file::{parse_rows, write_row};
use crate::config::options::PageKind;
use crate::config::consts::{STORE_DIR, STORE_SEP};

/// Load cached dataset for a given page (if present).
/// Assumes first row is headers when present.
pub fn load_dataset(kind: &PageKind) -> Result<Dataset> {
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

/// Persist a canonical dataset for a given page.
/// Always writes headers first (if present), then rows.
pub fn save_dataset(kind: &PageKind, ds: &Dataset) -> Result<PathBuf> {
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

fn store_dir() -> PathBuf {
    PathBuf::from(STORE_DIR)
}

fn store_path(kind: &PageKind) -> PathBuf {
    store_dir().join(page_filename(kind))
}

fn page_filename(kind: &PageKind) -> &'static str {
    match kind {
        PageKind::Teams         => "teams",
        PageKind::Players       => "players",
        PageKind::SeasonStats   => "season_stats",
        PageKind::CareerStats   => "career_stats",
        PageKind::Injuries      => "injuries",
        PageKind::GameResults   => "game_results",
    }
}

#[derive(Clone, Debug)]
pub struct Dataset {
    pub headers: Option<Vec<String>>,
    pub rows: Vec<Vec<String>>,
}

