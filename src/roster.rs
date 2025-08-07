// /src/roster.rs
// Logic for extracting player rows from a team page's HTML table.

use crate::html::{
    slice_between_ci, next_tag_block_ci, inner_after_open_tag,
    strip_tags, normalize_entities, to_lowercase_fast,
};

/// Given a full team page HTML and a team ID,
/// return a vector of CSV-ready rows: [Name, #Number, Race, Team, <attributes...>].
pub fn extract_player_rows(html: &str, team_id: u32) -> Result<Vec<Vec<String>>, Box<dyn std::error::Error>> {
    // Isolate the roster table contents
    let table = slice_between_ci(html, "<table class=teamroster", "</table>")
        .ok_or("teamroster table not found")?;

    // Extract team name from first row
    let team_name = extract_team_name(table)
        .unwrap_or_else(|| format!("Team {}", team_id));

    let mut out_rows: Vec<Vec<String>> = Vec::new();

    // Iterate over all <tr> blocks in the table
    let mut pos = 0usize;
    while let Some((row_start, row_end)) = next_tag_block_ci(table, "<tr", "</tr>", pos) {
        let tr = &table[row_start..row_end];
        pos = row_end;

        // Keep only player rows (identified by class attribute)
        let lc = to_lowercase_fast(&tr[..tr.len().min(200)]);
        let is_player = lc.contains(r#"class="playerrow""#) || lc.contains(r#"class="playerrow1""#);
        if !is_player {
            continue;
        }

        // Extract each <td> cell's text
        let mut tds = Vec::new();
        let mut td_pos = 0usize;
        while let Some((td_s, td_e)) = next_tag_block_ci(tr, "<td", "</td>", td_pos) {
            let td_block = &tr[td_s..td_e];
            let inner = inner_after_open_tag(td_block);
            let clean = strip_tags(normalize_entities(&inner));
            tds.push(clean);
            td_pos = td_e;
        }
        if tds.is_empty() {
            continue;
        }

        // First cell is fused: Name, #Number, Race → split it
        let fused = tds.remove(0);
        let (name, num_hash, race) = split_first_cell(&fused);

        // Compose final row: Name, #Number, Race, Team, rest...
        let mut fields = Vec::with_capacity(4 + tds.len());
        fields.push(name);
        fields.push(num_hash);
        fields.push(race);
        fields.push(team_name.clone());
        fields.extend(tds);

        out_rows.push(fields);
    }

    Ok(out_rows)
}

/// First row's first cell text; cut at " Team owner" or " | " if present.
fn extract_team_name(table_inner: &str) -> Option<String> {
    if let Some((tr_s, tr_e)) = next_tag_block_ci(table_inner, "<tr", "</tr>", 0) {
        let tr = &table_inner[tr_s..tr_e];
        if let Some((td_s, td_e)) = next_tag_block_ci(tr, "<td", "</td>", 0) {
            let td = &tr[td_s..td_e];
            let mut txt = inner_after_open_tag(td);
            txt = strip_tags(normalize_entities(&txt));

            if let Some(i) = txt.find(" Team owner") {
                return Some(txt[..i].trim().to_string());
            }
            if let Some(i) = txt.find(" | ") {
                return Some(txt[..i].trim().to_string());
            }
            let t = txt.trim();
            if !t.is_empty() { return Some(t.to_string()); }
        }
    }
    None
}

/// "Name #27 Common Drakon" → ("Name", "#27", "Common Drakon")
fn split_first_cell(fused: &str) -> (String, String, String) {
    if let Some(hidx) = fused.find('#') {
        let name = fused[..hidx].trim().to_string();
        let rest = fused[hidx..].trim(); // starts with '#'
        let mut parts = rest.splitn(2, ' ');
        let num = parts.next().unwrap_or("").to_string();
        let race = parts.next().unwrap_or("").trim().to_string();
        (name, num, race)
    } else {
        (fused.trim().to_string(), String::new(), String::new())
    }
}
