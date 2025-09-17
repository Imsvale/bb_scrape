// src/scrape/injuries.rs
use std::error::Error;

use crate::core::{html, net, sanitize};
use std::time::Instant;
use crate::store::DataSet;
use crate::get_teams;
use crate::core::VisChars;
use std::collections::HashMap;

fn strip_tags_keep_text(s: &str) -> String {
    // Convert HTML entities first (&nbsp; -> ' ') then strip tags and normalize whitespace
    html::strip_tags(sanitize::normalize_entities(s))
}

fn longest_team_prefix<'a>(s: &'a str, teams: &'a [(u32, String)]) -> Option<(&'a str, &'a str)> {
    // return (team_name, rest)
    let mut best: Option<&str> = None;
    for (_, name) in teams {
        let n = name.as_str();
        if s.starts_with(n) {
            match best { Some(prev) if prev.len() >= n.len() => {}, _ => best = Some(n) }
        }
    }
    if let Some(team) = best { Some((team, s[team.len()..].trim_start())) } else { None }
}

fn parse_line_slow(line: &str, season: &str, teams: &[(u32, String)]) -> Option<Vec<String>> {
    // Expect one event per <br> line. Strip html then parse by keywords.
    let txt = strip_tags_keep_text(line).replace("\r", "");
    logd!("Injuries: parse_line raw='{}'", &txt);
    if !txt.contains(" DUR ") || !txt.contains(" BRU ") || !txt.contains(" SR ") { return None; }

    // week
    let t = txt.trim_start();
    let w_idx = t.find('W')?;
    let mut pos = w_idx + 1;
    let mut week_digits = String::new();
    for ch in t[pos..].chars() { if ch.is_ascii_digit() { week_digits.push(ch); pos += 1; } else { break; } }
    let week = week_digits;
    logd!("Injuries: week={}", week);
    // Skip any leftover spaces
    let mut rest = t[pos..].trim_start();

    // victim team + victim until " DUR "
    let dur_pos = rest.find(" DUR ")?;
    let pre = rest[..dur_pos].trim_end();
    let (victim_team, mut victim_name) = if let Some((tn, rem)) = longest_team_prefix(pre, teams) {
        (tn.to_string(), rem.trim().to_string())
    } else {
        // fallback: split last word as start of name (best‑effort)
        let parts: Vec<&str> = pre.split_whitespace().collect();
        if parts.len() < 2 { return None; }
        let (vt, vn) = parts.split_at(parts.len().saturating_sub(2));
        (vt.join(" "), vn.join(" "))
    };
    // Extract any trailing " SR XX" from the victim name (KILLED variant places SR here)
    let mut sr_from_name: Option<String> = None;
    if let Some(ix) = victim_name.rfind(" SR ") {
        let tail = victim_name[ix+4..].trim();
        let mut digits = String::new();
        for ch in tail.chars() { if ch.is_ascii_digit() { digits.push(ch); } else { break; } }
        if !digits.is_empty() {
            sr_from_name = Some(digits);
            victim_name = victim_name[..ix].trim().to_string();
        }
    }

    rest = &rest[dur_pos + " DUR ".len()..];
    // DUR
    let mut dur_digits = String::new();
    for ch in rest.chars() { if ch.is_ascii_digit() { dur_digits.push(ch); } else { break; } }
    let dur = dur_digits;
    logd!("Injuries: victim_team='{}' victim='{}' dur={}", victim_team, victim_name, dur);
    // after DUR digits comes space then Type up to " by "
    let after_dur = rest[dur.len()..].trim_start();
    let by_pos = after_dur.find(" by ")?;
    let mut inj_type = after_dur[..by_pos].trim().to_string();
    logd!("Injuries: type='{}'", inj_type);

    rest = &after_dur[by_pos + " by ".len()..];
    let bru_pos = rest.find(" BRU ")?;
    let mut offender_pre = rest[..bru_pos].trim_end().to_string();
    // Handle the odd "by by <team>" double 'by' variant
    if offender_pre.to_ascii_lowercase().starts_with("by ") {
        offender_pre = offender_pre[3..].to_string();
    }
    let (off_team, offender) = if let Some((tn, rem)) = longest_team_prefix(&offender_pre, teams) {
        (tn.to_string(), rem.trim().to_string())
    } else {
        let parts: Vec<&str> = offender_pre.split_whitespace().collect();
        if parts.len() < 2 { (offender_pre.to_string(), String::new()) } else { let (vt, vn) = parts.split_at(parts.len()-2); (vt.join(" "), vn.join(" ")) }
    };

    rest = &rest[bru_pos + " BRU ".len()..];
    let mut bru_digits = String::new();
    for ch in rest.chars() { if ch.is_ascii_digit() { bru_digits.push(ch); } else { break; } }
    let bru = bru_digits;
    logd!("Injuries: offender_team='{}' offender='{}' bru={}", off_team, offender, bru);

    let after_bru = rest[bru.len()..].trim_start();
    // SR Drops from A to B
    let (mut sr0, mut sr1) = (String::new(), String::new());
    if let Some(from_pos) = after_bru.find("Drops from ") {
        let after_from = &after_bru[from_pos + "Drops from ".len()..];
        for ch in after_from.chars() { if ch.is_ascii_digit() { sr0.push(ch); } else { break; } }
        let after_sr0 = after_from[sr0.len()..].trim_start();
        // robust: find the next "to" token (with flexible spacing) and read digits after it
        let mut to_idx_opt = after_sr0.find(" to ");
        if to_idx_opt.is_none() { to_idx_opt = after_sr0.find(" to"); }
        if to_idx_opt.is_none() { to_idx_opt = after_sr0.find("to "); }
        if let Some(to_pos) = to_idx_opt {
            for ch in after_sr0[to_pos + 2..].chars() { // skip "to"
                if ch.is_ascii_whitespace() { continue; }
                if ch.is_ascii_digit() { sr1.push(ch); } else { break; }
            }
        }
        // Fallback: scan two digit groups after "Drops from"
        if sr1.is_empty() {
            let mut nums: Vec<String> = Vec::new();
            let mut cur = String::new();
            for ch in after_from.chars() {
                if ch.is_ascii_digit() { cur.push(ch); }
                else { if !cur.is_empty() { nums.push(std::mem::take(&mut cur)); } }
            }
            if !cur.is_empty() { nums.push(cur); }
            if nums.len() >= 2 {
                sr0 = nums[0].clone();
                sr1 = nums[1].clone();
            }
        }
    } else {
        logd!("Injuries: no SR delta found (likely KILLED/variant)");
        // KILLED formatting: ensure type is normalized to just "KILLED"
        if inj_type.to_ascii_uppercase().contains("KILL") { inj_type = "KILLED".into(); }
    }
    logd!("Injuries: SR0='{}' SR1='{}'", sr0, sr1);

    // Bounty marker
    let bounty = if txt.to_ascii_uppercase().contains("BOUNTY COLLECTED") { "BOUNTY COLLECTED" } else { "" };
    if !bounty.is_empty() { logd!("Injuries: bounty collected"); }

    // For KILLED lines, SR0 can live in the victim name; use it if we didn't parse any SR0
    if sr0.is_empty() {
        if let Some(s) = sr_from_name { sr0 = s; logd!("Injuries: used SR0 from victim name"); }
    }

    Some(vec![
        season.to_string(),
        week,
        victim_team,
        victim_name,
        dur,
        sr0,
        sr1,
        inj_type,
        off_team,
        offender,
        bru,
        bounty.to_string(),
    ])
}

