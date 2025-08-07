// /src/main.rs
// Minimal, std-only scraper for Brutalball team rosters.
// Usage:
//   cargo run --release -- -t 20
//   cargo run --release -- --all -o players_all.csv
//
// Output: NO HEADERS. Each row: Name, #Number, Race, Team, <attributes…>
// Assumptions (by design):
// - table has class="teamroster"
// - player rows have class="playerrow" or "playerrow1"
// - first cell is "Name #Number Race"

use std::env;
use std::fs::File;
use std::io::{Read, Write, BufWriter};
use std::net::TcpStream;
use std::time::Duration;

struct Cli {
    all: bool,
    one_team: Option<u32>,
    out: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = parse_cli()?;

    let team_ids: Vec<u32> = if cli.all {
        (0..32).collect()
    } else {
        vec![cli.one_team.unwrap()]
    };

    let mut out = BufWriter::new(File::create(&cli.out)?);

    for tid in team_ids {
        let url_path = format!("/brutalball/team.php?i={}", tid);
        let html = http_get("dozerverse.com", 80, &url_path)?;

        let table = slice_between_ci(&html, r#"<table class=teamroster"#, "</table>")
            .ok_or("teamroster table not found")?;

        // Team name: first tr > first td text, trimmed before " Team owner" or " | "
        let team_name = extract_team_name(table).unwrap_or_else(|| format!("Team {}", tid));

        // Iterate <tr>…</tr>
        let mut pos = 0usize;
        while let Some((row_start, row_end)) = next_tag_block_ci(table, "<tr", "</tr>", pos) {
            let tr = &table[row_start..row_end];
            pos = row_end;

            // Only player rows
            let lc = to_lowercase_fast(&tr[..tr.len().min(200)]);
            if !(lc.contains(r#"class="playerrow""#) || lc.contains(r#"class="playerrow1""#)) {
                continue;
            }

            // Collect <td>…</td>
            let mut tds = Vec::new();
            let mut td_pos = 0usize;
            while let Some((td_s, td_e)) = next_tag_block_ci(tr, "<td", "</td>", td_pos) {
                let td_inner = inner_after_open_tag(&tr[td_s..td_e]);
                tds.push(strip_tags(normalize_entities(&td_inner)));
                td_pos = td_e;
            }
            if tds.is_empty() {
                continue;
            }

            // Split fused first cell: "Name #27 Common Drakon"
            let fused = tds.remove(0);
            let (name, number_hash, race) = split_first_cell(&fused);

            // Row: Name, #Number, Race, Team, <rest…>
            let mut fields = Vec::with_capacity(4 + tds.len());
            fields.push(name);
            fields.push(number_hash);
            fields.push(race);
            fields.push(team_name.clone());
            fields.extend(tds);

            write_csv_row(&mut out, &fields)?;
        }
    }

    out.flush()?;
    println!("Wrote {}", cli.out);
    Ok(())
}

fn parse_cli() -> Result<Cli, Box<dyn std::error::Error>> {
    let mut all = false;
    let mut one_team: Option<u32> = None;
    let mut out = String::new();

    let mut args = env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--all" => all = true,
            "-t" | "--team" => {
                let v: u32 = args.next().ok_or("Missing team id")?.parse()?;
                if v >= 32 { return Err("Team id out of range (0..31)".into()); }
                one_team = Some(v);
            }
            "-o" | "--out" => out = args.next().ok_or("Missing output file")?,
            "-h" | "--help" => {
                eprintln!("Usage: --all | -t <id> [-o <output.csv>]");
                std::process::exit(0);
            }
            _ => return Err(format!("Unknown arg: {}", a).into()),
        }
    }
    if !all && one_team.is_none() {
        return Err("Specify --all or -t <id>".into());
    }
    if out.is_empty() {
        out = if all { "players_all.csv".into() } else { "players.csv".into() };
    }
    Ok(Cli { all, one_team, out })
}

fn http_get(host: &str, port: u16, path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect((host, port))?;
    stream.set_read_timeout(Some(Duration::from_secs(15)))?;
    stream.set_write_timeout(Some(Duration::from_secs(15)))?;

    // HTTP/1.0 avoids chunked transfer; server should close connection at end.
    let req = format!(
        "GET {} HTTP/1.0\r\nHost: {}\r\nUser-Agent: ims-bb-scraper/0.1\r\nConnection: close\r\n\r\n",
        path, host
    );
    stream.write_all(req.as_bytes())?;
    stream.flush()?;

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;
    let resp = String::from_utf8_lossy(&buf);

    // Split headers/body
    if let Some(idx) = resp.find("\r\n\r\n") {
        let status_line_end = resp.find("\r\n").unwrap_or(0);
        let status = &resp[..status_line_end];
        if !status.contains("200") {
            return Err(format!("HTTP error: {}", status).into());
        }
        Ok(resp[idx + 4..].to_string())
    } else {
        Err("Malformed HTTP response".into())
    }
}

