// src/core/html.rs
pub fn to_lower(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii() {
                c.to_ascii_lowercase()
            } else {
                c
            }
        })
        .collect()
}
pub fn slice_between_ci<'a>(s: &'a str, open_pat: &str, close_pat: &str) -> Option<&'a str> {
    let lc = to_lower(s);
    let open = to_lower(open_pat);
    let close = to_lower(close_pat);
    let o = lc.find(&open)?;
    let after = s[o..].find('>')? + o + 1;
    let cr = lc[after..].find(&close)?;
    Some(&s[after..after + cr])
}
pub fn next_tag_block_ci(s: &str, o: &str, c: &str, from: usize) -> Option<(usize, usize)> {
    let lc = to_lower(s);
    let ol = to_lower(o);
    let cl = to_lower(c);
    let start = lc.get(from..)?.find(&ol)? + from;
    let open_end = s[start..].find('>')? + start + 1;
    let end_rel = lc[open_end..].find(&cl)?;
    let end = open_end + end_rel + c.len();
    Some((start, end))
}
pub fn inner_after_open_tag(block: &str) -> String {
    if let Some(oe) = block.find('>') {
        if let Some(cs) = block.rfind('<') {
            if cs > oe {
                return block[oe + 1..cs].to_string();
            }
        }
    }
    s!()
}
pub fn strip_tags<S: AsRef<str>>(s: S) -> String {
    let s = s.as_ref();

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
    super::sanitize::normalize_ws(&out)
}
