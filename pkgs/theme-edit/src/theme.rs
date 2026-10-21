//! theme.nix (read only: palettes, derived colours, iconSets, presets) and
//! colors.json (read and written: variant, changed colours, icons,
//! wallpaper; unknown keys kept):
//!
//!   { "variant": "light", "dark": { "accent": "4fa3e0" }, "light": {},
//!     "icons": "papirus",
//!     "wallpaper": { "from": "/path/to/the/picked.png", "mode": "fill" } }

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

/// an icon set from theme.nix's iconSets
pub struct IconSet {
    pub key: String,
    /// shown in the picker: the comment after it, or the key
    pub label: String,
    pub package: String,
    /// packages it borrows icons from
    pub also: Vec<String>,
    /// the theme's name for dark and light
    pub names: [String; 2],
    /// its web page (https), to see what it looks like
    pub page: Option<String>,
}

/// a colour preset: every colour, dark and light
pub struct Preset {
    pub key: String,
    /// shown in the picker: the comment after it, or the key
    pub label: String,
    pub colours: [Vec<(String, Rgb)>; 2],
}

/// how sway fits the wallpaper to a screen (sway-output(5))
pub const MODES: [&str; 5] = ["fill", "fit", "center", "tile", "stretch"];

/// the wallpaper picked: the image it was copied from, and its mode
#[derive(Clone, PartialEq, Debug)]
pub struct Wallpaper {
    pub from: String,
    pub mode: String,
}

