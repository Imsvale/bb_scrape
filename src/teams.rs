// src/teams.rs

use std::{error::Error, fs, path::Path};

use crate::core::net;

/// Local cache file
const TEAM_NAMES_FILE: &str = "team_names.txt";

/// Load team names either from cache or the website.
/// Returns (id, name) pairs, always sorted by id.
pub fn load() -> Result<Vec<(u32, String)>, Box<dyn Error>> {
    if Path::new(TEAM_NAMES_FILE).exists() {
        if let Ok(text) = fs::read_to_string(TEAM_NAMES_FILE) {
            if let Ok(list) = parse_file(&text) {
                return Ok(list);
            }
        }
    }

    // fallback to live fetch
    let teams = fetch_all()?;
    // write cache
    let mut buf = s!();
    for (id, name) in &teams {
        buf.push_str(&format!("{},{}\n", id, name));
    }
    fs::write(TEAM_NAMES_FILE, buf)?;
    Ok(teams)
}

/// Parse a team_names.txt into Vec<(id, name)>
fn parse_file(text: &str) -> Result<Vec<(u32, String)>, Box<dyn Error>> {
    let mut out = Vec::new();
    for line in text.lines() {
        let mut parts = line.splitn(2, ',');
        let id_str = parts.next().ok_or("Malformed line")?;
        let name = parts.next().ok_or("Malformed line")?;
        let id: u32 = id_str.parse()?;
        out.push((id, name.to_string()));
    }
    Ok(out)
}

/// Fetch directly from the website (HTTP GET + scrape)
fn fetch_all() -> Result<Vec<(u32, String)>, Box<dyn Error>> {
    let html = net::http_get("/index.php")?;
    let mut teams = Vec::new();
    let needle = r#"href="team.php?i="#;
    let mut rest = html.as_str();

    while let Some(pos) = rest.find(needle) {
        rest = &rest[pos + needle.len()..];

        let mut id_str = s!();
        for c in rest.chars() {
            if c.is_ascii_digit() {
                id_str.push(c);
            } else {
                break;
            }
        }
        let id: u32 = id_str.parse()?;

        let gt = rest.find('>').ok_or("Malformed <a> tag")?;
        rest = &rest[gt + 1..];

        if let Some(end) = rest.find("</a>") {
            let name = rest[..end].trim().to_string();
            teams.push((id, name));
            rest = &rest[end + 4..];
        } else {
            break;
        }
    }

    // sort by ID to make predictable
    teams.sort_by_key(|(id, _)| *id);
    Ok(teams)
}
