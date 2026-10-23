//! Installed desktop apps (.desktop files), and the copies that make one
//! always open in gamescope.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// the key that marks a copy as nxgs's (its value: the entry's slug)
pub const MARK: &str = "X-nxgs";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct App {
    /// the desktop file ID: "org.vinegarhq.Sober.desktop"
    pub id: String,
    pub name: String,
    /// Exec with the field codes taken out
    pub command: String,
    pub path: PathBuf,
    /// an nxgs copy (always opens in gamescope)
    pub ours: bool,
}

impl App {
    /// launched through Steam, so gamescope has to go in its launch options
    pub fn is_steam_game(&self) -> bool {
        self.command.contains("steam://rungameid") || self.command.contains("steam://run/")
    }
}

/// $XDG_DATA_HOME, then $XDG_DATA_DIRS: the application folders, first wins
pub fn dirs() -> Vec<PathBuf> {
    let home = std::env::var_os("XDG_DATA_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::entry::home().join(".local/share"));
    let rest = std::env::var("XDG_DATA_DIRS")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    std::iter::once(home)
        .chain(rest.split(':').filter(|s| !s.is_empty()).map(PathBuf::from))
        .map(|d| d.join("applications"))
        .collect()
}

/// the apps a launcher would show, by name; nxgs's own copies are skipped
/// unless `ours`
pub fn scan(dirs: &[PathBuf], ours: bool) -> Vec<App> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for d in dirs {
        let mut files = Vec::new();
        walk(d, d, &mut files);
        files.sort();
        for (id, path) in files {
            let Ok(text) = std::fs::read_to_string(&path) else { continue };
            let Some(app) = parse(&id, &path, &text) else {
                // hidden or not an app: still hides the same ID further on
                seen.insert(id);
                continue;
            };
            if app.ours && !ours {
                continue;
            }
            if seen.insert(id) {
                out.push(app);
            }
        }
    }
    out.sort_by_key(|a| a.name.to_lowercase());
    out
}

/// the app behind an ID, never an nxgs copy (that would run nxgs again)
pub fn find(dirs: &[PathBuf], id: &str) -> Option<App> {
    dirs.iter().find_map(|d| {
        let path = d.join(id.replace('-', "/"));
        let flat = d.join(id);
        [flat, path].into_iter().find_map(|p| {
            let app = parse(id, &p, &std::fs::read_to_string(&p).ok()?)?;
            (!app.ours).then_some(app)
        })
    })
}

/// .desktop files under `dir`, with their IDs (subfolders joined with -)
fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(root, &p, out);
        } else if p.extension().is_some_and(|x| x == "desktop")
            && let Ok(rel) = p.strip_prefix(root)
        {
            out.push((rel.to_string_lossy().replace('/', "-"), p));
        }
    }
}

/// the [Desktop Entry] keys we need; None if hidden or not an app
pub fn parse(id: &str, path: &Path, text: &str) -> Option<App> {
    let mut in_entry = false;
    let (mut name, mut exec, mut kind, mut hidden, mut ours) = (None, None, None, false, false);
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        match k.trim() {
            "Name" => name = Some(v.trim().to_string()),
            "Exec" => exec = Some(v.trim().to_string()),
            "Type" => kind = Some(v.trim().to_string()),
            "NoDisplay" | "Hidden" if v.trim() == "true" => hidden = true,
            MARK => ours = true,
            _ => {}
        }
    }
    if hidden || kind.as_deref() != Some("Application") {
        return None;
    }
    Some(App {
        id: id.to_string(),
        name: name?,
        command: clean_exec(&exec?),
        path: path.to_path_buf(),
        ours,
    })
}

/// Exec without its field codes (%U, %f...) and flatpak's @@ markers;
/// %% stays a single %
pub fn clean_exec(exec: &str) -> String {
    let mut words = Vec::new();
    for w in exec.split_whitespace() {
        if w == "@@" || w == "@@u" || w == "@@f" {
            continue;
        }
        if w.len() == 2 && w.starts_with('%') && w != "%%" {
            continue;
        }
        words.push(w.replace("%%", "%"));
    }
    words.join(" ")
}

