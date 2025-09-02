// src/core/sanitize.rs

pub fn normalize_entities(s: &str) -> String {
    s.replace("&nbsp;", " ").replace("&amp;", "&")
}
pub fn normalize_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !prev_space { out.push(' '); prev_space = true; }
        } else { out.push(ch); prev_space = false; }
    }
    out.trim().to_string()
}
pub fn sanitize_team_filename(name: &str, id: u32) -> String {
    let mut out = String::with_capacity(name.len());
    let mut last_us = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() { out.push(ch); last_us = false; }
        else if ch.is_whitespace() { if !last_us { out.push('_'); last_us = true; } }
        else if ch=='-' || ch=='_' { if !(last_us && ch=='_') { out.push(ch); } last_us = ch=='_'; }
    }
    let out = out.trim_matches('_').to_string();
    if out.is_empty() { format!("team_{}", id) } else { out }
}

/// Remove any `[ ... ]` bracket tags (e.g. `[CAPTAIN]`, `[unavailable…]`).
/// Greedy within each bracket pair, no nesting.
pub fn strip_brackets(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_bracket = false;
    for ch in s.chars() {
        match ch {
            '[' => in_bracket = true,
            ']' => in_bracket = false,
            _ if !in_bracket => out.push(ch),
            _ => {}
        }
    }
    out.trim().to_string()
}

/// Keep only letters and spaces up to the first non-letter.
/// If the non-letter is preceded by a space, drop that trailing space too.
pub fn letters_only_trim(s: &str) -> String {
    let s = normalize_ws(s);
    for (i, ch) in s.char_indices() {
        if !(ch.is_alphabetic() || ch.is_whitespace()) {
            let cut = s[..i].trim_end();
            return cut.to_string();
        }
    }
    s.trim().to_string()
}
