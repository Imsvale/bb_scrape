// src/engine/engine.rs
use crate::core::{net, html, sanitize};
use crate::core::sanitize::{normalize_entities, normalize_ws, strip_brackets};
use crate::engine::types::*;

pub fn build_path(spec: &TableSpec, id: Option<u32>) -> String {
    match id {
        Some(i) => spec.path_tmpl.replace("{id}", &i.to_string()),
        None => spec.path_tmpl.to_string(),
    }
}

pub fn extract(spec: &TableSpec, id: Option<u32>) -> Result<OutputBundle, Box<dyn std::error::Error>> {
    let path = build_path(spec, id);
    let html_doc = net::http_get(&path)?;
    let table = match spec.locator {
        Locator::TagWithAttr { tag, attr:_, value_sub } => {
            // simple: find `<table class=...value_sub...>`
            html::slice_between_ci(&html_doc, &format!("<{tag} class={}", value_sub), &format!("</{tag}>"))
                .ok_or("table not found")?
        }
        Locator::FirstTableWithHeader(_h) => { return Err("Locator not implemented".into()); }
    };

    // Team name (first row first td)
    let team_name = extract_team_name(table).unwrap_or_else(|| id.map_or("Team".into(), |i| format!("Team {}", i)));

    // Headers
    let site_headers = match spec.header_mode {
        HeaderMode::ConsecutiveTh => read_consecutive_th(table),
        HeaderMode::None => Vec::new(),
    };
    let headers = if !spec.header_ops.is_empty() {
        let mut h = spec.header_ops.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let tail = if spec.drop_first_header && site_headers.len() > 1 {
            site_headers.iter().skip(1).cloned().collect::<Vec<_>>()
        } else { site_headers.clone() };
        h.extend(tail);
        Some(h)
    } else { None };

    // Rows
    let mut out_rows: Vec<Vec<String>> = Vec::new();
    let mut pos = 0usize;
    while let Some((tr_s, tr_e)) = html::next_tag_block_ci(table, "<tr", "</tr>", pos) {
        let tr = &table[tr_s..tr_e]; pos = tr_e;
        let lc = html::to_lower(&tr[..tr.len().min(200)]);
        let keep = match spec.row_selector {
            RowSelector::TrClassAny(list) => list.iter().any(|c| lc.contains(&format!(r#"class="{}""#, c))),
        };
        if !keep { continue; }

        // cells
        let mut cells = Vec::new();
        let mut td_pos = 0usize;
        while let Some((td_s, td_e)) = html::next_tag_block_ci(tr, "<td", "</td>", td_pos) {
            let block = &tr[td_s..td_e];
            let inner = html::inner_after_open_tag(block);
            let clean = html::strip_tags(normalize_entities(&inner));
            cells.push(clean);
            td_pos = td_e;
        }
        if cells.is_empty() { continue; }

        if spec.split_fused_first_cell {
            let fused = strip_brackets(&cells.remove(0));
            let (mut name, num, mut race) = split_fused(&fused, /*keep_hash unused here*/ true);
            name = normalize_ws(&name); race = normalize_ws(&race);
            if let Some(ins) = spec.insert_team_at {
                let mut row = vec![name, num, race];
                // if caller wants to strip hash, they can do it later in shaping phase; or you can pass a flag here
                row.insert(ins, team_name.clone());
                row.extend(cells);
                out_rows.push(row);
            } else {
                let mut row = vec![name, num, race];
                row.extend(cells);
                out_rows.push(row);
            }
        } else {
            out_rows.push(cells);
        }
    }

    Ok(OutputBundle {
        filename_stem: sanitize::sanitize_team_filename(&team_name, id.unwrap_or(0)),
        headers: headers.filter(|h| !h.is_empty()),
        rows: out_rows,
    })
}

fn read_consecutive_th(table_inner: &str) -> Vec<String> {
    let mut headers = Vec::new(); let mut pos = 0usize; let mut started = false;
    while let Some((s,e)) = html::next_tag_block_ci(table_inner, "<th", "</th>", pos) {
        let inner = html::inner_after_open_tag(&table_inner[s..e]);
        let clean = html::strip_tags(normalize_entities(&inner));
        headers.push(clean); pos = e; started = true;
        let rest = &table_inner[pos..]; let rest_trim = rest.trim_start();
        if !rest_trim.to_ascii_lowercase().starts_with("<th") { break; }
    }
    if started { headers } else { Vec::new() }
}

/// First <tr><td> cell, trimmed and cut at " Team owner" or " | "
fn extract_team_name(table_inner: &str) -> Option<String> {
    if let Some((tr_s, tr_e)) = html::next_tag_block_ci(table_inner, "<tr", "</tr>", 0) {
        let tr = &table_inner[tr_s..tr_e];
        if let Some((td_s, td_e)) = html::next_tag_block_ci(tr, "<td", "</td>", 0) {
            let td = &tr[td_s..td_e];
            let mut txt = html::inner_after_open_tag(td);
            txt = html::strip_tags(normalize_entities(&txt));
            if let Some(i) = txt.find(" Team owner") { return Some(txt[..i].trim().to_string()); }
            if let Some(i) = txt.find(" | ") { return Some(txt[..i].trim().to_string()); }
            let t = txt.trim(); if !t.is_empty() { return Some(t.to_string()); }
        }
    }
    None
}

fn split_fused(fused: &str, keep_hash: bool) -> (String, String, String) {
    if let Some(hidx) = fused.find('#') {
        let name = fused[..hidx].trim().to_string();
        let rest = fused[hidx..].trim();
        if let Some(sp) = rest.find(' ') {
            let num_raw = &rest[1..sp];
            let race = rest[sp+1..].trim().to_string();
            let number = if keep_hash { format!("#{}", num_raw) } else { num_raw.to_string() };
            return (name, number, race);
        }
    }
    (fused.trim().to_string(), String::new(), String::new())
}
