//! A small JSON reader (from sway-helpers); unneeded fields are skipped
//! without copying.

pub struct Parser<'a> {
    b: &'a [u8],
    i: usize,
    depth: usize,
}

/// sway's trees are a few levels deep; anything past this is not sway
const MAX_DEPTH: usize = 128;

impl<'a> Parser<'a> {
    pub fn new(b: &'a [u8]) -> Self {
        Parser { b, i: 0, depth: 0 }
    }

    fn ws(&mut self) {
        while self.i < self.b.len() && self.b[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.ws();
        self.b.get(self.i).copied()
    }

    fn eat(&mut self, c: u8) -> Option<()> {
        if self.peek()? == c {
            self.i += 1;
            Some(())
        } else {
            None
        }
    }

    fn word(&mut self, w: &[u8]) -> Option<()> {
        if self.b.get(self.i..)?.starts_with(w) {
            self.i += w.len();
            Some(())
        } else {
            None
        }
    }

    fn enter(&mut self) -> Option<()> {
        self.depth += 1;
        (self.depth <= MAX_DEPTH).then_some(())
    }

    /// Calls `field` for each key of an object. `field` must read or skip
    /// the value. Keys are given as raw bytes (sway's keys have no escapes).
    pub fn object(&mut self, mut field: impl FnMut(&mut Self, &[u8]) -> Option<()>) -> Option<()> {
        self.eat(b'{')?;
        self.enter()?;
        if self.peek()? == b'}' {
            self.i += 1;
            self.depth -= 1;
            return Some(());
        }
        loop {
            self.ws();
            let key = self.raw_string()?;
            self.eat(b':')?;
            field(self, key)?;
            match self.peek()? {
                b',' => self.i += 1,
                b'}' => {
                    self.i += 1;
                    self.depth -= 1;
                    return Some(());
                }
                _ => return None,
            }
        }
    }

    /// Calls `item` for each element of an array (or reads null as empty).
    pub fn array(&mut self, mut item: impl FnMut(&mut Self) -> Option<()>) -> Option<()> {
        if self.peek()? == b'n' {
            return self.word(b"null");
        }
        self.eat(b'[')?;
        self.enter()?;
        if self.peek()? == b']' {
            self.i += 1;
            self.depth -= 1;
            return Some(());
        }
        loop {
            item(self)?;
            match self.peek()? {
                b',' => self.i += 1,
                b']' => {
                    self.i += 1;
                    self.depth -= 1;
                    return Some(());
                }
                _ => return None,
            }
        }
    }

    /// Skips any value without keeping it.
    pub fn skip(&mut self) -> Option<()> {
        match self.peek()? {
            b'{' => self.object(|p, _| p.skip()),
            b'[' => self.array(|p| p.skip()),
            b'"' => self.raw_string().map(|_| ()),
            b't' => self.word(b"true"),
            b'f' => self.word(b"false"),
            b'n' => self.word(b"null"),
            _ => self.number().map(|_| ()),
        }
    }

    /// true/false; null reads as false
    pub fn bool(&mut self) -> Option<bool> {
        match self.peek()? {
            b't' => self.word(b"true").map(|_| true),
            b'f' => self.word(b"false").map(|_| false),
            b'n' => self.word(b"null").map(|_| false),
            _ => None,
        }
    }

    /// An integer; fractions keep their whole part, null reads as 0.
    fn number(&mut self) -> Option<i64> {
        self.ws();
        let start = self.i;
        while let Some(&c) = self.b.get(self.i) {
            if c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.' | b'e' | b'E') {
                self.i += 1;
            } else {
                break;
            }
        }
        if start == self.i {
            return None;
        }
        let text = std::str::from_utf8(&self.b[start..self.i]).ok()?;
        let whole = text.split(['.', 'e', 'E']).next()?;
        Some(whole.parse().unwrap_or(0))
    }

    /// The bytes between the quotes, escapes left as they are.
    fn raw_string(&mut self) -> Option<&'a [u8]> {
        self.eat(b'"')?;
        let start = self.i;
        loop {
            match *self.b.get(self.i)? {
                b'"' => {
                    self.i += 1;
                    return Some(&self.b[start..self.i - 1]);
                }
                b'\\' => self.i += 2,
                _ => self.i += 1,
            }
        }
    }

    /// A string with its escapes decoded; null reads as "".
    pub fn string(&mut self) -> Option<String> {
        if self.peek()? == b'n' {
            return self.word(b"null").map(|_| String::new());
        }
        let raw = self.raw_string()?;
        if !raw.contains(&b'\\') {
            return String::from_utf8(raw.to_vec()).ok();
        }
        let mut out = Vec::with_capacity(raw.len());
        let mut i = 0;
        while i < raw.len() {
            let c = raw[i];
            i += 1;
            if c != b'\\' {
                out.push(c);
                continue;
            }
            let e = *raw.get(i)?;
            i += 1;
            match e {
                b'n' => out.push(b'\n'),
                b't' => out.push(b'\t'),
                b'r' => out.push(b'\r'),
                b'b' => out.push(8),
                b'f' => out.push(12),
                b'u' => {
                    let hex = |at: usize| -> Option<u32> {
                        u32::from_str_radix(std::str::from_utf8(raw.get(at..at + 4)?).ok()?, 16).ok()
                    };
                    let mut cp = hex(i)?;
                    i += 4;
                    // surrogate pair (emoji and the like in window titles)
                    if (0xD800..0xDC00).contains(&cp) && raw.get(i..i + 2) == Some(b"\\u") {
                        if let Some(lo) = hex(i + 2).filter(|lo| (0xDC00..0xE000).contains(lo)) {
                            cp = 0x10000 + ((cp - 0xD800) << 10) + (lo - 0xDC00);
                            i += 6;
                        }
                    }
                    let ch = char::from_u32(cp).unwrap_or('\u{FFFD}');
                    out.extend_from_slice(ch.encode_utf8(&mut [0; 4]).as_bytes());
                }
                other => out.push(other), // \" \\ \/
            }
        }
        String::from_utf8(out).ok()
    }
}