// archived slower LTI/ALT parsers removed

pub fn collect_injuries(mut _progress: Option<&mut dyn crate::progress::Progress>) -> Result<DataSet, Box<dyn Error>> {
    logd!("Injuries: HTTP GET injury.php");
    let doc = net::http_get("injury.php")?;
    logd!("Injuries: fetched {} bytes", doc.len());
    // Try to find season from the document title if present; otherwise blank
    let mut season = {
        let lc = doc.to_ascii_lowercase();
        if let Some(idx) = lc.find("season ") {
            let mut s = String::new();
            for ch in doc[idx+7..].chars() { if ch.is_ascii_digit() { s.push(ch); } else { break; } }
            s
        } else { String::new() }
    };
    if season.is_empty() {
        if let Ok(Some(s)) = crate::store::load_season() { season = s.to_string(); }
    }
    
    let teams = get_teams::load().unwrap_or_default();
    logd!("Injuries: team list loaded ({} teams)", teams.len());
    let tindex = TeamIndex::new(&teams);
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut seen = 0usize;

    // Split by <br> to be robust against missing newlines
    for (i, chunk) in doc.split("<br>").enumerate() {
        if chunk.contains(" DUR ") {
            let preview: String = chunk.trim().chars().take(120).collect();
            logd!("Injuries: consider chunk #{}: {}...", i, preview);

            // Use fast parser by default in production path
            let now = Instant::now();
            let res = parse_line_fast_idx(chunk, &season, &tindex);
            let _dt = now.elapsed();

            match res {
                Some(r) => { rows.push(r); seen += 1; },
                None => { logd!("Injuries: parse failed on chunk #{}", i); }
            }
        }
    }
    logd!("Injuries: parsed {} event rows", seen);

    let headers = Some(vec![
        "S","W","Victim Team","Victim","DUR","SR0","SR1","Type","Offender Team","Offender","BRU","Bounty"
    ].iter().map(|s| s.to_string()).collect());

    Ok(DataSet { headers, rows })
}

