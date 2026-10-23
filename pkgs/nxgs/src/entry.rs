//! A saved entry: what to run and its gamescope settings. One file each in
//! ~/.config/nxgs/entries/, "key = value" lines.

use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Window {
    Fullscreen,
    Borderless,
    Windowed,
}

impl Window {
    pub const ALL: [Window; 3] = [Window::Fullscreen, Window::Borderless, Window::Windowed];

    pub fn word(self) -> &'static str {
        match self {
            Window::Fullscreen => "fullscreen",
            Window::Borderless => "borderless",
            Window::Windowed => "windowed",
        }
    }

    fn from_word(s: &str) -> Option<Window> {
        Window::ALL.into_iter().find(|w| w.word() == s)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Runs {
    /// a desktop file ID ("org.vinegarhq.Sober.desktop"), read when run
    App(String),
    /// a shell command
    Command(String),
    /// a Steam game: nxgs gives its launch option, Steam starts it
    Steam,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub runs: Runs,
    /// the sway output it opens on ("" for wherever the focus is)
    pub monitor: String,
    pub width: u32,
    pub height: u32,
    /// Hz, 0 for gamescope's default
    pub refresh: u32,
    pub window: Window,
    pub grab: bool,
    pub vrr: bool,
    /// frames per second, 0 for no limit
    pub fps: u32,
    /// frames per second while not focused, 0 for no limit
    pub unfocused_fps: u32,
    /// FSR upscaling from `render` percent of the size
    pub fsr: bool,
    pub render: u32,
    /// FSR sharpness, 0 (sharpest) to 20
    pub sharpness: u32,
    /// through gamemoderun
    pub gamemode: bool,
}

impl Entry {
    pub fn new(name: &str, runs: Runs) -> Entry {
        Entry {
            name: name.to_string(),
            runs,
            monitor: String::new(),
            width: 1920,
            height: 1080,
            refresh: 60,
            window: Window::Fullscreen,
            grab: false,
            vrr: false,
            fps: 0,
            unfocused_fps: 0,
            fsr: false,
            render: 75,
            sharpness: 5,
            gamemode: false,
        }
    }

    /// its file name and the name `nxgs run` takes: "The Sims 4" -> "the-sims-4"
    pub fn slug(&self) -> String {
        slug(&self.name)
    }

    pub fn to_text(&self) -> String {
        let yn = |b: bool| if b { "yes" } else { "no" };
        let (k, v) = match &self.runs {
            Runs::App(id) => ("app", id.as_str()),
            Runs::Command(c) => ("command", c.as_str()),
            Runs::Steam => ("for", "steam"),
        };
        format!(
            "name = {}\n{k} = {v}\nmonitor = {}\nwidth = {}\nheight = {}\nrefresh = {}\nwindow = {}\ngrab = {}\nvrr = {}\nfps = {}\nunfocused-fps = {}\nfsr = {}\nrender = {}\nsharpness = {}\ngamemode = {}\n",
            self.name,
            self.monitor,
            self.width,
            self.height,
            self.refresh,
            self.window.word(),
            yn(self.grab),
            yn(self.vrr),
            self.fps,
            self.unfocused_fps,
            yn(self.fsr),
            self.render,
            self.sharpness,
            yn(self.gamemode),
        )
    }

    /// None without a name and something to run; unknown keys are ignored,
    /// missing ones take the defaults
    pub fn from_text(text: &str) -> Option<Entry> {
        let mut e = Entry::new("", Runs::Command(String::new()));
        let mut runs = None;
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            let (k, v) = (k.trim(), v.trim());
            let num = |d: u32| v.parse().unwrap_or(d);
            match k {
                "name" => e.name = v.to_string(),
                "app" if !v.is_empty() => runs = Some(Runs::App(v.to_string())),
                "command" if !v.is_empty() => runs = Some(Runs::Command(v.to_string())),
                "for" if v == "steam" || v == "launcher" => runs = Some(Runs::Steam),
                "monitor" => e.monitor = v.to_string(),
                "width" => e.width = num(e.width).max(1),
                "height" => e.height = num(e.height).max(1),
                "refresh" => e.refresh = num(e.refresh),
                "window" => e.window = Window::from_word(v).unwrap_or(e.window),
                "grab" => e.grab = v == "yes",
                "vrr" => e.vrr = v == "yes",
                "fps" => e.fps = num(0),
                "unfocused-fps" => e.unfocused_fps = num(0),
                "fsr" => e.fsr = v == "yes",
                "render" => e.render = num(e.render).clamp(25, 100),
                "sharpness" => e.sharpness = num(e.sharpness).min(20),
                "gamemode" => e.gamemode = v == "yes",
                _ => {}
            }
        }
        e.runs = runs?;
        (!e.name.is_empty()).then_some(e)
    }

    /// gamescope's options, without the program
    pub fn gamescope_args(&self) -> Vec<String> {
        let mut a = vec![
            "-W".into(),
            self.width.to_string(),
            "-H".into(),
            self.height.to_string(),
        ];
        if self.refresh > 0 {
            a.extend(["-r".into(), self.refresh.to_string()]);
        }
        if self.fsr {
            let (w, h) = self.render_size();
            a.extend([
                "-w".into(),
                w.to_string(),
                "-h".into(),
                h.to_string(),
                "-F".into(),
                "fsr".into(),
                "--sharpness".into(),
                self.sharpness.to_string(),
            ]);
        }
        match self.window {
            Window::Fullscreen => a.push("-f".into()),
            Window::Borderless => a.push("-b".into()),
            Window::Windowed => {}
        }
        if self.grab {
            a.push("--force-grab-cursor".into());
        }
        if self.vrr {
            a.push("--adaptive-sync".into());
        }
        if self.fps > 0 {
            a.extend(["--framerate-limit".into(), self.fps.to_string()]);
        }
        if self.unfocused_fps > 0 {
            a.extend(["-o".into(), self.unfocused_fps.to_string()]);
        }
        a
    }

    /// the size the game renders at before FSR, even numbers
    pub fn render_size(&self) -> (u32, u32) {
        let f = |x: u32| ((x * self.render / 100) / 2 * 2).max(2);
        (f(self.width), f(self.height))
    }

    fn gamemode_prefix(&self) -> &'static str {
        if self.gamemode { "gamemoderun " } else { "" }
    }

    /// gamescope opens where the focus is: `nxgs on` moves it there first
    fn monitor_prefix(&self) -> String {
        if self.monitor.is_empty() {
            String::new()
        } else {
            format!("nxgs on {} ", self.monitor)
        }
    }

    /// Steam: Properties → Launch Options
    pub fn steam_option(&self) -> String {
        format!(
            "{}{}gamescope {} -- %command%",
            self.monitor_prefix(),
            self.gamemode_prefix(),
            self.gamescope_args().join(" ")
        )
    }

    /// Steam with GE-Proton: the game as a Wayland window, no gamescope
    pub fn proton_wayland_option(&self) -> String {
        let monitor = if self.monitor.is_empty() {
            String::new()
        } else {
            format!(" PROTON_WAYLAND_MONITOR={}", self.monitor)
        };
        format!(
            "PROTON_ENABLE_WAYLAND=1{monitor} {}%command%",
            self.gamemode_prefix()
        )
    }

    /// "1920×1080 · 144Hz · DP-1 · grab", for the list
    pub fn summary(&self) -> String {
        let mut parts = vec![format!("{}×{}", self.width, self.height)];
        if self.refresh > 0 {
            parts.push(format!("{}Hz", self.refresh));
        }
        if !self.monitor.is_empty() {
            parts.push(self.monitor.clone());
        }
        if self.window != Window::Fullscreen {
            parts.push(self.window.word().into());
        }
        if self.fsr {
            parts.push(format!("fsr {}%", self.render));
        }
        if self.grab {
            parts.push("grab".into());
        }
        if self.vrr {
            parts.push("vrr".into());
        }
        if self.fps > 0 {
            parts.push(format!("{}fps", self.fps));
        }
        parts.join(" · ")
    }
}

