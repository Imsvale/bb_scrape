// src/specs/game_results.rs
use std::error::Error;

use crate::core::{html, net};
use crate::core::html::{next_tag_block_ci, inner_after_open_tag, strip_tags};
use crate::core::sanitize::{normalize_entities, normalize_ws, letters_only_trim};

/// Output bundle (shapes neatly into store::DataSet)
pub struct GameResultsBundle {
    pub headers: Option<Vec<String>>,
    pub rows: Vec<Vec<String>>,
}

/// Scrape the full-season schedule/results from /season.php.
/// - Includes future games (blank scores, no match id).
/// - Columns: Season, Week, Home team, Home, Away, Away team, Match id
pub fn fetch() -> Result<GameResultsBundle, Box<dyn Error>> {
    let html_doc = net::http_get("season.php")?;
    let t = std::time::Instant::now();
    let out = parse_doc(&html_doc);
    logd!("Results: Parse season.php in {:?}", t.elapsed());
    Ok(out)
}

/// Split out for unit tests.
pub fn parse_doc(html_doc: &str) -> GameResultsBundle {
    // Season detection: <title>…Season N</title>, else fall back to stats pages, else empty.
    let season_str = detect_season(html_doc)
        .unwrap_or_else(|| s!(""));

    let mut rows_out: Vec<Vec<String>> = Vec::new();

    // Walk each <table …>…</table> block (each week is a standalone table).
    let mut pos = 0usize;
    while let Some((tb_s, tb_e)) = next_tag_block_ci(html_doc, "<table", "</table>", pos) {
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

            // Gather TD blocks
            let mut tds: Vec<&str> = Vec::with_capacity(4);
            let mut td_pos = 0usize;
            while let Some((td_s, td_e)) = next_tag_block_ci(tr_block, "<td", "</td>", td_pos) {
                tds.push(&tr_block[td_s..td_e]);
                td_pos = td_e;
            }
            if tds.len() < 3 {
                continue; // defensive: weird row
            }

            // ---- NEW: select by class, not position ----
            fn opener_lc(td: &str) -> String {
                let end = td.find('>').unwrap_or(td.len());
                td[..end].to_ascii_lowercase()
            }
            fn td_has_class(td: &str, needle: &str) -> bool {
                let lc = opener_lc(td);
                // tolerate single quotes, double quotes, unquoted, multi-class
                lc.contains(&format!(r#"class="{}""#, needle))
                    || lc.contains(&format!(r#"class='{}'"#, needle))
                    || (lc.contains("class=") && lc.contains(needle))
            }

            // According to the updated page:
            //   basichome  -> AWAY column (left)
            //   basicaway  -> HOME column (right)
            let away_td_opt = tds.iter().copied().find(|td| td_has_class(td, "basichome"));
            let home_td_opt = tds.iter().copied().find(|td| td_has_class(td, "basicaway"));

            // Prefer class-based mapping; fall back to old positional assumption if missing.
            let (home_team, home_score, away_team, away_score) = match (home_td_opt, away_td_opt) {
                (Some(home_td), Some(away_td)) => {
                    let (home_team, home_score) = extract_side(home_td);
                    let (away_team, away_score) = extract_side(away_td);
                    (home_team, home_score, away_team, away_score)
                }
                _ => {
                    // Legacy mapping: left = AWAY, right = HOME (no classes present).
                    let (away_team, away_score) = extract_side(tds[0]);
                    let home_td_idx = if tds.len() >= 3 { 2 } else { 1 };
                    let (home_team, home_score) = extract_side(tds[home_td_idx]);
                    (home_team, home_score, away_team, away_score)
                }
            };  

            // Minor robustness tweak / 
            use crate::config::consts::SCRAPE_FLIP_SIDES;
            let (home_team, home_score, away_team, away_score) = if SCRAPE_FLIP_SIDES {
                (away_team, away_score, home_team, home_score)
            } else {
                (home_team, home_score, away_team, away_score)
            };

            // Match ID from last td (optional for future games)
            let match_id = tds
                .last()
                .and_then(|td| extract_match_id(td))
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

    GameResultsBundle {
        headers: None,
        rows: rows_out,
    }
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
            let tail = &clean[idx + "season".len()..];
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

fn extract_side(td_block: &str) -> (String, String) {
    // Team name = first <a>…</a> text
    let team = {
        if let Some((a_s, a_e)) = next_tag_block_ci(td_block, "<a", "</a>", 0) {
            let a_inner = inner_after_open_tag(&td_block[a_s..a_e]);
            let raw = normalize_ws(&strip_tags(normalize_entities(&a_inner)));
            letters_only_trim(&raw)
        } else {
            s!()
        }
    };

    // Score = first <strong>…</strong> digits (present only for completed games)
    let score = {
        if let Some((s_s, s_e)) = next_tag_block_ci(td_block, "<strong", "</strong>", 0) {
            let inner = inner_after_open_tag(&td_block[s_s..s_e]);
            let txt = normalize_ws(&strip_tags(normalize_entities(&inner)));
            txt.chars().filter(|c| c.is_ascii_digit()).collect::<String>()
        } else {
            s!("")
        }
    };

    (team, score)
}


fn extract_match_id(td_block: &str) -> Option<String> {
    // Search opener for href=game.php?i=NNNN (quotes optional in source)
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

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal smoke test using a short synthetic snippet that follows the new layout.
    #[test]
    fn parses_one_game_new_layout() {
        let doc = r#"
            <html><head><title>Brutalball Schedule - Season 5</title></head>
            <body>
              <table>
                <tr><td colspan=4 class="conference">WEEK 2</td></tr>
                <tr class="playerrow">
                  <td class="basichome"><a href="team.php?i=10">Budget Roadies</a> &nbsp; <strong>6</strong></td>
                  <td class=spacer>&nbsp;</td>
                  <td class="basicaway"><strong>8</strong> &nbsp;<a href="team.php?i=24">Sportsball Union</a></td>
                  <td class=spacer align=center><a href=game.php?i=2241></a></td>
                </tr>
              </table>
            </body></html>
        "#;

        let out = parse_doc(doc);
        // assert_eq!(out.headers.as_ref().unwrap().len(), 7);
        assert_eq!(out.rows.len(), 1);
        let row = &out.rows[0];
        // S, W, Home team, Home, Away, Away team, Match id
        assert_eq!(row[0], "5");
        assert_eq!(row[1], "2");
        assert_eq!(row[2], "Sportsball Union"); // home
        assert_eq!(row[3], "8");
        assert_eq!(row[4], "6");
        assert_eq!(row[5], "Budget Roadies"); // away
        assert_eq!(row[6], "2241");
    }

    #[test]
    fn future_games_have_blank_scores_and_no_matchid() {
        let doc = r#"
            <html><head><title>Brutalball Schedule - Season 5</title></head>
            <body>
              <table>
                <tr><td colspan=4 class="conference">WEEK 9</td></tr>
                <tr class="playerrow">
                  <td class="basichome"><a href="team.php?i=12">Blood Pit Bouncers</a></td>
                  <td class=spacer>&nbsp;</td>
                  <td class="basicaway"><a href="team.php?i=4">Bumson Medics</a></td>
                  <td class=spacer align=center>&nbsp;</td>
                </tr>
              </table>
            </body></html>
        "#;

        let out = parse_doc(doc);
        assert_eq!(out.rows.len(), 1);
        let row = &out.rows[0];
        assert_eq!(row[1], "9");
        assert_eq!(row[2], "Bumson Medics"); // home
        assert_eq!(row[3], "");              // home score blank
        assert_eq!(row[4], "");              // away score blank
        assert_eq!(row[5], "Blood Pit Bouncers"); // away
        assert_eq!(row[6], "");              // no match id
    }

    #[test]
    fn parses_old_layout_left_away_right_home() {
        let doc = r#"
            <html><head><title>Brutalball Schedule - Season 3</title></head>
            <body>
              <table>
                <tr><td colspan=4 class="conference">WEEK 1</td></tr>
                <tr class="playerrow">
                  <td><a href="team.php?i=2">Away Team 2</a> <strong>3</strong></td>
                  <td class=spacer>&nbsp;</td>
                  <td><strong>7</strong> <a href="team.php?i=5">Home Team 5</a></td>
                  <td class=spacer align=center>&nbsp;</td>
                </tr>
              </table>
            </body></html>
        "#;

        let out = parse_doc(doc);
        assert_eq!(out.rows.len(), 1);
        let row = &out.rows[0];
        // S, W, Home team, Home, Away, Away team, Match id
        assert_eq!(row[0], "3");
        assert_eq!(row[1], "1");
        assert_eq!(row[2], "Home Team"); // trimmed non-letters
        assert_eq!(row[3], "7");
        assert_eq!(row[4], "3");
        assert_eq!(row[5], "Away Team"); // trimmed non-letters
    }
}
