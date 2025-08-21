// src/specs/players.rs

use std::error::Error;

use crate::core::{net, html};
use crate::core::html::{slice_between_ci, next_tag_block_ci, inner_after_open_tag, strip_tags};
use crate::core::sanitize::{normalize_entities, normalize_ws};

pub struct RosterBundle {
    pub team_name: String,
    pub headers: Option<Vec<String>>,
    pub rows: Vec<Vec<String>>,
}

pub fn fetch_and_extract(
    team_id: u32,
    keep_hash: bool,
    include_headers: bool,
) -> Result<RosterBundle, Box<dyn Error>> {
    let path = format!("team.php?i={}", team_id);
    let html_doc = net::http_get(&path)?; // see core/net.rs
    let table = slice_between_ci(&html_doc, "<table class=teamroster", "</table>")
        .ok_or("teamroster table not found")?;

    let team_name = extract_team_name(table).unwrap_or_else(|| format!("Team {}", team_id));

    // Headers (<th> not necessarily wrapped in <tr>)
    let site_headers = read_site_headers_row(table);
    let headers = if include_headers {
        let mut hdr = vec!["Name".to_string(), "Number".to_string(), "Race".to_string(), "Team".to_string()];
        if !site_headers.is_empty() {
            // Drop fused "Player" header if present
            let tail = if !site_headers.is_empty() && site_headers[0].to_ascii_lowercase().contains("name") {
                site_headers.iter().skip(1).cloned().collect::<Vec<_>>()
            } else {
                site_headers.clone()
            };
            hdr.extend(tail);
        }
        Some(hdr)
    } else { None };

    // Player rows
    let mut rows_out = Vec::new();
    let mut pos = 0usize;
    while let Some((tr_s, tr_e)) = next_tag_block_ci(table, "<tr", "</tr>", pos) {
        let tr = &table[tr_s..tr_e];
        pos = tr_e;

        // Only player rows
        let prefix = &tr[..tr.len().min(200)];
        let lc = html::to_lower(prefix);
        let is_player = lc.contains(r#"class="playerrow""#) || lc.contains(r#"class="playerrow1""#);
        if !is_player { continue; }

        // <td> cells
        let mut cells = Vec::new();
        let mut td_pos = 0usize;
        while let Some((td_s, td_e)) = next_tag_block_ci(tr, "<td", "</td>", td_pos) {
            let block = &tr[td_s..td_e];
            let inner = inner_after_open_tag(block);
            let clean = strip_tags(normalize_entities(&inner));
            cells.push(clean);
            td_pos = td_e;
        }
        if cells.is_empty() { continue; }

        // First cell: fused Name #Num Race, with possible [META]
        let fused = remove_bracket_tags(&cells.remove(0));
        let (mut name, num, mut race) = split_first_cell(&fused, keep_hash);
        name = normalize_ws(&name);
        race = normalize_ws(&race);

        // Row: Name, Number, Race, Team, rest...
        let mut row = Vec::with_capacity(4 + cells.len());
        row.push(name);
        row.push(num);
        row.push(race);
        row.push(team_name.clone());
        row.extend(cells);
        rows_out.push(row);
    }

    Ok(RosterBundle { team_name, headers, rows: rows_out })
}

/* ---------- helpers ---------- */

fn extract_team_name(table_inner: &str) -> Option<String> {
    if let Some((tr_s, tr_e)) = next_tag_block_ci(table_inner, "<tr", "</tr>", 0) {
        let tr = &table_inner[tr_s..tr_e];
        if let Some((td_s, td_e)) = next_tag_block_ci(tr, "<td", "</td>", 0) {
            let td = &tr[td_s..td_e];
            let mut txt = inner_after_open_tag(td);
            txt = strip_tags(normalize_entities(&txt));

            if let Some(i) = txt.find(" Team owner") { return Some(txt[..i].trim().to_string()); }
            if let Some(i) = txt.find(" | ") { return Some(txt[..i].trim().to_string()); }
            let t = txt.trim(); if !t.is_empty() { return Some(t.to_string()); }
        }
    }
    None
}

/// Read consecutive <th>…</th> header cells. Works even if not wrapped in <tr>.
fn read_site_headers_row(table_inner: &str) -> Vec<String> {
    let mut headers = Vec::new();
    let mut pos = 0usize;
    let mut started = false;

    while let Some((th_s, th_e)) = next_tag_block_ci(table_inner, "<th", "</th>", pos) {
        let th_block = &table_inner[th_s..th_e];
        let inner = inner_after_open_tag(th_block);
        let clean = strip_tags(normalize_entities(&inner));
        headers.push(clean);
        pos = th_e;
        started = true;

        // Stop when next non-ws isn't <th>
        let rest_trim = table_inner[pos..].trim_start();
        if !rest_trim.to_ascii_lowercase().starts_with("<th") {
            break;
        }
    }

    if started { headers } else { Vec::new() }
}

/// "Name #27 Common Drakon" → ("Name", "27" or "#27", "Common Drakon")
fn split_first_cell(fused: &str, keep_hash: bool) -> (String, String, String) {
    if let Some(hidx) = fused.find('#') {
        let name = fused[..hidx].trim().to_string();
        let rest = fused[hidx..].trim(); // starts with '#'
        let mut parts = rest.splitn(2, ' ');
        let raw_num = parts.next().unwrap_or(""); // "#27" or similar
        let race = parts.next().unwrap_or("").trim().to_string();

        // Keep or strip the hash according to flag
        let num = if keep_hash {
            raw_num.to_string()
        } else {
            raw_num.trim_start_matches('#').to_string()
        };

        (name, num, race)
    } else {
        (fused.trim().to_string(), s!(), s!())
    }
}

/// Remove any `[ ... ]` segments (e.g. `[CAPTAIN]`, `[unavailable ...]`).
fn remove_bracket_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_bracket = false;
    for ch in s.chars() {
        match ch {
            '[' => in_bracket = true,
            ']' => in_bracket = false,
            _ if !in_bracket => out.push(ch),
            _ => {}
        }
    }
    out.trim().to_string()
}
