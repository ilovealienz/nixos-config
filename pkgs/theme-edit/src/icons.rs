//! Finds icon themes that are installed, and icons in them, for the
//! picker's previews (freedesktop icon theme spec, simplified: the
//! closest size, then the themes it inherits, then hicolor).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The icons each set's preview shows: a few names for each, as sets
/// name some of them differently.
pub const SAMPLES: &[&[&str]] = &[
    &["folder"],
    &["user-home", "folder-home"],
    &["text-x-generic"],
    &["image-x-generic"],
    &["audio-x-generic"],
    &["utilities-terminal", "terminal", "org.gnome.Terminal"],
    &["web-browser", "internet-web-browser", "firefox"],
    &[
        "preferences-system",
        "preferences-desktop",
        "systemsettings",
    ],
];

/// Folders icon themes are installed in, most important first:
/// ~/.local/share/icons, ~/.icons, then icons/ in each XDG_DATA_DIRS
/// folder (on NixOS, the home-manager and system profiles).
pub fn roots() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/share"));
    let dirs = std::env::var("XDG_DATA_DIRS")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    let mut out = vec![data_home.join("icons"), home.join(".icons")];
    for d in dirs.split(':').filter(|d| !d.is_empty()) {
        let p = Path::new(d).join("icons");
        if !out.contains(&p) {
            out.push(p);
        }
    }
    out
}

/// One folder of a theme, from index.theme.
struct Dir {
    path: String,
    min: u32,
    max: u32,
    scale: u32,
}

impl Dir {
    /// how far from `want` this folder's icons are (0: a fit)
    fn distance(&self, want: u32) -> u32 {
        // index.theme is read from disk: no overflow on odd numbers
        let s = self.scale.max(1);
        let want = want.saturating_mul(s);
        let (min, max) = (self.min.saturating_mul(s), self.max.saturating_mul(s));
        if want < min {
            min - want
        } else {
            want.saturating_sub(max)
        }
    }
}

struct Index {
    inherits: Vec<String>,
    dirs: Vec<Dir>,
}

/// index.theme, the parts used here
fn parse_index(text: &str) -> Index {
    let mut inherits = Vec::new();
    let mut listed: Vec<String> = Vec::new();
    let mut sections: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut current = String::new();
    for l in text.lines() {
        let l = l.trim();
        if let Some(name) = l.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            current = name.to_string();
            continue;
        }
        let Some((k, v)) = l.split_once('=') else {
            continue;
        };
        let (k, v) = (k.trim(), v.trim());
        if current == "Icon Theme" {
            let list = || {
                v.split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(String::from)
            };
            match k {
                "Inherits" => inherits = list().collect(),
                "Directories" | "ScaledDirectories" => listed.extend(list()),
                _ => {}
            }
        } else {
            sections
                .entry(current.clone())
                .or_default()
                .insert(k.to_string(), v.to_string());
        }
    }
    let mut dirs = Vec::new();
    for path in listed {
        let Some(s) = sections.get(&path) else {
            continue;
        };
        let num = |k: &str| s.get(k).and_then(|v| v.parse::<u32>().ok());
        let Some(size) = num("Size") else {
            continue;
        };
        let threshold = num("Threshold").unwrap_or(2);
        let (min, max) = match s.get("Type").map(String::as_str) {
            Some("Fixed") => (size, size),
            Some("Scalable") => (
                num("MinSize").unwrap_or(size),
                num("MaxSize").unwrap_or(size),
            ),
            _ => (
                size.saturating_sub(threshold),
                size.saturating_add(threshold),
            ),
        };
        dirs.push(Dir {
            path,
            min,
            max,
            scale: num("Scale").unwrap_or(1),
        });
    }
    Index { inherits, dirs }
}

/// Installed icon themes, found and read once each.
pub struct Themes {
    roots: Vec<PathBuf>,
    /// theme name -> its folders in each root, and its index (None: not
    /// installed)
    cache: HashMap<String, Option<(Vec<PathBuf>, Index)>>,
}

impl Themes {
    pub fn new(roots: Vec<PathBuf>) -> Self {
        Themes {
            roots,
            cache: HashMap::new(),
        }
    }

    fn theme(&mut self, name: &str) -> Option<&(Vec<PathBuf>, Index)> {
        if !self.cache.contains_key(name) {
            // a theme's files can be spread over several roots; the first
            // index.theme found describes it
            let bases: Vec<PathBuf> = self
                .roots
                .iter()
                .map(|r| r.join(name))
                .filter(|p| p.is_dir())
                .collect();
            let index = bases
                .iter()
                .find_map(|b| std::fs::read_to_string(b.join("index.theme")).ok())
                .map(|t| parse_index(&t));
            let found = index.map(|i| (bases, i));
            self.cache.insert(name.to_string(), found);
        }
        self.cache.get(name).and_then(Option::as_ref)
    }