pub struct Theme {
    pub sections: Vec<Section>,
    pub derived: Vec<Derived>,
    /// 0 dark, 1 light: shown and edited
    pub variant: usize,
    pub saved_variant: usize,
    pub icon_sets: Vec<IconSet>,
    /// the set used when colors.json doesn't pick one
    pub icon_default: String,
    /// the set picked, and the one saved
    pub icons: String,
    pub saved_icons: String,
    /// the wallpaper picked (None: the config's default), and the saved one
    pub wallpaper: Option<Wallpaper>,
    pub saved_wallpaper: Option<Wallpaper>,
    /// folders added to the wallpaper picker, and the saved list
    pub wall_folders: Vec<String>,
    pub saved_wall_folders: Vec<String>,
    /// keys of colors.json this doesn't know, kept as JSON text
    pub extra: Vec<(String, String)>,
    /// the defaults first, then theme.nix's presets
    pub presets: Vec<Preset>,
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

/// `  key = { package = "p"; also = "a b"; dark = "D"; light = "L"; page = "https://…"; };  # Label`
/// (also and page are optional)
fn icon_line(l: &str) -> Option<IconSet> {
    let (code, label) = match l.split_once('#') {
        Some((c, m)) => (c, m.trim()),
        None => (l, ""),
    };
    let (key, rest) = code.split_once('=')?;
    let key = key.trim();
    let body = rest
        .trim()
        .strip_prefix('{')?
        .strip_suffix(';')?
        .trim_end()
        .strip_suffix('}')?;
    let (mut package, mut also, mut dark, mut light, mut page) = (None, None, None, None, None);
    for field in body.split(';').map(str::trim).filter(|f| !f.is_empty()) {
        let (k, v) = field.split_once('=')?;
        let (k, v) = (k.trim(), v.trim().strip_prefix('"')?.strip_suffix('"')?);
        if k == "page" {
            // a link, opened with xdg-open: https only, nothing odd in it
            let ok = v.starts_with("https://")
                && v.chars().all(|c| c.is_ascii_graphic())
                && !v.contains(['"', '\\', '$', '`']);
            if !ok {
                return None;
            }
            page = Some(v.to_string());
            continue;
        }
        if v.is_empty() || v.contains(['"', '\\', '$', '/']) {
            return None;
        }
        let slot = match k {
            "package" => &mut package,
            "also" => &mut also,
            "dark" => &mut dark,
            "light" => &mut light,
            _ => return None,
        };
        *slot = Some(v.to_string());
    }
    if !is_ident(key) {
        return None;
    }
    Some(IconSet {
        key: key.to_string(),
        label: if label.is_empty() { key } else { label }.to_string(),
        package: package?,
        also: also
            .map(|a: String| a.split_whitespace().map(String::from).collect())
            .unwrap_or_default(),
        names: [dark?, light?],
        page,
    })
}

/// theme.nix's icon sets and default; none if it has no iconSets
fn icon_sets(lines: &[&str]) -> Result<(Vec<IconSet>, String), String> {
    let Some((start, end)) = block(lines, 0, "iconSets = {") else {
        return Ok((Vec::new(), String::new()));
    };
    let mut sets = Vec::new();
    for (i, l) in lines.iter().enumerate().take(end).skip(start + 1) {
        let t = l.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let s = icon_line(t).ok_or(format!(
            "line {}: not a `name = {{ package = \"…\"; dark = \"…\"; light = \"…\"; }};` line:\n{t}",
            i + 1
        ))?;
        sets.push(s);
    }
    let default = lines
        .iter()
        .find_map(|l| {
            l.trim()
                .strip_prefix("iconDefault = \"")?
                .strip_suffix("\";")
                .map(str::to_string)
        })
        .ok_or("no `iconDefault = \"…\";` found")?;
    if !sets.iter().any(|s| s.key == default) {
        return Err(format!("iconDefault {default:?} isn't in iconSets"));
    }
    Ok((sets, default))
}

/// `  dark = { bg = "24221c"; fg = "d4b07b"; };` -> (0, [("bg", ..), ("fg", ..)])
fn preset_variant(l: &str) -> Option<(usize, Vec<(String, Rgb)>)> {
    let (v, rest) = l.split_once('=')?;
    let v = VARIANTS.iter().position(|n| *n == v.trim())?;
    let body = rest
        .trim()
        .strip_prefix('{')?
        .strip_suffix(';')?
        .trim_end()
        .strip_suffix('}')?;
    let mut out = Vec::new();
    for field in body.split(';').map(str::trim).filter(|f| !f.is_empty()) {
        let (k, c) = field.split_once('=')?;
        let (k, c) = (k.trim(), c.trim().strip_prefix('"')?.strip_suffix('"')?);
        if !is_ident(k) || c.len() != 6 {
            return None;
        }
        out.push((k.to_string(), parse_hex(c)?));
    }
    Some((v, out))
}

/// theme.nix's presets: `key = {  # Label`, a dark and a light line,
/// `};`; none if it has no presets
fn parse_presets(lines: &[&str]) -> Result<Vec<Preset>, String> {
    let Some(start) = lines.iter().position(|l| l.trim() == "presets = {") else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    let mut cur: Option<(Preset, [bool; 2])> = None;
    for (i, l) in lines.iter().enumerate().skip(start + 1) {
        let t = l.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let bad = || format!("line {}: not a preset line:\n{t}", i + 1);
        if t == "};" {
            match cur.take() {
                Some((p, [true, true])) => out.push(p),
                Some((p, _)) => {
                    return Err(format!("preset {}: needs a dark and a light line", p.key));
                }
                None => return Ok(out),
            }
            continue;
        }
        match &mut cur {
            None => {
                let (code, label) = match t.split_once('#') {
                    Some((c, m)) => (c.trim(), m.trim()),
                    None => (t, ""),
                };
                let key = code
                    .strip_suffix('{')
                    .and_then(|c| c.trim_end().strip_suffix('='))
                    .map(str::trim)
                    .filter(|k| is_ident(k))
                    .ok_or_else(bad)?;
                let label = if label.is_empty() { key } else { label };
                cur = Some((
                    Preset {
                        key: key.to_string(),
                        label: label.to_string(),
                        colours: Default::default(),
                    },
                    [false; 2],
                ));
            }
            Some((p, seen)) => {
                let (v, colours) = preset_variant(t).ok_or_else(bad)?;
                if seen[v] {
                    return Err(format!("preset {}: two {} lines", p.key, VARIANTS[v]));
                }
                seen[v] = true;
                p.colours[v] = colours;
            }
        }
    }
    Err("presets = { ... }; isn't closed".into())
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
    let (icon_sets, icon_default) = icon_sets(&lines)?;

    // the defaults, then the presets, each with exactly the palette's colours
    let default_label = lines
        .iter()
        .find_map(|l| {
            l.trim()
                .strip_prefix("defaultPreset = \"")?
                .strip_suffix("\";")
                .map(str::to_string)
        })
        .unwrap_or_else(|| "defaults".into());
    let mut presets = vec![Preset {
        key: String::new(),
        label: default_label,
        colours: [0, 1].map(|v| {
            sections
                .iter()
                .flat_map(|s| s.entries.iter())
                .map(|e| (e.name.clone(), e.default[v]))
                .collect()
        }),
    }];
    for p in parse_presets(&lines)? {
        for (v, colours) in p.colours.iter().enumerate() {
            let mut have: Vec<&str> = colours.iter().map(|(n, _)| n.as_str()).collect();
            have.sort();
            if have != dn {
                return Err(format!(
                    "preset {}: {} must have every colour of the palettes, once",
                    p.key, VARIANTS[v]
                ));
            }
        }
        presets.push(p);
    }

    let theme = Theme {
        sections,
        derived,
        variant: 0,
        saved_variant: 0,
        icon_sets,
        icons: icon_default.clone(),
        saved_icons: icon_default.clone(),
        icon_default,
        wallpaper: None,
        saved_wallpaper: None,
        wall_folders: Vec::new(),
        saved_wall_folders: Vec::new(),
        extra: Vec::new(),
        presets,
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
    /// one if the variant changed, and one if the icon set did
    pub fn changed(&self) -> usize {
        let colours: usize = self
            .entries()
            .map(|e| (0..2).filter(|&v| e.color[v] != e.saved[v]).count())
            .sum();
        colours
            + usize::from(self.variant != self.saved_variant)
            + usize::from(self.icons != self.saved_icons)
            + usize::from(self.wallpaper != self.saved_wallpaper)
            + usize::from(self.wall_folders != self.saved_wall_folders)
    }

    pub fn mark_saved(&mut self) {
        for e in self.entries_mut() {
            e.saved = e.color;
        }
        self.saved_variant = self.variant;
        self.saved_icons = self.icons.clone();
        self.saved_wallpaper = self.wallpaper.clone();
        self.saved_wall_folders = self.wall_folders.clone();
    }

    /// back to what was saved
    pub fn undo_all(&mut self) {
        for e in self.entries_mut() {
            e.color = e.saved;
        }
        self.variant = self.saved_variant;
        self.icons = self.saved_icons.clone();
        self.wallpaper = self.saved_wallpaper.clone();
        self.wall_folders = self.saved_wall_folders.clone();
    }

    /// every colour, dark and light, set to preset i's (unsaved)
    pub fn apply_preset(&mut self, i: usize) {
        let colours = self.presets[i].colours.clone();
        for e in self.entries_mut() {
            for (v, list) in colours.iter().enumerate() {
                if let Some((_, c)) = list.iter().find(|(n, _)| *n == e.name) {
                    e.color[v] = *c;
                }
            }
        }
    }

    /// the preset whose colours, dark and light, these are, if any (two
    /// presets can share a variant: Copper's light is Desert Night's)
    pub fn preset_now(&self) -> Option<usize> {
        self.presets.iter().position(|p| {
            self.entries().all(|e| {
                (0..2).all(|v| {
                    p.colours[v]
                        .iter()
                        .any(|(n, c)| *n == e.name && *c == e.color[v])
                })
            })
        })
    }

    pub fn icon_set(&self, key: &str) -> Option<&IconSet> {
        self.icon_sets.iter().find(|s| s.key == key)
    }

    /// apply colors.json (already parsed); unknown names are reported
    pub fn load_local(&mut self, local: &Local) -> Vec<String> {
        let mut unknown = Vec::new();
        if let Some(v) = local.variant {
            self.variant = v;
            self.saved_variant = v;
        }
        if let Some(i) = &local.icons {
            if self.icon_set(i).is_some() {
                self.icons = i.clone();
                self.saved_icons = i.clone();
            } else {
                unknown.push(format!("icons {i:?}"));
            }
        }
        self.wallpaper = local.wallpaper.clone();
        self.saved_wallpaper = local.wallpaper.clone();
        self.wall_folders = local.wall_folders.clone();
        self.saved_wall_folders = local.wall_folders.clone();
        self.extra = local.extra.clone();
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

    /// colors.json: the variant, the colours that differ from theme.nix,
    /// the icon set if it isn't the default, and keys kept from the file
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
        if self.icons != self.icon_default {
            out += &format!(",\n  \"icons\": \"{}\"", self.icons);
        }
        if let Some(w) = &self.wallpaper {
            out += &format!(
                ",\n  \"wallpaper\": {{ \"from\": \"{}\", \"mode\": \"{}\" }}",
                w.from, w.mode
            );
        }
        if !self.wall_folders.is_empty() {
            let list: Vec<String> = self
                .wall_folders
                .iter()
                .map(|f| format!("\"{f}\""))
                .collect();
            out += &format!(",\n  \"wallpaperFolders\": [ {} ]", list.join(", "));
        }
        for (k, v) in &self.extra {
            out += &format!(",\n  \"{k}\": {v}");
        }
        out + "\n}\n"
    }
}

/// colors.json, parsed
#[derive(Default)]
pub struct Local {
    pub variant: Option<usize>,
    pub colours: [Vec<(String, Rgb)>; 2],
    pub icons: Option<String>,
    pub wallpaper: Option<Wallpaper>,
    pub wall_folders: Vec<String>,
    /// other keys, as JSON text
    pub extra: Vec<(String, String)>,
}

/// Reads colors.json; unknown keys are kept (strings, objects and lists).
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
            ("icons", Value::Str(i)) => local.icons = Some(i),
            ("wallpaper", Value::Obj(w)) => {
                let (mut from, mut mode) = (None, None);
                for (k, v) in w {
                    match (k.as_str(), v) {
                        ("from", Value::Str(s)) if !s.is_empty() => from = Some(s),
                        ("mode", Value::Str(s)) if MODES.contains(&s.as_str()) => mode = Some(s),
                        (k, _) => return Err(format!("wallpaper.{k}: not a path or a mode")),
                    }
                }
                local.wallpaper = Some(Wallpaper {
                    from: from.ok_or("wallpaper: no \"from\"")?,
                    mode: mode.unwrap_or_else(|| "fill".into()),
                });
            }
            ("wallpaperFolders", Value::Arr(list)) => {
                for v in list {
                    match v {
                        Value::Str(s) if s.starts_with('/') => local.wall_folders.push(s),
                        _ => return Err("wallpaperFolders: not a list of folders".into()),
                    }
                }
            }
            ("variant" | "dark" | "light" | "icons" | "wallpaper" | "wallpaperFolders", _) => {
                return Err(format!("{key:?} has the wrong type"));
            }
            (_, v) => local.extra.push((key.clone(), v.json())),
        }
    }
    Ok(local)
}

