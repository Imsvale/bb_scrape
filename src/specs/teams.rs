// src/specs/teams.rs
use std::error::Error;
use crate::core::{ net, html };
use crate::core::html::{ next_tag_block_ci, strip_tags };

pub struct TeamsBundle {
    pub headers: Option<Vec<String>>,
    pub rows: Vec<Vec<String>>,
}

pub fn fetch() -> Result<TeamsBundle, Box<dyn Error>> {
    let html_doc = net::http_get("/index.php")?;
    let mut rows = Vec::new();

    let mut pos = 0usize;
    // Walk every <ul>...</ul> block in the document
    while let Some((ul_s, ul_e)) = next_tag_block_ci(&html_doc, "<ul", "</ul>", pos) {
        let ul_block = &html_doc[ul_s..ul_e];
        pos = ul_e;

        // Keep only those with class="mega-links"
        if let Some(gt) = ul_block.find('>') {
            let open = &ul_block[..gt];                       // the <ul ...> opener
            let lc = html::to_lower(open);
            if !lc.contains(r#"class="mega-links""#) && !lc.contains("mega-links") {
                continue;
            }
        } else {
            continue;
        }

        // Inside this <ul>, scan all anchors to team.php?i=*
        let needle = r#"href="team.php?i="#;
        let mut rest = ul_block;
        while let Some(p) = rest.find(needle) {
            rest = &rest[p + needle.len()..];

            // parse numeric id
            let mut id_str = String::new();
            for c in rest.chars() {
                if c.is_ascii_digit() { id_str.push(c); } else { break; }
            }
            let id: u32 = id_str.parse()?;

            // advance to '>' and read anchor text until </a>
            let gt = rest.find('>').ok_or("Malformed <a> tag")?;
            rest = &rest[gt + 1..];

            if let Some(end) = rest.find("</a>") {
                let name = strip_tags(&rest[..end]).trim().to_string();
                rows.push(vec![id.to_string(), name]);
                rest = &rest[end + 4..];
            } else {
                break;
            }
        }
    }

    // Be predictable
    rows.sort_by_key(|r| r.get(0).and_then(|s| s.parse::<u32>().ok()).unwrap_or(u32::MAX));
    rows.dedup_by(|a, b| a.get(0) == b.get(0)); // in case a team appears twice

    Ok(TeamsBundle {
        headers: Some(vec![s!("Id"), s!("Team")]),
        rows,
    })
}
