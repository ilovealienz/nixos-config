//! Talking to sway directly (its IPC socket), for repeat: the open
//! windows, and focusing one. In place of swaymsg and jq.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

const RUN_COMMAND: u32 = 0;
const GET_TREE: u32 = 4;

/// A JSON value, as much as reading sway's replies needs.
#[derive(Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(String),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// what jq counts as true: anything but null and false (and, here, a
    /// missing key)
    pub fn truthy(v: Option<&Json>) -> bool {
        !matches!(v, None | Some(Json::Null) | Some(Json::Bool(false)))
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    /// This value and every value inside it, objects' fields in order
    /// (jq's `..`).
    pub fn each(&self, f: &mut impl FnMut(&Json)) {
        f(self);
        match self {
            Json::Arr(items) => items.iter().for_each(|v| v.each(f)),
            Json::Obj(fields) => fields.iter().for_each(|(_, v)| v.each(f)),
            _ => {}
        }
    }
}

pub fn parse(s: &str) -> Option<Json> {
    let mut p = Parser { b: s.as_bytes(), i: 0 };
    let v = p.value(0)?;
    p.ws();
    (p.i == p.b.len()).then_some(v)
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }

    fn eat(&mut self, word: &str) -> bool {
        if self.b[self.i..].starts_with(word.as_bytes()) {
            self.i += word.len();
            true
        } else {
            false
        }
    }

    fn value(&mut self, depth: usize) -> Option<Json> {
        if depth > 256 {
            return None;
        }
        self.ws();
        match *self.b.get(self.i)? {
            b'{' => {
                self.i += 1;
                let mut fields = Vec::new();
                self.ws();
                if self.eat("}") {
                    return Some(Json::Obj(fields));
                }
                loop {
                    self.ws();
                    let k = self.string()?;
                    self.ws();
                    self.eat(":").then_some(())?;
                    let v = self.value(depth + 1)?;
                    fields.push((k, v));
                    self.ws();
                    if self.eat(",") {
                        continue;
                    }
                    self.eat("}").then_some(())?;
                    return Some(Json::Obj(fields));
                }
            }
            b'[' => {
                self.i += 1;
                let mut items = Vec::new();
                self.ws();
                if self.eat("]") {
                    return Some(Json::Arr(items));
                }
                loop {
                    items.push(self.value(depth + 1)?);
                    self.ws();
                    if self.eat(",") {
                        continue;
                    }
                    self.eat("]").then_some(())?;
                    return Some(Json::Arr(items));
                }
            }
            b'"' => self.string().map(Json::Str),
            b't' => self.eat("true").then_some(Json::Bool(true)),
            b'f' => self.eat("false").then_some(Json::Bool(false)),
            b'n' => self.eat("null").then_some(Json::Null),
            _ => {
                let start = self.i;
                while self.i < self.b.len() && matches!(self.b[self.i], b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E') {
                    self.i += 1;
                }
                (self.i > start).then(|| Json::Num(String::from_utf8_lossy(&self.b[start..self.i]).into_owned()))
            }
        }
    }

    fn hex4(&mut self) -> Option<u32> {
        let h = std::str::from_utf8(self.b.get(self.i..self.i + 4)?).ok()?;
        self.i += 4;
        u32::from_str_radix(h, 16).ok()
    }

    fn string(&mut self) -> Option<String> {
        self.eat("\"").then_some(())?;
        let mut out = Vec::new();
        loop {
            let c = *self.b.get(self.i)?;
            self.i += 1;
            match c {
                b'"' => return String::from_utf8(out).ok(),
                b'\\' => {
                    let e = *self.b.get(self.i)?;
                    self.i += 1;
                    let ch = match e {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => {
                            let hi = self.hex4()?;
                            let code = if (0xd800..0xdc00).contains(&hi) && self.eat("\\u") {
                                let lo = self.hex4()?;
                                0x10000 + ((hi - 0xd800) << 10) + (lo.wrapping_sub(0xdc00) & 0x3ff)
                            } else {
                                hi
                            };
                            char::from_u32(code).unwrap_or('\u{fffd}')
                        }
                        _ => return None,
                    };
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                }
                c => out.push(c),
            }
        }
    }
}

/// sway's socket: $SWAYSOCK (or $I3SOCK), else the one in
/// $XDG_RUNTIME_DIR
fn socket() -> Option<PathBuf> {
    for var in ["SWAYSOCK", "I3SOCK"] {
        if let Some(p) = std::env::var_os(var).filter(|v| !v.is_empty()) {
            return Some(p.into());
        }
    }
    let run = std::env::var_os("XDG_RUNTIME_DIR")?;
    let mut socks: Vec<PathBuf> = std::fs::read_dir(run)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("sway-ipc.") && n.ends_with(".sock")))
        .collect();
    socks.sort();
    socks.into_iter().next()
}

/// One request; sway's reply (None if sway can't be reached).
fn request(kind: u32, payload: &str) -> Option<String> {
    let mut s = UnixStream::connect(socket()?).ok()?;
    s.set_read_timeout(Some(Duration::from_secs(10))).ok()?;
    let mut msg = b"i3-ipc".to_vec();
    msg.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    msg.extend_from_slice(&kind.to_le_bytes());
    msg.extend_from_slice(payload.as_bytes());
    s.write_all(&msg).ok()?;
    let mut head = [0u8; 14];
    s.read_exact(&mut head).ok()?;
    (&head[..6] == b"i3-ipc").then_some(())?;
    let len = u32::from_le_bytes(head[6..10].try_into().ok()?) as usize;
    let mut body = vec![0u8; len];
    s.read_exact(&mut body).ok()?;
    String::from_utf8(body).ok()
}

pub fn tree() -> Option<Json> {
    parse(&request(GET_TREE, "")?)
}

/// Runs sway commands; true if every one of them worked (swaymsg's exit
/// code).
pub fn run(command: &str) -> bool {
    match request(RUN_COMMAND, command).and_then(|r| parse(&r)) {
        Some(Json::Arr(results)) => !results.is_empty() && results.iter().all(|r| matches!(r.get("success"), Some(Json::Bool(true)))),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json() {
        let v = parse(r#" {"a": [1, -2.5e3, true, false, null], "b": "x\"\\\/\n\u00e9\ud83d\ude00", "c": {}} "#).unwrap();
        assert_eq!(v.get("b").and_then(Json::as_str), Some("x\"\\/\né😀"));
        assert_eq!(v.get("a"), Some(&Json::Arr(vec![Json::Num("1".into()), Json::Num("-2.5e3".into()), Json::Bool(true), Json::Bool(false), Json::Null])));
        assert_eq!(v.get("c"), Some(&Json::Obj(vec![])));
        assert!(Json::truthy(v.get("c")) && !Json::truthy(v.get("zz")) && !Json::truthy(Some(&Json::Null)));
        assert!(Json::truthy(Some(&Json::Str(String::new()))) && Json::truthy(Some(&Json::Num("0".into()))));
        let mut n = 0;
        v.each(&mut |_| n += 1);
        assert_eq!(n, 9);
        for bad in ["", "{", "[1,]", "{\"a\" 1}", "\"abc", "tru", "[1] x", "\"\\q\""] {
            assert_eq!(parse(bad), None, "{bad:?}");
        }
    }
}
