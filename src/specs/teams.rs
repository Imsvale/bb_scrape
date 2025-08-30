// src/specs/teams.rs
//! Scraping *spec* for Teams.
//!
//! Purpose:
//! - Parse the **remote HTML** of `/index.php` and extract canonical `(team_id, full_team_name)`.
//! - Prefer the **league table** `td.namecheck > a[href^="team.php?i="]` (full names like "Eduslum Marching Band").
//! - Fall back to the **mega-menu** (`class="mega-links"`) only if the table isn’t found.
//!
//! Responsibilities:
//! - Networking via `core::net::http_get`.
//! - HTML slicing via `core::html` helpers.
//! - Return a **scraped dataset** as `TeamsBundle { headers, rows }`.
//!
//! Non-Responsibilities (by design):
//! - **No caching / persistence.**
//! - **No GUI state, filtering, or export rules.**
//!
//! TL;DR: `specs/teams.rs` knows *how to read the page* and produce a raw table; it does not decide *when* or *whether* to save/use it.

use std::error::Error;
use crate::core::{ net, html };
use crate::core::html::{ next_tag_block_ci, strip_tags };

pub struct TeamsBundle {
    pub headers: Option<Vec<String>>,
    pub rows: Vec<Vec<String>>,
}

pub fn fetch() -> Result<TeamsBundle, Box<dyn Error>> {
    let html_doc = net::http_get("/index.php")?;

    // 1) Try the league table first (preferred, has full team names).
    let mut rows = scrape_from_league_table(&html_doc)?;

    // 2) Fallback: mega-menu short names (legacy behavior).
    if rows.is_empty() {
        rows = scrape_from_mega_menu(&html_doc)?;
    }

    // tidy
    rows.sort_by_key(|r| r.get(0).and_then(|s| s.parse::<u32>().ok()).unwrap_or(u32::MAX));
    rows.dedup_by(|a, b| a.get(0) == b.get(0));

    Ok(TeamsBundle {
        headers: Some(vec![s!("Id"), s!("Team")]),
        rows,
    })
}