    /// whether the theme is installed (has an index.theme)
    pub fn installed(&mut self, name: &str) -> bool {
        // names come from theme.nix; never step outside the roots
        !matches!(name, "" | "." | "..") && !name.contains('/') && self.theme(name).is_some()
    }

    /// the file for the first of `names` the theme (or what it inherits,
    /// then hicolor) has, closest to `size` pixels
    pub fn lookup(&mut self, theme: &str, names: &[&str], size: u32) -> Option<PathBuf> {
        let mut seen: Vec<String> = Vec::new();
        let mut queue = vec![theme.to_string()];
        while let Some(t) = queue.first().cloned() {
            queue.remove(0);
            if seen.contains(&t) || seen.len() > 20 || !self.installed(&t) {
                continue;
            }
            seen.push(t.clone());
            if let Some(p) = names.iter().find_map(|n| self.in_theme(&t, n, size)) {
                return Some(p);
            }
            let (_, index) = self.theme(&t)?;
            queue.extend(index.inherits.iter().cloned());
        }
        if !seen.iter().any(|t| t == "hicolor") && theme != "hicolor" {
            return names.iter().find_map(|n| self.in_theme("hicolor", n, size));
        }
        None
    }

    fn in_theme(&mut self, theme: &str, name: &str, size: u32) -> Option<PathBuf> {
        if name.contains('/') {
            return None;
        }
        let (bases, index) = self.theme(theme)?;
        let mut dirs: Vec<&Dir> = index.dirs.iter().collect();
        dirs.sort_by_key(|d| d.distance(size));
        for d in dirs {
            for base in bases {
                for ext in ["svg", "png"] {
                    let p = base.join(&d.path).join(format!("{name}.{ext}"));
                    if p.is_file() {
                        return Some(p);
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(p: &Path, text: &str) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    #[test]
    fn finds_icons_by_size_and_inherits() {
        let root = std::env::temp_dir().join(format!("theme-edit-icons-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        write(
            &root.join("Mine/index.theme"),
            "[Icon Theme]\nName=Mine\nInherits=Base\nDirectories=16x16/places,48x48/places,scalable/apps\n\n\
             [16x16/places]\nSize=16\nType=Fixed\n\n[48x48/places]\nSize=48\nType=Fixed\n\n\
             [scalable/apps]\nSize=48\nMinSize=8\nMaxSize=512\nType=Scalable\n",
        );
        write(&root.join("Mine/16x16/places/folder.png"), "");
        write(&root.join("Mine/48x48/places/folder.png"), "");
        write(&root.join("Mine/scalable/apps/utilities-terminal.svg"), "");
        write(
            &root.join("Base/index.theme"),
            "[Icon Theme]\nInherits=Mine\nDirectories=24\n[24]\nSize=24\n",
        );
        write(&root.join("Base/24/text-x-generic.png"), "");
        write(
            &root.join("hicolor/index.theme"),
            "[Icon Theme]\nDirectories=32\n[32]\nSize=32\n",
        );
        write(&root.join("hicolor/32/firefox.png"), "");
        write(&root.join("Cursors/cursors/left_ptr"), "");

        let mut t = Themes::new(vec![root.join("missing"), root.clone()]);
        assert!(t.installed("Mine"));
        assert!(!t.installed("Cursors"), "no index.theme: not an icon theme");
        assert!(!t.installed("../Mine"));
        assert!(!t.installed(".") && !t.installed(".."));
        let rel =
            |p: Option<PathBuf>| p.map(|p| p.strip_prefix(&root).unwrap().display().to_string());
        // closest size: 16 for 20, 48 for 40
        assert_eq!(
            rel(t.lookup("Mine", &["folder"], 20)).as_deref(),
            Some("Mine/16x16/places/folder.png")
        );
        assert_eq!(
            rel(t.lookup("Mine", &["folder"], 40)).as_deref(),
            Some("Mine/48x48/places/folder.png")
        );
        // scalable fits any size in its range
        assert_eq!(
            rel(t.lookup("Mine", &["terminal", "utilities-terminal"], 24)).as_deref(),
            Some("Mine/scalable/apps/utilities-terminal.svg")
        );
        // from the inherited theme (which inherits back: no loop), then hicolor
        assert_eq!(
            rel(t.lookup("Mine", &["text-x-generic"], 24)).as_deref(),
            Some("Base/24/text-x-generic.png")
        );
        assert_eq!(
            rel(t.lookup("Mine", &["firefox"], 24)).as_deref(),
            Some("hicolor/32/firefox.png")
        );
        assert_eq!(t.lookup("Mine", &["nothing"], 24), None);
        assert_eq!(t.lookup("NotThere", &["folder"], 24), None);
        let _ = std::fs::remove_dir_all(&root);
    }
}
