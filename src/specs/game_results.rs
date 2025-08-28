use std::error::Error;

use crate::core::{html, net};
use crate::core::html::{next_tag_block_ci, inner_after_open_tag, strip_tags};
use crate::core::sanitize::{normalize_entities, normalize_ws};

/// Output bundle (shapes neatly into store::DataSet)
pub struct GameResultsBundle {
    pub headers: Option<Vec<String>>,
    pub rows: Vec<Vec<String>>,
}

/// Scrape the full-season schedule/results from /season.php.
/// - Includes future games (blank scores, no match id).
/// - Columns: Season, Week, Home team, Home result, Away result, Away team, Match id
pub fn fetch() -> Result<GameResultsBundle, Box<dyn Error>> {
    let html_doc = net::http_get("/season.php")?;

    // Season detection: <title>…Season N</title>, else fall back to stats pages, else empty.
    let season_str = detect_season(&html_doc)
        .or_else(|| detect_season_from("/stat_team.php").ok().flatten())
        .or_else(|| detect_season_from("/stat_team_performance.php").ok().flatten())
        .unwrap_or_else(|| s!(""));

    let mut rows_out: Vec<Vec<String>> = Vec::new();

    // Walk each <table …>…</table> block (each week is a standalone table).
    let mut pos = 0usize;
    while let Some((tb_s, tb_e)) = next_tag_block_ci(&html_doc, "<table", "</table>", pos) {
        let table = &html_doc[tb_s..tb_e];
        pos = tb_e;

        // Extract week number from the header row: <td colspan=4 class="conference">WEEK X</td>
        let week = extract_week_number(table);
        if week.is_none() {
            continue; // not one of the week tables; skip
        }
        let week_str = week.unwrap();

        // Iterate each game row: <tr class="playerrow"> / <tr class="playerrow1">
        let mut tr_pos = 0usize;
        while let Some((tr_s, tr_e)) = next_tag_block_ci(table, "<tr", "</tr>", tr_pos) {
            let tr_block = &table[tr_s..tr_e];
            tr_pos = tr_e;

            let head = html::to_lower(&tr_block[..tr_block.len().min(180)]);
            let is_game = head.contains(r#"class="playerrow""#) || head.contains(r#"class="playerrow1""#);
            if !is_game { continue; }

            // We expect 4 <td> blocks: basichome | spacer | basicaway | spacer/link
            let mut tds: Vec<&str> = Vec::with_capacity(4);
            let mut td_pos = 0usize;
            while let Some((td_s, td_e)) = next_tag_block_ci(tr_block, "<td", "</td>", td_pos) {
                tds.push(&tr_block[td_s..td_e]);
                td_pos = td_e;
            }
            if tds.len() < 3 {
                continue; // defensive: weird row
            }

            // Home side (td[0])
            let (home_team, home_score) = extract_side(tds[0], /*home=*/true);

            // Away side (td[2] if present; otherwise td[1] in malformed cases)
            let away_td_idx = if tds.len() >= 3 { 2 } else { 1 };
            let (away_team, away_score) = extract_side(tds[away_td_idx], /*home=*/false);

            // Match ID from last td (if present)
            let match_id = tds
                .last()
                .copied()
                .and_then(extract_match_id)
                .unwrap_or_else(|| s!(""));

            rows_out.push(vec![
                season_str.clone(),
                week_str.clone(),
                home_team,
                home_score,
                away_score,
                away_team,
                match_id,
            ]);
        }
    }

    Ok(GameResultsBundle {
        headers: Some(vec![
            s!("S"),
            s!("W"),
            s!("Home team"),
            s!("Home"),
            s!("Away"),
            s!("Away team"),
            s!("Match id"),
        ]),
        rows: rows_out,
    })
}

/* ---------------- helpers ---------------- */

fn detect_season(doc: &str) -> Option<String> {
    // Sniff <title>Brutalball Schedule - Season N</title>
    // Be forgiving about whitespace/case.
    if let Some((s, e)) = next_tag_block_ci(doc, "<title", "</title>", 0) {
        let title_inner = inner_after_open_tag(&doc[s..e]);
        let clean = normalize_ws(&strip_tags(normalize_entities(&title_inner)));
        // Find "Season " followed by digits anywhere in the title
        if let Some(idx) = clean.to_ascii_lowercase().find("season") {
            let tail = clean[idx..].chars().collect::<String>();
            let mut digits = String::new();
            for ch in tail.chars() {
                if ch.is_ascii_digit() {
                    digits.push(ch);
                } else if !digits.is_empty() {
                    break;
                }
            }
            if !digits.is_empty() {
                return Some(digits);
            }
        }
    }
    None
}

fn detect_season_from(path: &str) -> Result<Option<String>, Box<dyn Error>> {
    let doc = net::http_get(path)?;
    Ok(detect_season(&doc))
}

fn extract_week_number(table_html: &str) -> Option<String> {
    // Look for first <td … class="conference">…WEEK N…</td>
    let mut pos = 0usize;
    while let Some((td_s, td_e)) = next_tag_block_ci(table_html, "<td", "</td>", pos) {
        let td_block = &table_html[td_s..td_e];
        pos = td_e;

        // Check class attr in opener
        let opener = &td_block[..td_block.find('>').unwrap_or(td_block.len())];
        let opener_lc = opener.to_ascii_lowercase();
        if !(opener_lc.contains(r#"class="conference""#)
            || opener_lc.contains(r#"class='conference'"#)
            || opener_lc.contains("conference"))
        {
            continue;
        }

        let inner = inner_after_open_tag(td_block);
        let clean = normalize_ws(&strip_tags(normalize_entities(&inner)));
        // Expect “…WEEK N…”
        let lc = clean.to_ascii_lowercase();
        if let Some(i) = lc.find("week") {
            // collect trailing digits
            let mut digits = String::new();
            for ch in clean[i + 4..].chars() {
                if ch.is_ascii_digit() { digits.push(ch); }
                else if !digits.is_empty() { break; }
            }
            if !digits.is_empty() {
                return Some(digits);
            }
        }
        // Found conference td but didn't parse? Bail out to avoid wrong matches later.
        break;
    }
    None
}

fn extract_side(td_block: &str, home: bool) -> (String, String) {
    // Team name = first <a>…</a> text
    let team = {
        // Find <a …>…</a>
        if let Some((a_s, a_e)) = next_tag_block_ci(td_block, "<a", "</a>", 0) {
            let a_inner = inner_after_open_tag(&td_block[a_s..a_e]);
            let clean = normalize_ws(&strip_tags(normalize_entities(&a_inner)));
            clean
        } else {
            s!()
        }
    };

    // Score = first <strong>…</strong> digits (home score usually after team link, away before it; both covered)
    let score = {
        if let Some((s_s, s_e)) = next_tag_block_ci(td_block, "<strong", "</strong>", 0) {
            let inner = inner_after_open_tag(&td_block[s_s..s_e]);
            let txt = normalize_ws(&strip_tags(normalize_entities(&inner)));
            txt.chars().filter(|c| c.is_ascii_digit()).collect::<String>()
        } else {
            s!("")
        }
    };

    // Minimal polish: if team text contains stray entities/nbsp around the edges, normalize again.
    (team, score)
}

fn extract_match_id(td_block: &str) -> Option<String> {
    // Search opener for href=game.php?i=NNNN (quotes optional in source)
    // 1) Look for <a …> opener
    if let Some((a_s, a_e)) = next_tag_block_ci(td_block, "<a", ">", 0) {
        let opener = &td_block[a_s..a_e];
        let lc = opener.to_ascii_lowercase();
        if let Some(hp) = lc.find("href=") {
            let val = &opener[hp + 5..]; // after href=
            // Quote type?
            let (quote, start_off) = match val.as_bytes().first() {
                Some(b'"') => ('"', 1),
                Some(b'\'') => ('\'', 1),
                _ => ('\0', 0),
            };
            let end = if quote != '\0' {
                val[start_off..].find(quote).map(|e| start_off + e).unwrap_or(val.len())
            } else {
                val.find(|c: char| c.is_ascii_whitespace()).unwrap_or(val.len())
            };
            let href_val = &val[start_off..end];
            let href_lc = href_val.to_ascii_lowercase();
            if let Some(idx) = href_lc.find("game.php?i=") {
                let mut digits = String::new();
                for ch in href_val[idx + "game.php?i=".len()..].chars() {
                    if ch.is_ascii_digit() { digits.push(ch); } else { break; }
                }
                if !digits.is_empty() {
                    return Some(digits);
                }
            }
        }
    }
    None
}
