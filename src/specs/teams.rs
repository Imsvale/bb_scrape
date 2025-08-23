// src/specs/teams.rs
use std::error::Error;
use crate::core::{ 
    net,
    html, 
    sanitize,
};

pub struct TeamsBundle {
    pub headers: Option<Vec<String>>,
    pub rows: Vec<Vec<String>>,
}

/// Scrape the index and return (Id, Team) rows.
pub fn fetch() -> Result<TeamsBundle, Box<dyn Error>> {
    let html_doc = net::http_get("/index.php")?;
    
    // Restrict to mega-links block (div or ul, case-insensitive)
    let scope = html::slice_between_ci(&html_doc, r#"<div class="mega-links"#, "</div>")
        .or_else(|| html::slice_between_ci(&html_doc, r#"<ul class="mega-links"#, "</ul>"))
        .unwrap_or(&html_doc);

    let mut rows = Vec::new();
    let needle = r#"href="team.php?i="#;
    let mut rest = scope;

    while let Some(pos) = rest.find(needle) {
        rest = &rest[pos + needle.len()..];

        // parse id
        let mut id_str = String::new();
        for c in rest.chars() {
            if c.is_ascii_digit() { id_str.push(c); } else { break; }
        }
        let id: u32 = id_str.parse()?;

        // advance to > and read until </a>
        let gt = rest.find('>').ok_or("Malformed <a> tag")?;
        rest = &rest[gt + 1..];

        if let Some(end) = rest.find("</a>") {

            let raw = &rest[..end];

            let name = html::strip_tags(
                sanitize::normalize_entities(raw))
                .trim()
                .to_string();

            rows.push(vec![id.to_string(), name]);
            rest = &rest[end + 4..];
        } else {
            break;
        }
    }

    // Sort by id, stable
    rows.sort_by_key(|r| r.get(0).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0));

    // Deduplicate: same numeric id → keep first
    rows.dedup_by(|a, b| a.get(0) == b.get(0)); 

    Ok(TeamsBundle {
        headers: Some(vec![s!("Id"), s!("Team")]),
        rows,
    })
}
