// /src/html.rs

// Case-insensitive slice between an opening tag (with attributes) and its closing tag.
// Example: slice_between_ci(html, "<table class=teamroster", "</table>")
pub fn slice_between_ci<'a>(s: &'a str, open_pat: &str, close_pat: &str) -> Option<&'a str> {
    let lc = to_lowercase_fast(s);
    let open_lc = to_lowercase_fast(open_pat);
    let close_lc = to_lowercase_fast(close_pat);

    let open_idx = lc.find(&open_lc)?;
    let after_open = s[open_idx..].find('>')? + open_idx + 1;
    let close_idx_rel = lc[after_open..].find(&close_lc)?;
    Some(&s[after_open..after_open + close_idx_rel])
}

// Find next tag block like <open ...> ... </close>, case-insensitive.
pub fn next_tag_block_ci(s: &str, open_tag: &str, close_tag: &str, from: usize) -> Option<(usize, usize)> {
    let lc = to_lowercase_fast(s);
    let open_lc = to_lowercase_fast(open_tag);
    let close_lc = to_lowercase_fast(close_tag);

    let start = lc.get(from..)?.find(&open_lc)? + from;
    let open_end = s[start..].find('>')? + start + 1;
    let end_rel = lc[open_end..].find(&close_lc)?;
    let end = open_end + end_rel + close_tag.len();
    Some((start, end))
}

// Given "<td ...>INNER</td>", return "INNER".
pub fn inner_after_open_tag(block: &str) -> String {
    if let Some(open_end) = block.find('>') {
        if let Some(close_start) = block.rfind('<') {
            if close_start > open_end {
                return block[open_end + 1..close_start].to_string();
            }
        }
    }
    String::new()
}

// Remove "<...>" segments, then collapse whitespace.
pub fn strip_tags(s: String) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    normalize_ws(&out)
}

pub fn normalize_entities(s: &str) -> String {
    s.replace("&nbsp;", " ").replace("&amp;", "&")
}

pub fn normalize_ws(s: &str) -> String {
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

// ASCII-fast lowercasing sufficient for tags/attrs.
pub fn to_lowercase_fast(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii() { c.to_ascii_lowercase() } else { c })
        .collect()
}
