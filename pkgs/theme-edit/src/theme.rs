//! The palettes in theme.nix (read only) and your colours in
//! ~/.config/theme/colors.json (read and written).
//!
//! theme.nix gives the layout: the colour names with their comments, the
//! dark and light defaults, and the worked-out colours
//! (`name = mix c.a c.b 0.67;`). colors.json holds the variant in use and
//! only the colours that differ from the defaults:
//!
//!   { "variant": "light", "dark": { "accent": "4fa3e0" }, "light": {} }

pub type Rgb = [u8; 3];

pub const VARIANTS: [&str; 2] = ["dark", "light"];

pub struct Entry {
    pub name: String,
    pub comment: String,
    /// per variant (0 dark, 1 light): theme.nix's default, the colour
    /// being edited, and the colour in colors.json
    pub default: [Rgb; 2],
    pub color: [Rgb; 2],
    pub saved: [Rgb; 2],
}

pub struct Section {
    pub title: String,
    pub notes: Vec<String>,
    pub entries: Vec<Entry>,
}

pub struct Derived {
    pub name: String,
    pub a: String,
    pub b: String,
    pub t: f64,
    pub comment: String,
}

pub struct Theme {
    pub sections: Vec<Section>,
    pub derived: Vec<Derived>,
    /// 0 dark, 1 light: shown and edited
    pub variant: usize,
    pub saved_variant: usize,
}

pub fn parse_hex(s: &str) -> Option<Rgb> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let ch = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).ok();
    Some([ch(0)?, ch(2)?, ch(4)?])
}