/// Public helpers for benchmarking parsers on an arbitrary document (no network).
pub fn parse_doc_current(doc: &str, season: &str, teams: &[(u32, String)]) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    for chunk in doc.split("<br>") {
        if chunk.contains(" DUR ") {
            if let Some(r) = parse_line_slow(chunk, season, teams) { rows.push(r); }
        }
    }
    rows
}

// Lightweight first-char index for team names to reduce comparisons.
struct TeamIndex<'a> {
    by_first: HashMap<char, Vec<&'a str>>, // lowercase first char -> names
}

impl<'a> TeamIndex<'a> {
    fn new(teams: &'a [(u32, String)]) -> Self {
        let mut m: HashMap<char, Vec<&'a str>> = HashMap::with_capacity(64);
        for (_, name) in teams {
            if let Some(fc) = name.chars().next() {
                m.entry(fc.to_ascii_lowercase()).or_default().push(name.as_str());
            }
        }
        // sort by length desc to allow early longest-match
        for v in m.values_mut() { v.sort_by_key(|s| std::cmp::Reverse(s.len())); }
        Self { by_first: m }
    }

    fn split_prefix<'b>(&self, s: &'b str) -> Option<(&'a str, &'b str)> {
        let fc = s.chars().next()?.to_ascii_lowercase();
        if let Some(cands) = self.by_first.get(&fc) {
            for &cand in cands {
                if s.starts_with(cand) {
                    return Some((cand, s[cand.len()..].trim_start()));
                }
            }
        }
        None
    }
}

// removed alt/lti doc parsers (archived)

// -------- Fast single-pass parser using VisChars (char-by-char, no full string)

