// src/core/vischars.rs
// Fast visible-text character iterator for a single HTML line.
// Skips tags (<...>), skips entities (&...;), collapses ASCII whitespace to a single ' '.

pub struct VisChars<'a> {
    s: &'a str,
    b: &'a [u8],
    i: usize,
    n: usize,
}

impl<'a> VisChars<'a> {
    pub fn new(s: &'a str) -> Self { Self { s, b: s.as_bytes(), i: 0, n: s.len() } }

    #[inline]
    fn skip_tag(&mut self) {
        // called when current byte is '<'
        self.i += 1;
        let mut in_s = false; // '
        let mut in_d = false; // "
        while self.i < self.n {
            match self.b[self.i] {
                b'\'' if !in_d => in_s = !in_s,
                b'"'  if !in_s => in_d = !in_d,
                b'>' if !in_s && !in_d => { self.i += 1; break; }
                _ => {}
            }
            self.i += 1;
        }
    }

    #[inline]
    fn skip_entity(&mut self) {
        // called when current byte is '&'
        self.i += 1;
        while self.i < self.n {
            if self.b[self.i] == b';' { self.i += 1; break; }
            self.i += 1;
        }
    }

    #[inline]
    fn next_char(&mut self) -> Option<char> {
        if self.i >= self.n { return None; }
        let c = self.b[self.i];
        if c < 0x80 { self.i += 1; Some(c as char) }
        else {
            let ch = self.s[self.i..].chars().next().unwrap();
            self.i += ch.len_utf8();
            Some(ch)
        }
    }
}

impl<'a> Iterator for VisChars<'a> {
    type Item = char;
    fn next(&mut self) -> Option<Self::Item> {
        while self.i < self.n {
            match self.b[self.i] {
                b'<' => { self.skip_tag(); continue; }
                b'&' => { self.skip_entity(); return Some(' '); }
                b' ' | b'\t' | b'\r' | b'\n' => {
                    // collapse consecutive whitespace to a single space
                    while self.i < self.n {
                        match self.b[self.i] { b' ' | b'\t' | b'\r' | b'\n' => self.i += 1, _ => break }
                    }
                    return Some(' ');
                }
                _ => return self.next_char(),
            }
        }
        None
    }
}