pub fn hex(c: Rgb) -> String {
    format!("{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

/// theme.nix's mix: blend a towards b, t = how much of a
pub fn mix(a: Rgb, b: Rgb, t: f64) -> Rgb {
    let one = |i: usize| (a[i] as f64 * t + b[i] as f64 * (1.0 - t) + 0.5).floor() as u8;
    [one(0), one(1), one(2)]
}

/// WCAG contrast ratio, 1.0 to 21.0
pub fn contrast(a: Rgb, b: Rgb) -> f32 {
    fn lum(c: Rgb) -> f64 {
        let f = |v: u8| {
            let v = v as f64 / 255.0;
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * f(c[0]) + 0.7152 * f(c[1]) + 0.0722 * f(c[2])
    }
    let (x, y) = (lum(a), lum(b));
    let (hi, lo) = if x > y { (x, y) } else { (y, x) };
    ((hi + 0.05) / (lo + 0.05)) as f32
}

/// the lines after the first line starting with `opener` (after
/// `after`), up to its `};`
fn block(lines: &[&str], after: usize, opener: &str) -> Option<(usize, usize)> {
    let start = lines
        .iter()
        .skip(after)
        .position(|l| l.trim_start().starts_with(opener))?
        + after;
    let end = lines[start..].iter().position(|l| l.trim() == "};")? + start;
    Some((start, end))
}

fn is_ident(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// `  name = "e5a440";  # comment` -> (name, colour, comment)
fn entry_line(l: &str) -> Option<(String, Rgb, String)> {
    let (name, rest) = l.split_once('=')?;
    let name = name.trim();
    let value = rest.trim_start().strip_prefix('"')?;
    if !is_ident(name) || value.get(6..8)? != "\";" {
        return None;
    }
    let color = parse_hex(&value[..6])?;
    let comment = value[8..].trim().trim_start_matches('#').trim().to_string();
    Some((name.to_string(), color, comment))
}

/// `  name = mix c.accent c.bg 0.67;  # comment`
fn derived_line(l: &str) -> Option<Derived> {
    let (code, comment) = match l.split_once('#') {
        Some((c, m)) => (c, m.trim().to_string()),
        None => (l, String::new()),
    };
    let (name, rest) = code.split_once('=')?;
    let mut w = rest.trim().trim_end_matches(';').split_whitespace();
    if w.next()? != "mix" {
        return None;
    }
    let a = w.next()?.strip_prefix("c.")?.to_string();
    let b = w.next()?.strip_prefix("c.")?.to_string();
    let t: f64 = w.next()?.parse().ok()?;
    if w.next().is_some() || !is_ident(name.trim()) || !(0.0..=1.0).contains(&t) {
        return None;
    }
    Some(Derived {
        name: name.trim().to_string(),
        a,
        b,
        t,
        comment,
    })
}

/// one palette block: sections (from the comments) of entries
fn palette(lines: &[&str], (start, end): (usize, usize)) -> Result<Vec<Section>, String> {
    let mut sections: Vec<Section> = Vec::new();
    let mut comments: Vec<String> = Vec::new();
    let mut after_blank = true;
    for (i, l) in lines.iter().enumerate().take(end).skip(start + 1) {
        let t = l.trim();
        if t.is_empty() {
            after_blank = true;
            comments.clear();
            continue;
        }
        if let Some(c) = t.strip_prefix('#') {
            comments.push(c.trim().to_string());
            continue;
        }
        let Some((name, color, comment)) = entry_line(l) else {
            return Err(format!(
                "line {}: not a `name = \"rrggbb\";` line:\n{t}",
                i + 1
            ));
        };
        if after_blank || sections.is_empty() {
            let mut c = std::mem::take(&mut comments).into_iter();
            sections.push(Section {
                title: c.next().unwrap_or_default(),
                notes: c.collect(),
                entries: Vec::new(),
            });
        } else if !comments.is_empty() {
            sections.last_mut().unwrap().notes.append(&mut comments);
        }
        after_blank = false;
        let both = [color; 2];
        sections.last_mut().unwrap().entries.push(Entry {
            name,
            comment,
            default: both,
            color: both,
            saved: both,
        });
    }
    Ok(sections)
}

/// theme.nix: the layout and comments from the dark palette, the
/// defaults from both
pub fn parse(text: &str) -> Result<Theme, String> {
    let lines: Vec<&str> = text.split('\n').collect();
    let pal = lines
        .iter()
        .position(|l| l.trim_start().starts_with("palettes = {"))
        .ok_or("no `palettes = { dark = { ... }; light = { ... }; };` found")?;
    let dark = block(&lines, pal, "dark = {").ok_or("no `dark = { ... };` palette")?;
    let light = block(&lines, pal, "light = {").ok_or("no `light = { ... };` palette")?;
    let mut sections = palette(&lines, dark)?;
    let light: Vec<Entry> = palette(&lines, light)?
        .into_iter()
        .flat_map(|s| s.entries)
        .collect();

    let names =
        |it: &mut dyn Iterator<Item = &Entry>| it.map(|e| e.name.clone()).collect::<Vec<_>>();
    let mut dn = names(&mut sections.iter().flat_map(|s| s.entries.iter()));
    let mut ln = names(&mut light.iter());
    dn.sort();
    ln.sort();
    if dn.is_empty() || dn != ln {
        return Err("the dark and light palettes must have the same colours".into());
    }
    for e in sections.iter_mut().flat_map(|s| s.entries.iter_mut()) {
        let l = light.iter().find(|l| l.name == e.name).unwrap().default[0];
        e.default[1] = l;
        e.color[1] = l;
        e.saved[1] = l;
    }

    let mut derived = Vec::new();
    if let Some((ds, de)) = block(&lines, 0, "derived = c: {") {
        derived = lines[ds + 1..de]
            .iter()
            .filter_map(|l| derived_line(l))
            .collect();
    }
    let theme = Theme {
        sections,
        derived,
        variant: 0,
        saved_variant: 0,
    };
    for d in &theme.derived {
        if theme.get(&d.a).is_none() || theme.get(&d.b).is_none() {
            return Err(format!(
                "{} mixes colours that aren't in the palette",
                d.name
            ));
        }
    }
    Ok(theme)
}

impl Theme {
    pub fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.sections.iter().flat_map(|s| s.entries.iter())
    }

    pub fn entries_mut(&mut self) -> impl Iterator<Item = &mut Entry> {
        self.sections.iter_mut().flat_map(|s| s.entries.iter_mut())
    }

    /// a palette or worked-out colour of the variant shown
    pub fn get(&self, name: &str) -> Option<Rgb> {
        if let Some(e) = self.entries().find(|e| e.name == name) {
            return Some(e.color[self.variant]);
        }
        let d = self.derived.iter().find(|d| d.name == name)?;
        Some(mix(self.get(&d.a)?, self.get(&d.b)?, d.t))
    }

    /// the colour, or magenta if theme.nix doesn't have it
    pub fn c(&self, name: &str) -> Rgb {
        self.get(name).unwrap_or([255, 0, 255])
    }

    /// colours (in either variant) changed since loading or saving, plus
    /// one if the variant changed
    pub fn changed(&self) -> usize {
        let colours: usize = self
            .entries()
            .map(|e| (0..2).filter(|&v| e.color[v] != e.saved[v]).count())
            .sum();
        colours + usize::from(self.variant != self.saved_variant)
    }

    pub fn mark_saved(&mut self) {
        for e in self.entries_mut() {
            e.saved = e.color;
        }
        self.saved_variant = self.variant;
    }

    /// apply colors.json (already parsed); unknown names are reported
    pub fn load_local(&mut self, local: &Local) -> Vec<String> {
        let mut unknown = Vec::new();
        if let Some(v) = local.variant {
            self.variant = v;
            self.saved_variant = v;
        }
        for (v, colours) in local.colours.iter().enumerate() {
            for (name, c) in colours {
                match self.entries_mut().find(|e| &e.name == name) {
                    Some(e) => {
                        e.color[v] = *c;
                        e.saved[v] = *c;
                    }
                    None => unknown.push(format!("{}.{name}", VARIANTS[v])),
                }
            }
        }
        unknown
    }

    /// colors.json: the variant, and the colours that differ from theme.nix
    pub fn to_json(&self) -> String {
        let mut out = format!("{{\n  \"variant\": \"{}\"", VARIANTS[self.variant]);
        for (v, name) in VARIANTS.iter().enumerate() {
            let mine: Vec<String> = self
                .entries()
                .filter(|e| e.color[v] != e.default[v])
                .map(|e| format!("    \"{}\": \"{}\"", e.name, hex(e.color[v])))
                .collect();
            if mine.is_empty() {
                out += &format!(",\n  \"{name}\": {{}}");
            } else {
                out += &format!(",\n  \"{name}\": {{\n{}\n  }}", mine.join(",\n"));
            }
        }
        out + "\n}\n"
    }
}

/// colors.json, parsed
#[derive(Default)]
pub struct Local {
    pub variant: Option<usize>,
    pub colours: [Vec<(String, Rgb)>; 2],
}

/// Reads colors.json. Only the shape theme.nix reads is accepted: an
/// object with "variant" (a string) and "dark"/"light" (objects of
/// 6-digit hex strings).
pub fn parse_local(text: &str) -> Result<Local, String> {
    let mut p = Json {
        s: text.as_bytes(),
        i: 0,
    };
    let top = p.object()?;
    p.ws();
    if p.i != p.s.len() {
        return Err("unexpected text after the closing }".into());
    }
    let mut local = Local::default();
    for (key, value) in top {
        match (key.as_str(), value) {
            ("variant", Value::Str(v)) => {
                let i = VARIANTS
                    .iter()
                    .position(|n| *n == v)
                    .ok_or(format!("variant must be dark or light, not {v:?}"))?;
                local.variant = Some(i);
            }
            (k, Value::Obj(colours)) if VARIANTS.contains(&k) => {
                let v = VARIANTS.iter().position(|n| *n == k).unwrap();
                for (name, c) in colours {
                    let Value::Str(c) = c else {
                        return Err(format!("{k}.{name}: not a string"));
                    };
                    let rgb = parse_hex(&c)
                        .filter(|_| c.len() == 6)
                        .ok_or(format!("{k}.{name}: {c:?} isn't 6-digit hex"))?;
                    local.colours[v].push((name, rgb));
                }
            }
            (k, _) => return Err(format!("unexpected {k:?}")),
        }
    }
    Ok(local)
}

enum Value {
    Str(String),
    Obj(Vec<(String, Value)>),
}

struct Json<'a> {
    s: &'a [u8],
    i: usize,
}

impl Json<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn eat(&mut self, c: u8) -> Result<(), String> {
        self.ws();
        if self.s.get(self.i) == Some(&c) {
            self.i += 1;
            Ok(())
        } else {
            Err(format!("expected '{}' at byte {}", c as char, self.i))
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.eat(b'"')?;
        let start = self.i;
        while self.i < self.s.len() && self.s[self.i] != b'"' {
            if self.s[self.i] == b'\\' {
                return Err("escapes aren't supported".into());
            }
            self.i += 1;
        }
        let out = std::str::from_utf8(&self.s[start..self.i])
            .map_err(|e| e.to_string())?
            .to_string();
        self.eat(b'"')?;
        Ok(out)
    }

    fn value(&mut self) -> Result<Value, String> {
        self.ws();
        match self.s.get(self.i) {
            Some(b'"') => Ok(Value::Str(self.string()?)),
            Some(b'{') => Ok(Value::Obj(self.object()?)),
            _ => Err(format!("expected a string or an object at byte {}", self.i)),
        }
    }

    fn object(&mut self) -> Result<Vec<(String, Value)>, String> {
        self.eat(b'{')?;
        let mut out = Vec::new();
        self.ws();
        if self.s.get(self.i) == Some(&b'}') {
            self.i += 1;
            return Ok(out);
        }
        loop {
            let k = self.string()?;
            self.eat(b':')?;
            out.push((k, self.value()?));
            self.ws();
            match self.s.get(self.i) {
                Some(b',') => self.i += 1,
                Some(b'}') => {
                    self.i += 1;
                    return Ok(out);
                }
                _ => return Err(format!("expected ',' or '}}' at byte {}", self.i)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = include_str!("../testdata/theme.nix");

    #[test]
    fn reads_theme_nix() {
        let t = parse(FILE).unwrap();
        let names: Vec<_> = t.entries().map(|e| e.name.as_str()).collect();
        assert_eq!(names.len(), 16);
        assert_eq!(names[0], "bgDark");
        assert_eq!(t.c("accent"), [0xe5, 0xa4, 0x40]);
        let acc = t.entries().find(|e| e.name == "accent").unwrap();
        assert_eq!(hex(acc.default[1]), "9c5a08");
        assert_eq!(
            t.sections[0].title,
            "backgrounds: change these four together to shift the tint"
        );
        let s = t
            .sections
            .iter()
            .find(|s| s.entries.iter().any(|e| e.name == "accent"))
            .unwrap();
        assert_eq!(s.notes, ["original amber: e5a440"]);
        assert_eq!(t.derived.len(), 2);
    }

    #[test]
    fn derived_matches_nix() {
        // from nix-instantiate --eval of theme.nix
        let mut t = parse(FILE).unwrap();
        assert_eq!(hex(t.c("selectionBg")), "a57934");
        assert_eq!(hex(t.c("accentDim")), "4e3f24");
        t.load_local(&parse_local(r#"{"variant":"light","light":{"accent":"115599"}}"#).unwrap());
        assert_eq!(hex(t.c("bg")), "f4eee0");
        assert_eq!(hex(t.c("selectionBg")), "5c87b0");
    }

    #[test]
    fn colors_json_round_trip() {
        let mut t = parse(FILE).unwrap();
        assert_eq!(
            t.to_json(),
            "{\n  \"variant\": \"dark\",\n  \"dark\": {},\n  \"light\": {}\n}\n"
        );
        let unknown = t.load_local(
            &parse_local(
                r#" { "variant": "light", "dark": { "accent": "4FA3E0", "nope": "123456" } } "#,
            )
            .unwrap(),
        );
        assert_eq!(unknown, ["dark.nope"]);
        assert_eq!((t.variant, t.changed()), (1, 0));
        for e in t.entries_mut().filter(|e| e.name == "red") {
            e.color[1] = [1, 2, 3];
        }
        assert_eq!(t.changed(), 1);
        let json = t.to_json();
        assert_eq!(
            json,
            "{\n  \"variant\": \"light\",\n  \"dark\": {\n    \"accent\": \"4fa3e0\"\n  },\n  \"light\": {\n    \"red\": \"010203\"\n  }\n}\n"
        );
        let mut again = parse(FILE).unwrap();
        again.load_local(&parse_local(&json).unwrap());
        assert_eq!(again.to_json(), json);
    }

    #[test]
    fn bad_colors_json() {
        for bad in [
            r#"{"variant":"blue"}"#,
            r#"{"dark":{"accent":"zzz"}}"#,
            r##"{"dark":{"accent":"#e5a440"}}"##,
            r#"{"dark":{"accent":1}}"#,
            r#"{"other":{}}"#,
            r#"{"dark":{}} x"#,
            r#"{"dark":{"a":"A"}}"#,
        ] {
            assert!(parse_local(bad).is_err(), "{bad}");
        }
    }
}