/// Parse from the main league table:
///   <td class="namecheck"><a href="team.php?i=31">Eduslum Marching Band</a> ...</td>
fn scrape_from_league_table(doc: &str) -> Result<Vec<Vec<String>>, Box<dyn Error>> {
    let mut out: Vec<Vec<String>> = Vec::new();

    // Grab the first <table>...</table> block; the page uses a single centered table.
    let table_block = if let Some((ts, te)) = next_tag_block_ci(doc, "<table", "</table>", 0) {
        &doc[ts..te]
    } else {
        return Ok(out); // no table; caller will try fallback
    };

    // Walk the table block, find <td ... class="...namecheck..."> anchors to team.php?i=
    let bytes = table_block.as_bytes();
    let mut i = 0usize;
    let n = bytes.len();

    // tiny helpers
    let mut next_char_pos = |from: usize, ch: u8| -> Option<usize> {
        bytes.get(from..)?.iter().position(|&c| c == ch).map(|off| from + off)
    };
    let lower_contains = |s: &str, needle: &str| s.to_ascii_lowercase().contains(needle);

    while i < n {
        // find next '<'
        let lt = match next_char_pos(i, b'<') { Some(p) => p, None => break };
        if lt + 1 >= n { break; }

        // find matching '>'
        let gt = match next_char_pos(lt + 1, b'>') { Some(p) => p, None => break };

        // opener text
        let tag_text = &table_block[(lt + 1)..gt]; // e.g. "td class=…", "/td", "a href=…"
        let tag_text_trim = tag_text.trim();

        // parse tag name
        let mut name_end = 0usize;
        for (idx, ch) in tag_text_trim.bytes().enumerate() {
            if ch.is_ascii_alphabetic() || ch == b'/' { name_end = idx + 1; } else { break; }
        }
        let tag_name = &tag_text_trim[..name_end]; // "td", "/td", "a", "/a", ...
        let (is_close, name) = if tag_name.starts_with('/') {
            (true, &tag_name[1..])
        } else {
            (false, tag_name)
        };

        // Look for <td ... class=...namecheck...>
        if !is_close && name.eq_ignore_ascii_case("td") {
            let rest = &tag_text_trim[name.len()..];
            let rest_lc = rest.to_ascii_lowercase();

            let is_name_cell = rest_lc.contains(r#"class="namecheck""#)
                || rest_lc.contains(r#"class='namecheck'"#)
                || rest_lc.contains("namecheck");

            if is_name_cell {
                // Inner of this <td> ... </td>
                // Use a tolerant case-insensitive block capture starting after this opener.
                let from = gt + 1;
                // Find the matching closing </td> from 'from'
                if let Some((_, td_end)) = html::next_tag_block_ci(&table_block, "<td", "</td>", lt) {
                    let inner = &table_block[from..td_end - "</td>".len()];

                    // Find first <a ... href="team.php?i=ID">NAME</a> inside this cell.
                    // Extract ID and inner text up to </a>.
                    if let Some(a_pos) = inner.to_ascii_lowercase().find("<a") {
                        // opener
                        let opener_end = inner[a_pos..].find('>').map(|e| a_pos + e).unwrap_or(a_pos);
                        let a_open = &inner[a_pos..opener_end + 1];
                        let a_open_lc = a_open.to_ascii_lowercase();

                        // parse id from href
                        let mut team_id: Option<u32> = None;
                        if let Some(hp) = a_open_lc.find("href=") {
                            let val = a_open[hp + 5..].trim_start();
                            let (quote, start_off) = match val.as_bytes().first() {
                                Some(b'"') => ('"', 1),
                                Some(b'\'') => ('\'', 1),
                                _ => ('\0', 0),
                            };
                            let end = if quote != '\0' {
                                val[start_off..].find(quote).map(|e| start_off + e)
                            } else {
                                val.find(|c: char| c.is_ascii_whitespace() || c == '>')
                            }.unwrap_or(val.len());
                            let href_val = &val[start_off..end];

                            if let Some(idx) = href_val.to_ascii_lowercase().find("team.php?i=") {
                                let mut digits = String::new();
                                for ch in href_val[idx + "team.php?i=".len()..].chars() {
                                    if ch.is_ascii_digit() { digits.push(ch); } else { break; }
                                }
                                if !digits.is_empty() {
                                    team_id = digits.parse::<u32>().ok();
                                }
                            }
                        }

                        // anchor text up to </a>
                        let after_gt = opener_end + 1;
                        if let (Some(id), Some(close_rel)) = (team_id, inner[after_gt..].to_ascii_lowercase().find("</a>")) {
                            let close_abs = after_gt + close_rel;
                            let name = strip_tags(&inner[after_gt..close_abs]).trim().to_string();
                            if !name.is_empty() {
                                out.push(vec![id.to_string(), name]);
                            }
                        }
                    }

                    i = td_end; // jump to after </td>
                    continue;
                }
            }
        }

        i = gt + 1;
    }

    Ok(out)
}

/// Fallback: parse from the mega-menu (short names) if league table parsing yielded nothing.
fn scrape_from_mega_menu(html_doc: &str) -> Result<Vec<Vec<String>>, Box<dyn Error>> {
    let bytes = html_doc.as_bytes();
    let mut i = 0usize;
    let n = bytes.len();

    let mut in_mega = false;
    let mut rows: Vec<Vec<String>> = Vec::new();

    // tiny helpers
    let next_char_pos = |from: usize, ch: u8| -> Option<usize> {
        bytes[from..].iter().position(|&c| c == ch).map(|off| from + off)
    };

    while i < n {
        // find next '<'
        let lt = match next_char_pos(i, b'<') { Some(p) => p, None => break };
        // copy through any '<' that is actually escaped or malformed
        if lt + 1 >= n { break; }

        // Is it an end tag?
        let is_end = bytes.get(lt + 1) == Some(&b'/');

        // find matching '>' for this tag
        let gt = match next_char_pos(lt + 1, b'>') { Some(p) => p, None => break };

        // tag opener text (between '<' and '>')
        let tag_text = &html_doc[(lt + 1)..gt]; // e.g., "/ul", "ul class=…", "a href=…"
        let tag_text_trim = tag_text.trim();

        // parse tag name (letters only)
        let mut name_end = 0usize;
        for (idx, ch) in tag_text_trim.bytes().enumerate() {
            if ch.is_ascii_alphabetic() || ch == b'/' { name_end = idx + 1; } else { break; }
        }
        let tag_name = &tag_text_trim[..name_end]; // can be "ul", "/ul", "a", "/a", etc.
        let (is_close, name) = if tag_name.starts_with('/') {
            (true, &tag_name[1..])
        } else {
            (false, tag_name)
        };

        // ----- handle UL open/close to maintain in_mega -----
        if !is_close && !is_end && name.eq_ignore_ascii_case("ul") {
            // opening <ul ...>
            // look for class attribute in the rest of the opener
            let rest = &tag_text_trim[name.len()..];
            let rest_lc = rest.to_ascii_lowercase();
            let has_mega = rest_lc.contains(r#"class="mega-links""#)
                || rest_lc.contains(r#"class='mega-links'"#)
                || rest_lc.contains("mega-links"); // tolerant to multiple classes/order
            in_mega = has_mega;
            i = gt + 1;
            continue;
        } else if is_close && name.eq_ignore_ascii_case("ul") {
            // closing </ul>
            in_mega = false;
            i = gt + 1;
            continue;
        }

        // Inside mega-links: collect <a href="team.php?i=ID">SHORT NAME</a>
        if in_mega && !is_close && !is_end && name.eq_ignore_ascii_case("a") {
            // opener: extract href
            let rest = &tag_text_trim[name.len()..];
            let rest_lc = rest.to_ascii_lowercase();
            let href_pos = rest_lc.find("href=");
            let mut team_id: Option<u32> = None;

            if let Some(hp) = href_pos {
                let val = rest[hp + 5..].trim_start();
                let (quote, start_off) = match val.as_bytes().first() {
                    Some(b'"') => ('"', 1),
                    Some(b'\'') => ('\'', 1),
                    _ => ('\0', 0),
                };
                let end = if quote != '\0' {
                    val[start_off..].find(quote).map(|e| start_off + e)
                } else {
                    // unquoted: end at first whitespace
                    val.find(|c: char| c.is_ascii_whitespace())
                }.unwrap_or(val.len());
                let href_val = &val[start_off..end];

                if let Some(idx) = href_val.to_ascii_lowercase().find("team.php?i=") {
                    let mut digits = String::new();
                    for ch in href_val[idx + "team.php?i=".len()..].chars() {
                        if ch.is_ascii_digit() { digits.push(ch); } else { break; }
                    }
                    if !digits.is_empty() {
                        team_id = digits.parse::<u32>().ok();
                    }
                }
            }

            // find closing </a> to get the text
            let after_gt = gt + 1;
            let a_close = html_doc[after_gt..].to_ascii_lowercase().find("</a>");
            if let (Some(id), Some(close_rel)) = (team_id, a_close) {
                let close_abs = after_gt + close_rel;
                let name = strip_tags(&html_doc[after_gt..close_abs]).trim().to_string();
                rows.push(vec![id.to_string(), name]);
                i = close_abs + "</a>".len();
                continue;
            }
        }

        // default advance
        i = gt + 1;
    }

    Ok(rows)
}