pub fn slug(name: &str) -> String {
    let mut s = String::new();
    for c in name.chars().flat_map(char::to_lowercase) {
        if c.is_alphanumeric() {
            s.push(c);
        } else if !s.ends_with('-') && !s.is_empty() {
            s.push('-');
        }
    }
    let s = s.trim_end_matches('-').to_string();
    if s.is_empty() { "entry".into() } else { s }
}

/// ~/.config/nxgs/entries
pub fn dir() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".config"));
    base.join("nxgs").join("entries")
}

pub fn home() -> PathBuf {
    PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
}

/// every readable entry, by name
pub fn load_all(dir: &Path) -> Vec<Entry> {
    let mut out: Vec<Entry> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "conf"))
        .filter_map(|e| Entry::from_text(&std::fs::read_to_string(e.path()).ok()?))
        .collect();
    out.sort_by_key(|e| e.name.to_lowercase());
    out
}

pub fn save(dir: &Path, e: &Entry) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(dir.join(format!("{}.conf", e.slug())), e.to_text())
}

pub fn remove(dir: &Path, e: &Entry) -> std::io::Result<()> {
    std::fs::remove_file(dir.join(format!("{}.conf", e.slug())))
}

/// by its slug or its name (any case)
pub fn find(dir: &Path, which: &str) -> Option<Entry> {
    let want = slug(which);
    load_all(dir).into_iter().find(|e| e.slug() == want)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sims() -> Entry {
        let mut e = Entry::new("The Sims 4", Runs::Command("steam".into()));
        e.monitor = "DP-1".into();
        e.refresh = 144;
        e.grab = true;
        e
    }

    #[test]
    fn round_trips() {
        let mut e = sims();
        e.fsr = true;
        e.render = 67;
        e.window = Window::Borderless;
        e.gamemode = true;
        assert_eq!(Entry::from_text(&e.to_text()), Some(e));
        let app = Entry::new("Sober", Runs::App("org.vinegarhq.Sober.desktop".into()));
        assert_eq!(Entry::from_text(&app.to_text()), Some(app));
    }

    #[test]
    fn reads_partial_and_rejects_empty() {
        let e = Entry::from_text("name = x\ncommand = y\nwidth = 1280\nbad line\nfps = nope\n").unwrap();
        assert_eq!((e.width, e.height, e.fps), (1280, 1080, 0));
        assert_eq!(Entry::from_text("name = x\n"), None);
        assert_eq!(Entry::from_text("command = y\n"), None);
    }

    #[test]
    fn builds_options() {
        let e = sims();
        assert_eq!(
            e.steam_option(),
            "nxgs on DP-1 gamescope -W 1920 -H 1080 -r 144 -f --force-grab-cursor -- %command%"
        );
        assert_eq!(
            e.proton_wayland_option(),
            "PROTON_ENABLE_WAYLAND=1 PROTON_WAYLAND_MONITOR=DP-1 %command%"
        );
        let mut f = sims();
        f.fsr = true;
        f.render = 67;
        f.window = Window::Windowed;
        f.grab = false;
        f.fps = 60;
        f.gamemode = true;
        assert_eq!(
            f.steam_option(),
            "nxgs on DP-1 gamemoderun gamescope -W 1920 -H 1080 -r 144 -w 1286 -h 722 -F fsr --sharpness 5 --framerate-limit 60 -- %command%"
        );
        assert_eq!(f.summary(), "1920×1080 · 144Hz · DP-1 · windowed · fsr 67% · 60fps");
    }

    #[test]
    fn slugs() {
        assert_eq!(slug("The Sims 4"), "the-sims-4");
        assert_eq!(slug("  Roblox (Sober)! "), "roblox-sober");
        assert_eq!(slug("???"), "entry");
    }

    #[test]
    fn saves_finds_removes() {
        let dir = std::env::temp_dir().join(format!("nxgs-test-{}", std::process::id()));
        let e = sims();
        save(&dir, &e).unwrap();
        assert_eq!(find(&dir, "the sims 4"), Some(e.clone()));
        assert_eq!(find(&dir, "the-sims-4").map(|e| e.name), Some("The Sims 4".into()));
        assert_eq!(load_all(&dir).len(), 1);
        remove(&dir, &e).unwrap();
        assert!(load_all(&dir).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
