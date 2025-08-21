// src/store.rs
use std::{fs, io, path::{PathBuf}, time::SystemTime, collections::HashMap};
use crate::csv::{self, parse_rows, detect_headers, Delim};
use crate::params::{DEFAULT_OUT_DIR, PLAYERS_SUBDIR, DEFAULT_SINGLE_FILE};

pub struct Dataset { pub headers: Option<Vec<String>>, pub rows: Vec<Vec<String>> }

fn headers_path() -> PathBuf {
    PathBuf::from(DEFAULT_OUT_DIR).join(PLAYERS_SUBDIR).join("$headers")
}

pub fn save_players_headers(headers: &[String]) -> io::Result<()> {
    let p = headers_path();

    // Ensure parent directories exist
    if let Some(parent) = p.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let file = fs::File::create(&p)?; // Use the computed path here
    let mut writer = io::BufWriter::new(file);
    csv::write_row(&mut writer, headers, Delim::Csv)?;

    Ok(())
}

pub fn load_players_headers() -> Option<Vec<String>> {
    let p = headers_path();
    let txt = fs::read_to_string(p).ok()?;
    let rows = parse_rows(&txt, Delim::Csv);
    rows.into_iter().next()
}

pub fn load_players_local() -> Result<Dataset, Box<dyn std::error::Error>> {
    let dir = PathBuf::from(DEFAULT_OUT_DIR).join(PLAYERS_SUBDIR);
    let merged_path = dir.join(DEFAULT_SINGLE_FILE);

    let mut headers = load_players_headers();
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut merged_mtime: Option<SystemTime> = None;

    if merged_path.exists() {
        let text = fs::read_to_string(&merged_path)?;
        let parsed = parse_rows(&text, Delim::Csv);
        let (h, r) = detect_headers(parsed);
        if headers.is_none() { headers = h; }
        rows = r;
        merged_mtime = fs::metadata(&merged_path).ok().and_then(|m| m.modified().ok());
    }

    let mut by_team: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    for row in rows.drain(..) {
        if row.len() > 3 {
            by_team.entry(row[3].clone()).or_default().push(row);
        }
    }

    if dir.exists() {
        for entry in fs::read_dir(&dir)? {
            let path = entry?.path();
            if !path.is_file() { continue; }
            if path.file_name().and_then(|s| s.to_str()) == Some(DEFAULT_SINGLE_FILE) { continue; }
            if path.extension().and_then(|s| s.to_str()).unwrap_or("") != "csv" { continue; }

            let team_file_mtime = fs::metadata(&path).ok().and_then(|m| m.modified().ok());
            let newer = match (team_file_mtime, merged_mtime) {
                (Some(tf), Some(mg)) => tf > mg,
                (Some(_), None)      => true,
                _                    => false,
            };
            if !newer { continue; }

            let text = fs::read_to_string(&path)?;
            let parsed = parse_rows(&text, Delim::Csv);
            let (h2, rows2) = detect_headers(parsed);
            if headers.is_none() { headers = h2; }
            if let Some(first) = rows2.get(0) {
                if first.len() > 3 {
                    by_team.insert(first[3].clone(), rows2);
                    continue;
                }
            }
        }
    }

    // If we still have no headers but we’re “Players”, synthesize canonical
    if headers.is_none() {
        headers = Some(vec![
            "Name","Number","Race","Team","XP","TV","OVR","RN","HB","QB","GN","BK","DL","LB","CV",
            "Spd","Str","Agl","Stm","Tck","Blk","Ddg","BrB","Hnd","Pas","Vis","Bru","Dur","Sal"
        ].into_iter().map(|s| s.to_string()).collect());
    }

    let mut final_rows = Vec::new();
    for (_, mut rs) in by_team { final_rows.append(&mut rs); }

    Ok(Dataset { headers, rows: final_rows })
}