enum Value {
    Str(String),
    Obj(Vec<(String, Value)>),
    Arr(Vec<Value>),
}

impl Value {
    /// back to JSON (strings have no escapes: the reader refuses them)
    fn json(&self) -> String {
        match self {
            Value::Str(s) => format!("\"{s}\""),
            Value::Obj(o) => {
                let inner: Vec<String> = o
                    .iter()
                    .map(|(k, v)| format!("\"{k}\": {}", v.json()))
                    .collect();
                if inner.is_empty() {
                    "{}".into()
                } else {
                    format!("{{ {} }}", inner.join(", "))
                }
            }
            Value::Arr(a) => {
                let inner: Vec<String> = a.iter().map(Value::json).collect();
                if inner.is_empty() {
                    "[]".into()
                } else {
                    format!("[ {} ]", inner.join(", "))
                }
            }
        }
    }
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
            Some(b'[') => Ok(Value::Arr(self.array()?)),
            _ => Err(format!(
                "expected a string, object or list at byte {}",
                self.i
            )),
        }
    }

    fn array(&mut self) -> Result<Vec<Value>, String> {
        self.eat(b'[')?;
        let mut out = Vec::new();
        self.ws();
        if self.s.get(self.i) == Some(&b']') {
            self.i += 1;
            return Ok(out);
        }
        loop {
            out.push(self.value()?);
            self.ws();
            match self.s.get(self.i) {
                Some(b',') => self.i += 1,
                Some(b']') => {
                    self.i += 1;
                    return Ok(out);
                }
                _ => return Err(format!("expected ',' or ']' at byte {}", self.i)),
            }
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
        assert_eq!(t.c("accent"), [0xc8, 0x6f, 0x42]);
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
        assert_eq!(s.notes, ["Desert Night's amber (a preset): e5a440"]);
        assert_eq!(t.derived.len(), 2);
    }

    #[test]
    fn derived_matches_nix() {
        // from nix-instantiate --eval of theme.nix
        let mut t = parse(FILE).unwrap();
        assert_eq!(hex(t.c("selectionBg")), "945535");
        assert_eq!(hex(t.c("accentDim")), "4e3123");
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
    fn icon_sets_from_theme_nix() {
        let t = parse(FILE).unwrap();
        assert_eq!(t.icon_sets.len(), 16);
        assert_eq!(t.icon_default, "gruvbox-plus");
        assert_eq!(t.icons, "gruvbox-plus");
        let p = t.icon_set("papirus").unwrap();
        assert_eq!(p.label, "Papirus");
        assert_eq!(p.package, "papirus-icon-theme");
        assert_eq!(p.names, ["Papirus-Dark", "Papirus-Light"]);
        assert!(p.also.is_empty());
        let n = t.icon_set("numix-circle").unwrap();
        assert_eq!(n.also, ["numix-icon-theme"]);
        assert_eq!(n.names, ["Numix-Circle", "Numix-Circle-Light"]);
        assert_eq!(t.icon_set("rose-pine").unwrap().label, "Rosé Pine");
        for bad in [
            "x = { package = \"p\"; dark = \"D\"; };",
            "x = { package = \"p\"; dark = \"D\"; light = \"../L\"; };",
            "x = { package = \"p\"; dark = \"D\"; light = \"L\"; other = \"o\"; };",
            "x = { package = p; dark = \"D\"; light = \"L\"; };",
            "x y = { package = \"p\"; dark = \"D\"; light = \"L\"; };",
        ] {
            assert!(icon_line(bad).is_none(), "{bad}");
        }
        // an older theme.nix, without icon sets, still reads
        let old: String = FILE
            .lines()
            .filter(|l| {
                !l.contains("package = ")
                    && !l.contains("iconSets = {")
                    && !l.contains("iconDefault =")
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(parse(&old).unwrap().icon_sets.is_empty());
    }

    #[test]
    fn icons_and_other_keys_in_colors_json() {
        let mut t = parse(FILE).unwrap();
        let unknown = t.load_local(
            &parse_local(r#"{"variant":"dark","icons":"papirus","note":"hi","wallpaper":{"from":"/a b.png","mode":"fit"},"e":{}}"#)
                .unwrap(),
        );
        assert!(unknown.is_empty());
        assert_eq!((t.icons.as_str(), t.changed()), ("papirus", 0));
        let json = t.to_json();
        assert_eq!(
            json,
            "{\n  \"variant\": \"dark\",\n  \"dark\": {},\n  \"light\": {},\n  \"icons\": \"papirus\",\n  \"wallpaper\": { \"from\": \"/a b.png\", \"mode\": \"fit\" },\n  \"note\": \"hi\",\n  \"e\": {}\n}\n"
        );
        // reads back the same
        let mut again = parse(FILE).unwrap();
        again.load_local(&parse_local(&json).unwrap());
        assert_eq!(again.to_json(), json);
        // picking another counts as one change; undo goes back
        t.icons = "kora".into();
        assert_eq!(t.changed(), 1);
        t.undo_all();
        assert_eq!((t.icons.as_str(), t.changed()), ("papirus", 0));
        // the default isn't written
        t.icons = "gruvbox-plus".into();
        assert!(!t.to_json().contains("icons"));
        t.mark_saved();
        assert_eq!(t.changed(), 0);
        // an unknown set is reported, and the default used
        let mut u = parse(FILE).unwrap();
        assert_eq!(
            u.load_local(&parse_local(r#"{"icons":"nope"}"#).unwrap()),
            ["icons \"nope\""]
        );
        assert_eq!(u.icons, "gruvbox-plus");
    }

    #[test]
    fn wallpaper_in_colors_json() {
        let mut t = parse(FILE).unwrap();
        t.load_local(&parse_local(r#"{"wallpaper":{"from":"/w/x.jpg"}}"#).unwrap());
        let w = Wallpaper {
            from: "/w/x.jpg".into(),
            mode: "fill".into(),
        };
        assert_eq!(t.wallpaper.as_ref(), Some(&w));
        assert_eq!(t.changed(), 0);
        t.wallpaper.as_mut().unwrap().mode = "tile".into();
        assert_eq!(t.changed(), 1);
        t.undo_all();
        assert_eq!(t.wallpaper.as_ref(), Some(&w));
        // back to the default: no key
        t.wallpaper = None;
        assert!(!t.to_json().contains("wallpaper"));
        t.mark_saved();
        assert_eq!(t.changed(), 0);
    }

    #[test]
    fn wallpaper_folders_and_lists() {
        let mut t = parse(FILE).unwrap();
        t.load_local(
            &parse_local(r#"{"wallpaperFolders":["/mnt/a b","/w"],"other":["x",{"y":"z"},[]]}"#)
                .unwrap(),
        );
        assert_eq!(t.wall_folders, ["/mnt/a b", "/w"]);
        let json = t.to_json();
        assert!(json.contains("\n  \"wallpaperFolders\": [ \"/mnt/a b\", \"/w\" ]"));
        assert!(json.contains("\n  \"other\": [ \"x\", { \"y\": \"z\" }, [] ]"));
        let mut again = parse(FILE).unwrap();
        again.load_local(&parse_local(&json).unwrap());
        assert_eq!(again.to_json(), json);
        t.wall_folders.pop();
        assert_eq!(t.changed(), 1);
        t.undo_all();
        assert_eq!(t.changed(), 0);
        t.wall_folders.clear();
        assert!(!t.to_json().contains("wallpaperFolders"));
        for bad in [
            r#"{"wallpaperFolders":"/a"}"#,
            r#"{"wallpaperFolders":["relative"]}"#,
            r#"{"wallpaperFolders":[{}]}"#,
            r#"{"wallpaperFolders":["/a",]}"#,
            r#"{"x":[}"#,
        ] {
            assert!(parse_local(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn icon_set_pages() {
        let t = parse(FILE).unwrap();
        assert!(
            t.icon_sets.iter().all(|s| s.page.is_some()),
            "every set has a page"
        );
        let ok = icon_line(
            "x = { package = \"p\"; dark = \"D\"; light = \"L\"; page = \"https://a.org/b\"; };",
        )
        .unwrap();
        assert_eq!(ok.page.as_deref(), Some("https://a.org/b"));
        for bad in [
            "x = { package = \"p\"; dark = \"D\"; light = \"L\"; page = \"http://a.org\"; };",
            "x = { package = \"p\"; dark = \"D\"; light = \"L\"; page = \"https://a.org/$(x)\"; };",
            "x = { package = \"p\"; dark = \"D\"; light = \"L\"; page = \"https://a b\"; };",
        ] {
            assert!(icon_line(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn presets_from_theme_nix() {
        let mut t = parse(FILE).unwrap();
        let labels: Vec<_> = t.presets.iter().map(|p| p.label.as_str()).collect();
        assert_eq!(labels, ["Copper", "Desert Night"]);
        assert_eq!(t.presets[1].key, "desert-night");
        assert_eq!(t.preset_now(), Some(0));
        // Desert Night: the old defaults, dark and light
        t.apply_preset(1);
        assert_eq!(hex(t.c("accent")), "e5a440");
        assert_eq!(hex(t.c("bg")), "24221c");
        assert_eq!(hex(t.c("selectionBg")), "a57934");
        assert_eq!(t.preset_now(), Some(1));
        // the seven dark colours that differ from Copper; light is the same
        assert_eq!(t.changed(), 7);
        let json = t.to_json();
        assert!(json.contains("\"accent\": \"e5a440\"") && json.contains("\"light\": {}"));
        // saved and read back, it's still Desert Night
        let mut again = parse(FILE).unwrap();
        again.load_local(&parse_local(&json).unwrap());
        assert_eq!(again.preset_now(), Some(1));
        // undo, or picking the default, goes back
        t.undo_all();
        assert_eq!((t.changed(), t.preset_now()), (0, Some(0)));
        t.apply_preset(1);
        t.apply_preset(0);
        assert_eq!(t.changed(), 0);
        // one colour changed by hand: no preset matches
        t.entries_mut().find(|e| e.name == "red").unwrap().color[0] = [1, 2, 3];
        assert_eq!(t.preset_now(), None);
        // dark and light are both matched: in light, Desert Night is still
        // Desert Night though its light colours are Copper's
        t.undo_all();
        t.apply_preset(1);
        t.variant = 1;
        assert_eq!(t.preset_now(), Some(1));
        t.entries_mut().find(|e| e.name == "red").unwrap().color[0] = [1, 2, 3];
        assert_eq!(t.preset_now(), None);
    }

    #[test]
    fn bad_presets() {
        let with = |old: &str, new: &str| {
            assert!(FILE.contains(old), "{old}");
            parse(&FILE.replacen(old, new, 1))
        };
        // a colour missing, unknown, twice, or not hex
        assert!(with("bg = \"24221c\"; ", "").is_err());
        assert!(with("bg = \"24221c\"; ", "bg = \"24221c\"; nope = \"123456\"; ").is_err());
        assert!(with("bg = \"24221c\"; ", "bg = \"24221c\"; bg = \"24221c\"; ").is_err());
        assert!(with("bg = \"24221c\"; ", "bg = \"24221g\"; ").is_err());
        // no light line, two dark lines, not closed
        let light = FILE
            .lines()
            .find(|l| l.trim_start().starts_with("light = { bgDark"))
            .unwrap();
        assert!(with(&format!("{light}\n"), "").is_err());
        let dark = FILE
            .lines()
            .find(|l| l.trim_start().starts_with("dark = { bgDark"))
            .unwrap();
        assert!(with(&format!("{light}\n"), &format!("{dark}\n")).is_err());
        assert!(with("    };\n  };\n\n  # Worked out", "    };\n\n  # Worked out").is_err());
        // a bad name
        assert!(with("desert-night = {  # Desert Night", "desert night = {").is_err());
        // a theme.nix without presets still reads: just the defaults
        let s = FILE.find("  presets = {").unwrap();
        let e = FILE[s..].find("\n  };\n").unwrap() + s + 5;
        let old = format!("{}{}", &FILE[..s], &FILE[e..]);
        let t = parse(&old).unwrap();
        assert_eq!(t.presets.len(), 1);
        assert_eq!(t.presets[0].label, "Copper");
    }

    #[test]
    fn bad_colors_json() {
        for bad in [
            r#"{"variant":"blue"}"#,
            r#"{"dark":{"accent":"zzz"}}"#,
            r##"{"dark":{"accent":"#e5a440"}}"##,
            r#"{"dark":{"accent":1}}"#,
            r#"{"icons":{}}"#,
            r#"{"wallpaper":"x"}"#,
            r#"{"wallpaper":{"mode":"fill"}}"#,
            r#"{"wallpaper":{"from":"/a","mode":"zoom"}}"#,
            r#"{"wallpaper":{"from":"","mode":"fill"}}"#,
            r#"{"wallpaper":{"from":"/a","x":"y"}}"#,
            r#"{"variant":{}}"#,
            r#"{"dark":"x"}"#,
            r#"{"dark":{}} x"#,
            r#"{"dark":{"a":"A"}}"#,
        ] {
            assert!(parse_local(bad).is_err(), "{bad}");
        }
    }
}