/// ~/.local/share/applications/<id>: the copy that opens it in gamescope
pub fn copy_path(id: &str) -> PathBuf {
    dirs()
        .into_iter()
        .next()
        .unwrap_or_else(|| PathBuf::from("applications"))
        .join(id)
}

/// the app's desktop file with Exec running `nxgs run <slug>` (the
/// [Desktop Entry] section only: other actions stay as they were) and
/// D-Bus activation off, so launchers use that Exec
pub fn make_copy(original: &str, run: &str, slug: &str) -> String {
    let mut out = String::new();
    let mut in_entry = false;
    for line in original.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            if in_entry {
                out.push_str(&format!("DBusActivatable=false\n{MARK}={slug}\n"));
            }
            in_entry = t == "[Desktop Entry]";
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if in_entry {
            match t.split_once('=').map(|(k, _)| k.trim()) {
                Some("Exec") => {
                    out.push_str(&format!("Exec={run}\n"));
                    continue;
                }
                Some("DBusActivatable") | Some(MARK) | Some("TryExec") => continue,
                _ => {}
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    if in_entry {
        out.push_str(&format!("DBusActivatable=false\n{MARK}={slug}\n"));
    }
    out
}

/// writes the copy (refusing to replace a file that isn't nxgs's)
pub fn install_copy(app: &App, run: &str, slug: &str) -> Result<PathBuf, String> {
    let dest = copy_path(&app.id);
    if dest.exists() && !is_ours(&dest) {
        return Err(format!("{} is already there and isn't nxgs's", dest.display()));
    }
    let text = std::fs::read_to_string(&app.path).map_err(|e| e.to_string())?;
    if let Some(d) = dest.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    std::fs::write(&dest, make_copy(&text, run, slug)).map_err(|e| e.to_string())?;
    Ok(dest)
}

/// removes the copy if it's nxgs's
pub fn remove_copy(id: &str) -> Result<bool, String> {
    let dest = copy_path(id);
    if !dest.exists() {
        return Ok(false);
    }
    if !is_ours(&dest) {
        return Err(format!("{} isn't nxgs's, left alone", dest.display()));
    }
    std::fs::remove_file(&dest).map_err(|e| e.to_string())?;
    Ok(true)
}

pub fn is_ours(path: &Path) -> bool {
    owner_of(path).is_some()
}

/// the slug of the entry an nxgs copy runs, None if it isn't one
pub fn owner_of(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    text.lines()
        .find_map(|l| l.trim_start().strip_prefix(&format!("{MARK}=")).map(|s| s.trim().to_string()))
}

/// which entry the app's copy belongs to (several entries can run one app)
pub fn pinned_by(id: &str) -> Option<String> {
    owner_of(&copy_path(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOBER: &str = "[Desktop Entry]\nName=Sober\nType=Application\nExec=/usr/bin/flatpak run --branch=stable --command=sober org.vinegarhq.Sober @@u %u @@\nDBusActivatable=true\nTryExec=sober\n\n[Desktop Action play]\nName=Play\nExec=sober --play\n";

    #[test]
    fn cleans_exec() {
        assert_eq!(clean_exec("brave %U"), "brave");
        assert_eq!(clean_exec("a --x=%%1 %f"), "a --x=%1");
        assert_eq!(
            clean_exec("/usr/bin/flatpak run --command=sober org.vinegarhq.Sober @@u %u @@"),
            "/usr/bin/flatpak run --command=sober org.vinegarhq.Sober"
        );
    }

    #[test]
    fn parses_and_skips_hidden() {
        let a = parse("sober.desktop", Path::new("/x"), SOBER).unwrap();
        assert_eq!(a.name, "Sober");
        assert_eq!(a.command, "/usr/bin/flatpak run --branch=stable --command=sober org.vinegarhq.Sober");
        assert!(!a.ours);
        assert!(parse("h", Path::new("/x"), "[Desktop Entry]\nName=H\nType=Application\nExec=h\nNoDisplay=true\n").is_none());
        assert!(parse("l", Path::new("/x"), "[Desktop Entry]\nName=L\nType=Link\nURL=x\n").is_none());
        let steam = parse("s", Path::new("/x"), "[Desktop Entry]\nName=Sims\nType=Application\nExec=steam steam://rungameid/1222670\n").unwrap();
        assert!(steam.is_steam_game());
    }

    #[test]
    fn copy_runs_nxgs() {
        let c = make_copy(SOBER, "nxgs run sober", "sober");
        assert!(c.contains("[Desktop Entry]\nName=Sober\nType=Application\nExec=nxgs run sober\n"), "{c}");
        assert!(c.contains("DBusActivatable=false\nX-nxgs=sober\n[Desktop Action play]"), "{c}");
        // the action keeps its own Exec
        assert!(c.contains("Exec=sober --play"), "{c}");
        assert!(!c.contains("TryExec") && !c.contains("DBusActivatable=true"), "{c}");
        let a = parse("sober.desktop", Path::new("/x"), &c).unwrap();
        assert!(a.ours);
        let f = std::env::temp_dir().join(format!("nxgs-owner-{}.desktop", std::process::id()));
        std::fs::write(&f, &c).unwrap();
        assert_eq!(owner_of(&f).as_deref(), Some("sober"));
        let _ = std::fs::remove_file(&f);
        // a file that's just the entry section
        let d = make_copy("[Desktop Entry]\nName=A\nExec=a %U\nType=Application\n", "nxgs run a", "a");
        assert!(d.ends_with("Exec=nxgs run a\nType=Application\nDBusActivatable=false\nX-nxgs=a\n"), "{d}");
    }

    #[test]
    fn scans_with_ids_and_precedence() {
        let base = std::env::temp_dir().join(format!("nxgs-apps-{}", std::process::id()));
        let (home, sys) = (base.join("home"), base.join("sys"));
        std::fs::create_dir_all(home.join("kde")).unwrap();
        std::fs::create_dir_all(&sys).unwrap();
        std::fs::write(sys.join("a.desktop"), "[Desktop Entry]\nName=Alpha\nType=Application\nExec=alpha %U\n").unwrap();
        std::fs::write(sys.join("b.desktop"), "[Desktop Entry]\nName=Beta\nType=Application\nExec=beta\n").unwrap();
        std::fs::write(home.join("kde/c.desktop"), "[Desktop Entry]\nName=Gamma\nType=Application\nExec=gamma\n").unwrap();
        // the user's copy hides b; nxgs's copy of a is skipped but a is still found
        std::fs::write(home.join("b.desktop"), "[Desktop Entry]\nName=Beta\nType=Application\nExec=beta\nHidden=true\n").unwrap();
        std::fs::write(home.join("a.desktop"), make_copy("[Desktop Entry]\nName=Alpha\nType=Application\nExec=alpha %U\n", "nxgs run alpha", "alpha")).unwrap();
        let dirs = vec![home.clone(), sys.clone()];
        let apps = scan(&dirs, false);
        let names: Vec<_> = apps.iter().map(|a| (a.id.as_str(), a.name.as_str())).collect();
        assert_eq!(names, [("a.desktop", "Alpha"), ("kde-c.desktop", "Gamma")]);
        assert_eq!(apps[0].command, "alpha");
        assert!(scan(&dirs, true).iter().any(|a| a.ours));
        // find skips the copy, goes to the original
        assert_eq!(find(&dirs, "a.desktop").unwrap().path, sys.join("a.desktop"));
        assert_eq!(find(&dirs, "kde-c.desktop").unwrap().name, "Gamma");
        let _ = std::fs::remove_dir_all(&base);
    }
}