// Case-insensitive find of an opening tag + slice until end tag (first occurrence).
fn slice_between_ci<'a>(s: &'a str, open_pat: &str, close_pat: &str) -> Option<&'a str> {
    let lc = to_lowercase_fast(s);
    let open_lc = to_lowercase_fast(open_pat);
    let close_lc = to_lowercase_fast(close_pat);

    let open_idx = lc.find(&open_lc)?;
    let after_open = s[open_idx..].find('>')? + open_idx + 1;
    let close_idx_rel = lc[after_open..].find(&close_lc)?;
    Some(&s[after_open..after_open + close_idx_rel])
}

// Find next tag block like <open ...> ... </close>, case-insensitive.
fn next_tag_block_ci(s: &str, open_tag: &str, close_tag: &str, from: usize) -> Option<(usize, usize)> {
    let lc = to_lowercase_fast(s);
    let open_lc = to_lowercase_fast(open_tag);
    let close_lc = to_lowercase_fast(close_tag);

    let start = lc[from..].find(&open_lc)? + from;
    let open_end = s[start..].find('>')? + start + 1;
    let end_rel = lc[open_end..].find(&close_lc)?;
    let end = open_end + end_rel + close_tag.len();
    Some((start, end))
}

// Given "<td ...>INNER</td>", return "INNER" (naive).
fn inner_after_open_tag(td_block: &str) -> String {
    if let Some(open_end) = td_block.find('>') {
        if let Some(close_start) = td_block.rfind('<') {
            if close_start > open_end {
                return td_block[open_end + 1..close_start].to_string();
            }
        }
    }
    String::new()
}

// Strip all tags: remove <...> segments; collapse whitespace.
fn strip_tags(mut s: String) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.drain(..) {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    normalize_ws(&out)
}

fn normalize_entities(s: &str) -> String {
    // Minimal: handle &nbsp; and &amp; (add more if needed)
    s.replace("&nbsp;", " ").replace("&amp;", "&")
}

fn normalize_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    out.trim().to_string()
}

fn to_lowercase_fast(s: &str) -> String {
    // ASCII-ish lowercasing is sufficient for tags/attributes.
    s.chars().map(|c| if c.is_ascii() { c.to_ascii_lowercase() } else { c }).collect()
}

fn extract_team_name(table_inner: &str) -> Option<String> {
    if let Some((tr_s, tr_e)) = next_tag_block_ci(table_inner, "<tr", "</tr>", 0) {
        let tr = &table_inner[tr_s..tr_e];
        if let Some((td_s, td_e)) = next_tag_block_ci(tr, "<td", "</td>", 0) {
            let td = &tr[td_s..td_e];
            let txt = strip_tags(normalize_entities(&inner_after_open_tag(td)));
            if let Some(i) = txt.find(" Team owner") {
                return Some(txt[..i].trim().to_string());
            }
            if let Some(i) = txt.find(" | ") {
                return Some(txt[..i].trim().to_string());
            }
            let t = txt.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    None
}

fn split_first_cell(fused: &str) -> (String, String, String) {
    // Split on first '#', then number is the next token, race is the remainder.
    if let Some(hidx) = fused.find('#') {
        let name = fused[..hidx].trim().to_string();
        let rest = fused[hidx..].trim(); // starts with '#'
        let mut parts = rest.splitn(2, ' ');
        let num = parts.next().unwrap_or("").to_string(); // includes '#'
        let race = parts.next().unwrap_or("").trim().to_string();
        (name, num, race)
    } else {
        (fused.trim().to_string(), String::new(), String::new())
    }
}

fn write_csv_row(out: &mut BufWriter<File>, fields: &[String]) -> std::io::Result<()> {
    let mut first = true;
    for field in fields {
        if !first {
            write!(out, ",")?;
        }
        let needs_quote = field.contains(',') || field.contains('"') || field.contains('\n');
        if needs_quote {
            let escaped = field.replace('"', "\"\"");
            write!(out, "\"{}\"", escaped)?;
        } else {
            write!(out, "{}", field)?;
        }
        first = false;
    }
    writeln!(out)?;
    Ok(())
}
