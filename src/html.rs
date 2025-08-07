// /src/html.rs
// Low-level HTML string manipulation helpers.
// These are deliberately naive but tailored to the Brutalball site structure.
// They operate case-insensitively on ASCII tag/attribute names.

/// Find the section between an opening tag (with attributes) and its matching closing tag,
/// case-insensitive on the tag name and attributes.
/// Returns the HTML *inside* the opening/closing tags.
///
/// Example:
/// ```
/// let table_inner = slice_between_ci(html, "<table class=teamroster", "</table>");
/// ```
pub fn slice_between_ci<'a>(s: &'a str, open_pat: &str, close_pat: &str) -> Option<&'a str> {
    let lc = to_lowercase_fast(s);
    let open_lc = to_lowercase_fast(open_pat);
    let close_lc = to_lowercase_fast(close_pat);

    // Find the opening tag (e.g., <table class=teamroster>)
    let open_idx = lc.find(&open_lc)?;
    // Jump past the '>' of the opening tag
    let after_open = s[open_idx..].find('>')? + open_idx + 1;
    // Find the closing tag
    let close_idx_rel = lc[after_open..].find(&close_lc)?;
    Some(&s[after_open..after_open + close_idx_rel])
}

/// Find the next complete tag block from `from` onwards, case-insensitive.
/// A block is from the start of the opening tag to the end of the closing tag.
///
/// Example:
/// `<tr ...> ... </tr>` or `<td ...> ... </td>`
pub fn next_tag_block_ci(s: &str, open_tag: &str, close_tag: &str, from: usize) -> Option<(usize, usize)> {
    let lc = to_lowercase_fast(s);
    let open_lc = to_lowercase_fast(open_tag);
    let close_lc = to_lowercase_fast(close_tag);

    // Locate the opening tag
    let start = lc.get(from..)?.find(&open_lc)? + from;
    // Jump past the end of the opening tag
    let open_end = s[start..].find('>')? + start + 1;
    // Find the closing tag
    let end_rel = lc[open_end..].find(&close_lc)?;
    let end = open_end + end_rel + close_tag.len();
    Some((start, end))
}

/// Given a complete tag block like `<td ...>INNER</td>`,
/// return the INNER text without the wrapping tags (still may contain nested tags).
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

/// Remove all HTML tags `<...>` from the string, then collapse whitespace.
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

/// Minimal HTML entity decoding: handle `&nbsp;` and `&amp;` only.
pub fn normalize_entities(s: &str) -> String {
    s.replace("&nbsp;", " ").replace("&amp;", "&")
}

/// Collapse sequences of whitespace into a single space and trim.
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

/// Fast ASCII-only lowercasing for tag/attribute matching.
pub fn to_lowercase_fast(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii() { c.to_ascii_lowercase() } else { c })
        .collect()
}
