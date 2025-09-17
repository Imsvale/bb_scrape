// src/specs/players.rs

use std::error::Error;

use crate::core::{net, html};
use crate::core::html::{slice_between_ci, next_tag_block_ci, inner_after_open_tag, strip_tags};
use crate::core::sanitize::{normalize_entities, normalize_ws, letters_only_trim};

pub struct RosterBundle {
    pub headers: Option<Vec<String>>,
    pub rows: Vec<Vec<String>>,
}

pub fn fetch_and_extract(
    team_id: u32,
) -> Result<RosterBundle, Box<dyn Error>> {
    let path = format!("team.php?i={}", team_id);
    let html_doc = net::http_get(&path)?; // see core/net.rs
    let table = slice_between_ci(&html_doc, "<table class=teamroster", "</table>")
        .ok_or("teamroster table not found")?;

    let team_name = extract_team_name(table).unwrap_or_else(|| format!("Team {}", team_id));

    // Headers (<th> not necessarily wrapped in <tr>)
    let site_headers = read_site_headers_row(table);

    // Always construct headers: Name, Number, Race, Team, then the site's tail
    let headers = {
        let mut hdr = vec![
            s!("Name"), 
            s!("#"), 
            s!("Race"), 
            s!("Team")
        ];

        if !site_headers.is_empty() {
            // If the first site header already says "Name", drop it to avoid duplicates
            let tail = if site_headers[0].to_ascii_lowercase().contains("name") {
                site_headers.iter().skip(1).cloned().collect::<Vec<_>>()
            } else {
                site_headers.clone()
            };
            hdr.extend(tail);
        }
        Some(hdr)
    };

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
        let (mut name, num, mut race) = split_first_cell(&fused);
        name = normalize_ws(&name);
        race = normalize_ws(&race);

        // Row: Name, #Number, Race, Team, rest...
        let mut row = Vec::with_capacity(4 + cells.len());
        row.push(name);
        row.push(num);
        row.push(race);
        row.push(team_name.clone());
        row.extend(cells);
        rows_out.push(row);
    }

    Ok(RosterBundle { headers, rows: rows_out })
}

/* ---------- helpers ---------- */

fn extract_team_name(table_inner: &str) -> Option<String> {
    if let Some((tr_s, tr_e)) = next_tag_block_ci(table_inner, "<tr", "</tr>", 0) {
        let tr = &table_inner[tr_s..tr_e];
        if let Some((td_s, td_e)) = next_tag_block_ci(tr, "<td", "</td>", 0) {
            let td = &tr[td_s..td_e];
            let mut txt = inner_after_open_tag(td);
            txt = strip_tags(normalize_entities(&txt));

            // New site format appends season record in parentheses, e.g. "Team (6 - 0 - 2)".
            // Strip known suffixes while extracting the clean team name.
            if let Some(i) = txt.find(" Team owner") {
                let name = strip_record_suffix(&txt[..i]);
                return Some(letters_only_trim(name.trim()));
            }
            if let Some(i) = txt.find(" | ") {
                let name = strip_record_suffix(&txt[..i]);
                return Some(letters_only_trim(name.trim()));
            }
            let t = strip_record_suffix(txt.trim());
            let t = letters_only_trim(&t);
            if !t.is_empty() { return Some(t); }
        }
    }
    None
}

/// Remove a trailing parenthesized season record like "(6 - 0 - 2)".
/// Conservative check: only if the parentheses content contains only digits,
/// spaces and hyphens, with at least one hyphen and some digits.
fn strip_record_suffix(s: &str) -> String {
    let t = s.trim();
    if t.ends_with(')') {
        if let Some(open) = t.rfind('(') {
            if open > 0 {
                let inner = &t[open + 1..t.len() - 1];
                let has_digit = inner.chars().any(|c| c.is_ascii_digit());
                let hyphens = inner.chars().filter(|&c| c == '-').count();
                let ok_chars = inner.chars().all(|c| c.is_ascii_digit() || c == '-' || c.is_ascii_whitespace());
                if has_digit && hyphens >= 1 && ok_chars {
                    return t[..open].trim().to_string();
                }
            }
        }
    }
    t.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::sanitize::letters_only_trim;

    #[test]
    fn letters_only_trims_at_non_letter() {
        assert_eq!(letters_only_trim("Failurewood Hills (6 - 0 - 2)"), "Failurewood Hills");
        assert_eq!(letters_only_trim("Alpha Beta,"), "Alpha Beta");
        assert_eq!(letters_only_trim("OnlyLetters"), "OnlyLetters");
    }

    #[test]
    fn strip_record_suffix_variants() {
        assert_eq!(strip_record_suffix("Team (6 - 0 - 2)"), "Team");
        assert_eq!(strip_record_suffix("Team (Champions)"), "Team (Champions)");
    }

    #[test]
    fn extract_team_name_handles_owner_and_pipe() {
        let table = r#"
            <tr><td><h5>Failurewood Hills (6 - 0 - 2) Team owner Foo</h5></td></tr>
        "#;
        assert_eq!(extract_team_name(table).as_deref(), Some("Failurewood Hills"));

        let table2 = r#"
            <tr><td><h5>My Team | Division Alpha</h5></td></tr>
        "#;
        assert_eq!(extract_team_name(table2).as_deref(), Some("My Team"));
    }

    #[test]
    fn extract_team_name_trims_digits_and_punct() {
        let table = r#"
            <tr><td><h5>Team 2</h5></td></tr>
        "#;
        assert_eq!(extract_team_name(table).as_deref(), Some("Team"));

        let table2 = r#"
            <tr><td><h5>Team-Name</h5></td></tr>
        "#;
        assert_eq!(extract_team_name(table2).as_deref(), Some("Team"));
    }

    #[test]
    fn split_first_cell_variants() {
        assert_eq!(split_first_cell("Name #27 Race"), ("Name".into(), "#27".into(), "Race".into()));
        assert_eq!(split_first_cell("Name #27"), ("Name".into(), "#27".into(), "".into()));
        assert_eq!(split_first_cell("Name"), ("Name".into(), "".into(), "".into()));
    }

    #[test]
    fn remove_bracket_tags_works() {
        assert_eq!(remove_bracket_tags("[CAPTAIN] Name [out]"), "Name");
    }

    #[test]
    fn read_site_headers_row_consecutive_th() {
        let html = r#"
            <table>
              <th>A</th><th>B</th>  <td>stop</td>
            </table>
        "#;
        let inner = html; // function scans whole string for <th> blocks
        let hdrs = read_site_headers_row(inner);
        assert_eq!(hdrs, vec!["A", "B"]);
    }
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
fn split_first_cell(fused: &str) -> (String, String, String) {
    if let Some(hidx) = fused.find('#') {
        let name = s!(fused[..hidx].trim());
        let rest = fused[hidx..].trim(); // starts with '#'
        let mut parts = rest.splitn(2, ' ');
        let num = s!(parts.next().unwrap_or("")); // "#27" or similar
        let race = s!(parts.next().unwrap_or("").trim());

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