fn parse_line_fast_base(line: &str, season: &str, teams: &[(u32, String)]) -> Option<Vec<String>> {
    // Helper inline matchers
    struct Matcher { pat: &'static [u8], idx: usize, lower: bool }
    impl Matcher {
        fn new(p: &'static str, lower: bool) -> Self { Self { pat: p.as_bytes(), idx: 0, lower } }
        fn feed(&mut self, ch: char) -> bool {
            let c = if self.lower { ch.to_ascii_lowercase() as u8 } else { ch as u8 };
            let need = self.pat[self.idx];
            if c == need { self.idx += 1; if self.idx == self.pat.len() { self.idx = 0; return true; } }
            else { self.idx = if c == self.pat[0] { 1 } else { 0 }; }
            false
        }
    }

    let mut it = VisChars::new(line);

    // 1) Find week: 'W' digits
    let mut week = String::new();
    let mut saw_w = false;
    while let Some(ch) = it.next() {
        if !saw_w {
            if ch == 'W' { saw_w = true; }
            continue;
        } else {
            if ch.is_ascii_digit() { week.push(ch); } else { break; }
        }
    }
    if week.is_empty() { return None; }

    // 2) Collect victim segment until " DUR "
    let mut pre = String::new();
    let mut m_dur = Matcher::new(" DUR ", false);
    while let Some(ch) = it.next() {
        if m_dur.feed(ch) { break; }
        pre.push(ch);
    }
    if pre.is_empty() { return None; }
    // The streaming matcher adds partial token chars; strip them if present
    if pre.ends_with(" DUR") { pre.truncate(pre.len() - 4); }
    let pre = pre.trim();
    // split team/name
    let (victim_team, mut victim_name) = if let Some((tn, rem)) = longest_team_prefix(pre, teams) {
        (tn.to_string(), rem.trim().to_string())
    } else {
        let parts: Vec<&str> = pre.split_whitespace().collect();
        if parts.len()<2 { return None; }
        let (a,b)=parts.split_at(parts.len()-2); (a.join(" "), b.join(" "))
    };
    // peel trailing SR from victim name if present
    let mut sr_from_name: Option<String> = None;
    if let Some(ix) = victim_name.rfind(" SR ") {
        let tail = victim_name[ix+4..].trim();
        let mut d=String::new(); for ch in tail.chars(){ if ch.is_ascii_digit(){ d.push(ch);} else {break;} }
        if !d.is_empty(){ sr_from_name=Some(d); victim_name = victim_name[..ix].trim().to_string(); }
    }

    // 3) DUR digits
    let mut dur = String::new();
    while let Some(ch) = it.next() { if ch.is_ascii_digit() { dur.push(ch);} else { if ch!=' ' { break; } else { break; } } }
    if dur.is_empty() { return None; }

    // 4) Type until " by "
    let mut typ = String::new(); let mut m_by = Matcher::new(" by ", true);
    while let Some(ch) = it.next() {
        if m_by.feed(ch) { break; }
        typ.push(ch);
    }
    let mut typ = typ.trim().to_string();
    if typ.ends_with(" by") { typ.truncate(typ.len() - 3); }

    // 5) Offender segment until " BRU "
    let mut offender_pre = String::new(); let mut m_bru = Matcher::new(" BRU ", false);
    while let Some(ch) = it.next() {
        if m_bru.feed(ch) { break; }
        offender_pre.push(ch);
    }
    if offender_pre.ends_with(" BRU") { offender_pre.truncate(offender_pre.len() - 4); }
    let offender_pre = offender_pre.trim();
    let offender_pre = offender_pre.strip_prefix("by ").unwrap_or(offender_pre).trim();
    let (off_team, offender) = if let Some((tn, rem)) = longest_team_prefix(offender_pre, teams) {
        (tn.to_string(), rem.trim().to_string())
    } else {
        let parts: Vec<&str> = offender_pre.split_whitespace().collect();
        if parts.len()<2 { (offender_pre.to_string(), String::new()) } else { let (a,b)=parts.split_at(parts.len()-2); (a.join(" "), b.join(" ")) }
    };

    // 6) BRU digits
    let mut bru = String::new();
    while let Some(ch) = it.next() { if ch.is_ascii_digit(){ bru.push(ch);} else { break; } }

    // 7) Post BRU: watch for "Drops from ", then digits A, then " to ", digits B; also bounty "BOUNTY COLLECTED"
    let mut sr0 = String::new(); let mut sr1 = String::new();
    let mut m_drops = Matcher::new("drops from ", true);
    let mut m_to = Matcher::new(" to ", true);
    let mut m_bounty = Matcher::new("bounty collected", true);
    let mut saw_bounty = false;
    let mut phase = 0; // 0: searching Drops; 1: reading SR0; 2: waiting ' to '; 3: reading SR1; 4: done
    while let Some(ch) = it.next() {
        // bounty detection in background
        if m_bounty.feed(ch) { saw_bounty = true; }

        match phase {
            0 => { if m_drops.feed(ch) { phase = 1; } }
            1 => { if ch.is_ascii_digit(){ sr0.push(ch);} else { if m_to.feed(ch) { phase = 3; } } }
            2 => unreachable!(),
            3 => { if ch.is_ascii_digit(){ sr1.push(ch);} else { /* stop after first non-digit */ phase = 4; }
            }
            _ => {}
        }
    }
    if sr0.is_empty(){ if let Some(s)=sr_from_name { sr0 = s; } }
    let bounty = if saw_bounty { "BOUNTY COLLECTED" } else { "" };
    let mut typ = typ;
    if sr1.is_empty() && typ.to_ascii_uppercase().contains("KILL") { typ = "KILLED".into(); }

    Some(vec![
        season.to_string(), week, victim_team, victim_name, dur, sr0, sr1, typ, off_team, offender, bru, bounty.to_string()
    ])
}

fn parse_line_fast_idx<'a>(line: &str, season: &str, tindex: &TeamIndex<'a>) -> Option<Vec<String>> {
    // Same as base but uses TeamIndex for faster prefix match
    struct Matcher { pat: &'static [u8], idx: usize, lower: bool }
    impl Matcher { fn new(p: &'static str, lower: bool) -> Self { Self{ pat:p.as_bytes(), idx:0, lower } }
        fn feed(&mut self, ch: char) -> bool { let c = if self.lower { ch.to_ascii_lowercase() as u8 } else { ch as u8 }; let need=self.pat[self.idx]; if c==need{ self.idx+=1; if self.idx==self.pat.len(){ self.idx=0; return true; } } else { self.idx = if c==self.pat[0]{1}else{0}; } false } }
    let mut it = VisChars::new(line);
    let mut week = String::new(); let mut saw_w=false; while let Some(ch)=it.next(){ if !saw_w{ if ch=='W'{saw_w=true;} continue; } else { if ch.is_ascii_digit(){ week.push(ch);} else {break;} } }
    if week.is_empty(){return None;}
    let mut pre=String::new(); let mut m_dur=Matcher::new(" DUR ",false); while let Some(ch)=it.next(){ if m_dur.feed(ch){break;} pre.push(ch);} if pre.is_empty(){return None;} if pre.ends_with(" DUR"){ pre.truncate(pre.len()-4);} let pre=pre.trim();
    let (victim_team, mut victim_name) = if let Some((tn, rem)) = tindex.split_prefix(pre) { (tn.to_string(), rem.trim().to_string()) } else { let parts:Vec<&str>=pre.split_whitespace().collect(); if parts.len()<2{return None;} let(a,b)=parts.split_at(parts.len()-2); (a.join(" "), b.join(" ")) };
    let mut sr_from_name:Option<String>=None; if let Some(ix)=victim_name.rfind(" SR "){ let tail=victim_name[ix+4..].trim(); let mut d=String::new(); for ch in tail.chars(){ if ch.is_ascii_digit(){d.push(ch);} else {break;} } if !d.is_empty(){ sr_from_name=Some(d); victim_name=victim_name[..ix].trim().to_string(); } }
    let mut dur=String::new(); while let Some(ch)=it.next(){ if ch.is_ascii_digit(){ dur.push(ch);} else { break; } } if dur.is_empty(){return None;}
    let mut typ=String::new(); let mut m_by=Matcher::new(" by ",true); while let Some(ch)=it.next(){ if m_by.feed(ch){break;} typ.push(ch);} let mut typ=typ.trim().to_string(); if typ.ends_with(" by"){ typ.truncate(typ.len()-3);} 
    let mut offender_pre=String::new(); let mut m_bru=Matcher::new(" BRU ",false); while let Some(ch)=it.next(){ if m_bru.feed(ch){break;} offender_pre.push(ch);} if offender_pre.ends_with(" BRU"){ offender_pre.truncate(offender_pre.len()-4);} let offender_pre=offender_pre.trim(); let offender_pre=offender_pre.strip_prefix("by ").unwrap_or(offender_pre).trim();
    let (off_team, offender) = if let Some((tn, rem)) = tindex.split_prefix(offender_pre) { (tn.to_string(), rem.trim().to_string()) } else { let parts:Vec<&str>=offender_pre.split_whitespace().collect(); if parts.len()<2 { (offender_pre.to_string(), String::new()) } else { let(a,b)=parts.split_at(parts.len()-2); (a.join(" "), b.join(" ")) } };
    let mut bru=String::new(); while let Some(ch)=it.next(){ if ch.is_ascii_digit(){ bru.push(ch);} else { break; } }
    let mut sr0=String::new(); let mut sr1=String::new(); let mut m_drops=Matcher::new("drops from ",true); let mut m_to=Matcher::new(" to ",true); let mut m_bounty=Matcher::new("bounty collected",true); let mut saw_bounty=false; let mut phase=0; while let Some(ch)=it.next(){ if m_bounty.feed(ch){ saw_bounty=true; } match phase{ 0=>{ if m_drops.feed(ch){ phase=1; } } 1=>{ if ch.is_ascii_digit(){ sr0.push(ch);} else { if m_to.feed(ch){ phase=3; } } } 3=>{ if ch.is_ascii_digit(){ sr1.push(ch);} else { phase=4; } } _=>{} } }
    if sr0.is_empty(){ if let Some(s)=sr_from_name{ sr0=s; } }
    let bounty = if saw_bounty { "BOUNTY COLLECTED" } else { "" };
    let mut typ=typ; if sr1.is_empty() && typ.to_ascii_uppercase().contains("KILL"){ typ="KILLED".into(); }
    Some(vec![ season.to_string(), week, victim_team, victim_name, dur, sr0, sr1, typ, off_team, offender, bru, bounty.to_string() ])
}

pub fn parse_doc_fast_base(doc: &str, season: &str, teams: &[(u32, String)]) -> Vec<Vec<String>> {
    let mut rows=Vec::new(); for chunk in doc.split("<br>") { if chunk.contains(" DUR ") { if let Some(r)=parse_line_fast_base(chunk, season, teams){ rows.push(r);} } } rows
}

pub fn parse_doc_fast_idx(doc: &str, season: &str, teams: &[(u32, String)]) -> Vec<Vec<String>> {
    let idx = TeamIndex::new(teams); let mut rows=Vec::new(); for chunk in doc.split("<br>") { if chunk.contains(" DUR ") { if let Some(r)=parse_line_fast_idx(chunk, season, &idx){ rows.push(r);} } } rows
}

pub fn parse_doc_fast(doc: &str, season: &str, teams: &[(u32, String)]) -> Vec<Vec<String>> { parse_doc_fast_idx(doc, season, teams) }
// removed slower LTI parser

#[cfg(test)]
mod tests {
    use super::*;

    fn load_sample() -> String {
        std::fs::read_to_string(".ignore/page_samples/injury.txt").expect("read injury sample")
    }

    fn load_teams() -> Vec<(u32, String)> {
        crate::get_teams::load().unwrap_or_else(|_| crate::scrape::list_teams())
    }

    #[test]
    fn fast_parser_matches_current_on_sample() {
        let doc = load_sample();
        let teams = load_teams();
        let season = "";

        let slow = parse_doc_current(&doc, season, &teams);
        let fast = parse_doc_fast_idx(&doc, season, &teams);

        assert_eq!(slow.len(), fast.len(), "row count differs: {} vs {}", slow.len(), fast.len());

        for (i, (a, b)) in slow.iter().zip(fast.iter()).enumerate() {
            assert_eq!(a, b, "row {} differs:\nslow={:?}\nfast={:?}", i, a, b);
        }
    }
}
