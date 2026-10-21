//! Colours: the desktop theme's palette (a flat JSON object of hex
//! strings, from theme.nix), or the terminal's own colours without one.

use ratatui::style::Color;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Palette {
    pub fg: Color,
    pub bright: Color,
    pub muted: Color,
    pub surface: Color,
    pub accent: Color,
    pub on_accent: Color,
    pub red: Color,
    pub orange: Color,
    pub yellow: Color,
    pub green: Color,
    pub blue: Color,
    pub magenta: Color,
}

impl Default for Palette {
    fn default() -> Self {
        Palette {
            fg: Color::Reset,
            bright: Color::White,
            muted: Color::DarkGray,
            surface: Color::DarkGray,
            accent: Color::Yellow,
            on_accent: Color::Black,
            red: Color::Red,
            orange: Color::LightRed,
            yellow: Color::Yellow,
            green: Color::Green,
            blue: Color::Blue,
            magenta: Color::Magenta,
        }
    }
}

impl Palette {
    pub fn from_json(text: &str) -> Palette {
        let map = parse_flat_json(text);
        let d = Palette::default();
        let get = |k: &str, fallback: Color| map.get(k).and_then(|v| hex(v)).unwrap_or(fallback);
        Palette {
            fg: get("fg", d.fg),
            bright: get("fgBright", d.bright),
            muted: get("muted", d.muted),
            surface: get("surface", d.surface),
            accent: get("accent", d.accent),
            on_accent: get("onAccent", d.on_accent),
            red: get("red", d.red),
            orange: get("orange", d.orange),
            yellow: get("yellow", d.yellow),
            green: get("green", d.green),
            blue: get("blue", d.blue),
            magenta: get("magenta", d.magenta),
        }
    }

    /// colours for the top-level folders, in turn
    pub fn folder_colours(&self) -> [Color; 6] {
        [
            self.blue,
            self.green,
            self.yellow,
            self.magenta,
            self.orange,
            self.red,
        ]
    }
}

pub fn hex(s: &str) -> Option<Color> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let ch = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).ok();
    Some(Color::Rgb(ch(0)?, ch(2)?, ch(4)?))
}

/// `{"a": "b", ...}`: the string values; other values are skipped
pub fn parse_flat_json(text: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let mut chars = text.chars().peekable();
    let read_string = |chars: &mut std::iter::Peekable<std::str::Chars>| {
        let mut s = String::new();
        while let Some(c) = chars.next() {
            match c {
                '"' => break,
                '\\' => {
                    if let Some(n) = chars.next() {
                        s.push(n);
                    }
                }
                _ => s.push(c),
            }
        }
        s
    };
    while let Some(c) = chars.next() {
        if c != '"' {
            continue;
        }
        let key = read_string(&mut chars);
        while chars.peek().is_some_and(|c| c.is_whitespace() || *c == ':') {
            chars.next();
        }
        if chars.peek() == Some(&'"') {
            chars.next();
            out.insert(key, read_string(&mut chars));
        } else {
            // not a string: skip to the next entry
            while chars.peek().is_some_and(|c| *c != ',' && *c != '}') {
                chars.next();
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_palette() {
        let p = Palette::from_json(r##"{ "accent": "c86f42", "fg": "#d4a97b", "nope": "zz" }"##);
        assert_eq!(p.accent, Color::Rgb(0xc8, 0x6f, 0x42));
        assert_eq!(p.fg, Color::Rgb(0xd4, 0xa9, 0x7b));
        assert_eq!(p.red, Color::Red);
        assert_eq!(Palette::from_json("").blue, Color::Blue);
    }

    #[test]
    fn skips_other_values() {
        let m = parse_flat_json(r#"{"n": 5, "a": "x\"y", "b": true, "c": "z"}"#);
        assert_eq!(m.get("a").map(String::as_str), Some("x\"y"));
        assert_eq!(m.get("c").map(String::as_str), Some("z"));
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn hex_rejects_junk() {
        assert_eq!(hex("12345"), None);
        assert_eq!(hex("12345g"), None);
        assert_eq!(hex("000000"), Some(Color::Rgb(0, 0, 0)));
    }
}
