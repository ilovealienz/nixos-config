//! Everything that isn't drawing: reading times, the tools' state, the
//! config, recent entries, window and app names, and running the tools.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// "5" "2.5" "1:30" "1:02:05" "90s" "1h30m" -> seconds; a plain number is
/// minutes. The same rules as the shell tools had.
pub fn duration(t: &str) -> Option<i64> {
    let t = t.trim();
    let num = |s: &str| s.parse::<i64>().ok();
    let parts: Vec<&str> = t.split(':').collect();
    let secs = match parts.len() {
        2 if parts[1].len() <= 2 => num(parts[0])? * 60 + num(parts[1])?,
        3 if parts[1].len() <= 2 && parts[2].len() <= 2 => num(parts[0])? * 3600 + num(parts[1])? * 60 + num(parts[2])?,
        1 if t.chars().all(|c| c.is_ascii_digit() || c == '.') && t.chars().filter(|&c| c == '.').count() <= 1 => {
            (t.parse::<f64>().ok()? * 60.0).round() as i64
        }
        1 => {
            let mut total = 0;
            let mut n = String::new();
            for c in t.chars() {
                match c {
                    '0'..='9' => n.push(c),
                    'h' | 'm' | 's' if !n.is_empty() => {
                        total += num(&n)? * match c {
                            'h' => 3600,
                            'm' => 60,
                            _ => 1,
                        };
                        n.clear();
                    }
                    _ => return None,
                }
            }
            if !n.is_empty() {
                return None;
            }
            total
        }
        _ => return None,
    };
    (secs > 0).then_some(secs)
}

/// "1h 30m tea time" -> ("1h30m", "tea time"): the time, then the rest
pub fn split_when(s: &str) -> (String, String) {
    let unit = |w: &str| !w.is_empty() && duration(w).is_some() && w.ends_with(['h', 'm', 's']);
    let mut words = s.split_whitespace().peekable();
    let mut when = words.next().unwrap_or("").to_string();
    while unit(&when) && words.peek().is_some_and(|w| unit(w)) {
        when.push_str(words.next().unwrap());
    }
    (when, words.collect::<Vec<_>>().join(" "))
}

/// 75 -> 1:15, 3725 -> 1:02:05
pub fn hms(s: i64) -> String {
    let s = s.max(0);
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, s % 3600 / 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

/// something running (or paused), from its tool's state file. left: what
/// the bar shows (time left; the stopwatch's time so far; until the
/// alarm); total: the timer's length or how often (repeat, drink); at:
/// the alarm's unix time.
#[derive(Debug, Clone, PartialEq)]
pub struct Running {
    pub tool: &'static str,
    pub paused: bool,
    pub left: i64,
    pub total: i64,
    pub at: i64,
    pub window: String,
    pub label: String,
    /// which one, for tools that run several at once ("1", "2", ...);
    /// empty for the others
    pub slot: String,
}

/// every tool, in the bar's order
pub const TOOLS: [&str; 5] = ["timer", "stopwatch", "alarm", "repeat", "drink"];
/// the tools that run several at once (`slots` in sway/msqu.nix)
pub const SLOTTED: [&str; 2] = ["timer", "alarm"];

/// where the tools keep their state: $XDG_RUNTIME_DIR/msqu/TOOL/
pub fn state_dir() -> PathBuf {
    PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR").unwrap_or_else(|| "/tmp".into())).join("msqu")
}

/// the state files:
///   timer      "run END TOTAL LABEL" / "pause LEFT TOTAL LABEL"
///   repeat     "run END EVERY WINDOW LABEL" / "pause LEFT EVERY WINDOW LABEL"
///   drink      "run END EVERY" / "pause LEFT EVERY"
///   stopwatch  "run START SO_FAR" / "pause SO_FAR"
///   alarm      "at UNIXTIME LABEL"
pub fn read_running(tool: &'static str, text: &str) -> Option<Running> {
    let now = now();
    let num = |s: Option<&str>| s?.parse::<i64>().ok();
    let mut r = Running { tool, paused: false, left: 0, total: 0, at: 0, window: "-".into(), label: String::new(), slot: String::new() };
    match tool {
        "stopwatch" => {
            let mut w = text.split_whitespace();
            match w.next()? {
                "run" => {
                    let start = num(w.next())?;
                    r.left = num(w.next())? + now - start;
                }
                "pause" => {
                    r.paused = true;
                    r.left = num(w.next())?;
                }
                _ => return None,
            }
        }
        "alarm" => {
            let mut w = text.trim().splitn(3, ' ');
            (w.next()? == "at").then_some(())?;
            r.at = num(w.next())?;
            r.left = r.at - now;
            r.label = w.next().unwrap_or("").to_string();
        }
        _ => {
            let n = match tool {
                "repeat" => 5,
                "timer" => 4,
                _ => 3,
            };
            let mut w = text.trim().splitn(n, ' ');
            let mode = w.next()?;
            let t = num(w.next())?;
            r.total = num(w.next())?;
            if tool == "repeat" {
                r.window = w.next()?.to_string();
            }
            if tool != "drink" {
                r.label = w.next().unwrap_or("").to_string();
            }
            (r.paused, r.left) = match mode {
                "run" => (false, t - now),
                "pause" => (true, t),
                _ => return None,
            };
        }
    }
    r.left = r.left.max(0);
    Some(r)
}

/// what's running, in the bar's order: a tool with slots has a state file
/// per slot ("1", "2", ...), the others one named after the tool
pub fn running() -> Vec<Running> {
    let mut out = Vec::new();
    for tool in TOOLS {
        let dir = state_dir().join(tool);
        let mut files: Vec<(String, String)> = match std::fs::read_dir(&dir) {
            Ok(rd) => rd
                .flatten()
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    let numbered = name.parse::<u32>().is_ok();
                    // (a tool with slots may have a file under its own name
                    // from before it had them: its alarm still rings)
                    if !(numbered || (name == tool && !SLOTTED.contains(&tool))) {
                        return None;
                    }
                    Some((name, std::fs::read_to_string(e.path()).ok()?))
                })
                .collect(),
            Err(_) => continue,
        };
        // slots in number order
        files.sort_by_key(|(n, _)| n.parse::<u32>().unwrap_or(0));
        for (name, text) in files {
            if let Some(mut r) = read_running(tool, &text) {
                if name != tool {
                    r.slot = name;
                }
                out.push(r);
            }
        }
    }
    out
}

/// "7:30" "19:45" "7" "7pm" "7:30pm" "7 pm" "12am" -> (hour, minute),
/// 24-hour; a bare hour is on the hour
pub fn clock(t: &str) -> Option<(u32, u32)> {
    let t = t.trim().to_lowercase().replace(' ', "");
    let (t, half) = if let Some(x) = t.strip_suffix("am") {
        (x, Some(0))
    } else if let Some(x) = t.strip_suffix("pm") {
        (x, Some(12))
    } else {
        (t.as_str(), None)
    };
    let (h, m) = match t.split_once(':') {
        Some((h, m)) if m.len() == 2 => (h, m),
        None => (t, "0"),
        Some(_) => return None,
    };
    if h.is_empty() || h.len() > 2 || !h.chars().chain(m.chars()).all(|c| c.is_ascii_digit()) {
        return None;
    }
    let (mut h, m): (u32, u32) = (h.parse().ok()?, m.parse().ok()?);
    if m > 59 {
        return None;
    }
    match half {
        Some(add) if (1..=12).contains(&h) => h = h % 12 + add,
        Some(_) => return None,
        None if h > 23 => return None,
        None => {}
    }
    Some((h, m))
}

/// "7:30 pm wake up" -> ((19, 30), "wake up"): a clock time, then the rest
pub fn split_clock(s: &str) -> Option<((u32, u32), String)> {
    let words: Vec<&str> = s.split_whitespace().collect();
    let first = *words.first()?;
    if let Some(&ap) = words.get(1).filter(|w| matches!(w.to_lowercase().as_str(), "am" | "pm")) {
        if let Some(c) = clock(&format!("{first}{ap}")) {
            return Some((c, words[2..].join(" ")));
        }
    }
    Some((clock(first)?, words[1..].join(" ")))
}

/// the next time the clock reads h:m (local time), as a unix time: today,
/// or tomorrow if that's passed
pub fn next_at(h: u32, m: u32) -> i64 {
    next_at_from(now(), h, m)
}

pub fn next_at_from(from: i64, h: u32, m: u32) -> i64 {
    // mktime works out the date (and DST) from a day and time
    let at = |day: i32| unsafe {
        let t: libc::time_t = from as libc::time_t;
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&t, &mut tm);
        tm.tm_mday += day;
        tm.tm_hour = h as i32;
        tm.tm_min = m as i32;
        tm.tm_sec = 0;
        tm.tm_isdst = -1;
        libc::mktime(&mut tm) as i64
    };
    let today = at(0);
    if today > from { today } else { at(1) }
}

/// unix time -> "07:30", or "Fri 07:30" if it isn't today
pub fn clock_text(at: i64) -> String {
    let parts = |t: i64| unsafe {
        let t: libc::time_t = t as libc::time_t;
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&t, &mut tm);
        (tm.tm_year, tm.tm_yday, tm.tm_wday, tm.tm_hour, tm.tm_min)
    };
    let (y, d, wd, h, m) = parts(at);
    let (ny, nd, ..) = parts(now());
    if (y, d) == (ny, nd) {
        format!("{h:02}:{m:02}")
    } else {
        let day = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"][wd.clamp(0, 6) as usize];
        format!("{day} {h:02}:{m:02}")
    }
}

/// seconds -> "3h 12m", "45m", "30s": roughly how long until something
pub fn rough(s: i64) -> String {
    let s = s.max(0);
    match s {
        0..60 => format!("{s}s"),
        60..3600 => format!("{}m", (s + 59) / 60),
        _ => {
            let m = (s + 59) / 60;
            if m % 60 == 0 { format!("{}h", m / 60) } else { format!("{}h {}m", m / 60, m % 60) }
        }
    }
}

/// a sum typed into the menu: "1024x8", "4+4", "(3+4)*2^3", "10/3",
/// "17%5" (remainder). x, × and * multiply; / and ÷ divide; ^ is a power.
/// None if it isn't one (a plain number, a time, words) or has no answer.
pub fn calc(s: &str) -> Option<f64> {
    let toks = calc_tokens(s)?;
    // a sum: some operator after the start (not just "-5" or "(5)")
    if !toks.iter().skip(1).any(|t| matches!(t, Tok::Op(_))) {
        return None;
    }
    let mut p = Calc { toks, at: 0 };
    let v = p.expr()?;
    (p.at == p.toks.len() && v.is_finite()).then_some(v)
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tok {
    Num(f64),
    Op(char),
    Open,
    Close,
}

fn calc_tokens(s: &str) -> Option<Vec<Tok>> {
    let mut out = Vec::new();
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        match c {
            ' ' => {}
            '0'..='9' | '.' => {
                // commas group thousands: 1,000
                let start = i;
                while i + 1 < cs.len() && (cs[i + 1].is_ascii_digit() || cs[i + 1] == '.' || cs[i + 1] == ',') {
                    i += 1;
                }
                let n: String = cs[start..=i].iter().filter(|&&c| c != ',').collect();
                out.push(Tok::Num(n.parse().ok()?));
            }
            '+' | '-' | '^' | '%' => out.push(Tok::Op(c)),
            '*' | 'x' | 'X' | '×' => out.push(Tok::Op('*')),
            '/' | '÷' => out.push(Tok::Op('/')),
            '(' => out.push(Tok::Open),
            ')' => out.push(Tok::Close),
            _ => return None,
        }
        i += 1;
    }
    Some(out)
}

struct Calc {
    toks: Vec<Tok>,
    at: usize,
}

impl Calc {
    fn peek(&self) -> Option<Tok> {
        self.toks.get(self.at).copied()
    }
    fn op(&mut self, ops: &[char]) -> Option<char> {
        match self.peek() {
            Some(Tok::Op(c)) if ops.contains(&c) => {
                self.at += 1;
                Some(c)
            }
            _ => None,
        }
    }
    // a + b - c
    fn expr(&mut self) -> Option<f64> {
        let mut v = self.term()?;
        while let Some(o) = self.op(&['+', '-']) {
            let r = self.term()?;
            v = if o == '+' { v + r } else { v - r };
        }
        Some(v)
    }
    // a * b / c % d
    fn term(&mut self) -> Option<f64> {
        let mut v = self.unary()?;
        while let Some(o) = self.op(&['*', '/', '%']) {
            let r = self.unary()?;
            v = match o {
                '*' => v * r,
                '/' => v / r,
                _ => v % r,
            };
        }
        Some(v)
    }
    // -a, +a: looser than ^, so -2^2 is -4
    fn unary(&mut self) -> Option<f64> {
        match self.op(&['-', '+']) {
            Some('-') => Some(-self.unary()?),
            Some(_) => self.unary(),
            None => self.power(),
        }
    }
    // a ^ b ^ c, from the right; the power can be negative (2^-1)
    fn power(&mut self) -> Option<f64> {
        let b = self.primary()?;
        if self.op(&['^']).is_some() {
            return Some(b.powf(self.unary()?));
        }
        Some(b)
    }
    fn primary(&mut self) -> Option<f64> {
        let t = self.peek()?;
        self.at += 1;
        match t {
            Tok::Num(n) => Some(n),
            Tok::Open => {
                let v = self.expr()?;
                (self.peek()? == Tok::Close).then(|| self.at += 1)?;
                Some(v)
            }
            _ => None,
        }
    }
}

/// a calc answer for showing and copying: 8192, 3.3333333333, 1.5e+20
pub fn calc_text(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else if v.abs() >= 1e15 || v.abs() < 1e-6 {
        format!("{v:e}")
    } else {
        let s = format!("{v:.10}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// a saved repeat: label, how often, window ("app_id=ID", "class=C" or -)
#[derive(Debug, Clone, PartialEq)]
pub struct Preset {
    pub label: String,
    pub every: String,
    pub window: String,
}

pub struct Config {
    pub colors: HashMap<String, String>,
    pub presets: Vec<Preset>,
    /// tools that run several at once: how many
    pub slots: HashMap<String, usize>,
}

/// the file sway/msqu.nix writes: "color NAME HEX",
/// "preset LABEL|EVERY|WINDOW" and "slots TOOL N" lines
pub fn parse_config(text: &str) -> Config {
    let mut c = Config { colors: HashMap::new(), presets: Vec::new(), slots: HashMap::new() };
    for line in text.lines() {
        if let Some((k, v)) = line.strip_prefix("slots ").and_then(|r| r.split_once(' ')) {
            if let Ok(n) = v.trim().parse() {
                c.slots.insert(k.to_string(), n);
            }
        } else if let Some(rest) = line.strip_prefix("color ") {
            if let Some((k, v)) = rest.split_once(' ') {
                c.colors.insert(k.to_string(), v.trim().to_string());
            }
        } else if let Some(rest) = line.strip_prefix("preset ") {
            let mut p = rest.splitn(3, '|');
            if let (Some(l), Some(e), Some(w)) = (p.next(), p.next(), p.next()) {
                c.presets.push(Preset { label: l.into(), every: e.into(), window: w.into() });
            }
        }
    }
    c
}

/// things started from the menu, newest first: "timer\tSECS\tLABEL",
/// "repeat\tSECS\tWINDOW\tLABEL", "alarm\tHH:MM\tLABEL", "drink\tSECS"
#[derive(Debug, Clone, PartialEq)]
pub enum Recent {
    Timer { secs: i64, label: String },
    Repeat { secs: i64, window: String, label: String },
    Alarm { h: u32, m: u32, label: String },
    Drink { secs: i64 },
}

fn recent_path() -> PathBuf {
    let base = std::env::var_os("XDG_STATE_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state"));
    base.join("msqu/recent")
}

pub fn parse_recent(text: &str) -> Vec<Recent> {
    text.lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            match f.as_slice() {
                ["timer", s, label] => Some(Recent::Timer { secs: s.parse().ok()?, label: label.to_string() }),
                ["repeat", s, w, label] => {
                    Some(Recent::Repeat { secs: s.parse().ok()?, window: w.to_string(), label: label.to_string() })
                }
                ["alarm", t, label] => {
                    let (h, m) = clock(t)?;
                    Some(Recent::Alarm { h, m, label: label.to_string() })
                }
                ["drink", s] => Some(Recent::Drink { secs: s.parse().ok()? }),
                _ => None,
            }
        })
        .collect()
}

fn recent_line(r: &Recent) -> String {
    match r {
        Recent::Timer { secs, label } => format!("timer\t{secs}\t{label}"),
        Recent::Repeat { secs, window, label } => format!("repeat\t{secs}\t{window}\t{label}"),
        Recent::Alarm { h, m, label } => format!("alarm\t{h:02}:{m:02}\t{label}"),
        Recent::Drink { secs } => format!("drink\t{secs}"),
    }
}

pub fn load_recent() -> Vec<Recent> {
    parse_recent(&std::fs::read_to_string(recent_path()).unwrap_or_default())
}

/// puts r first, without duplicates, keeping 8
pub fn remember(r: Recent) {
    let mut list = load_recent();
    list.retain(|x| x != &r);
    list.insert(0, r);
    list.truncate(8);
    save_recent(&list);
}

/// takes r out of the list
pub fn forget(r: &Recent) {
    let mut list = load_recent();
    list.retain(|x| x != r);
    save_recent(&list);
}

pub fn clear_recent() {
    let _ = std::fs::remove_file(recent_path());
}

fn save_recent(list: &[Recent]) {
    let path = recent_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let text: String = list.iter().map(|r| recent_line(r) + "\n").collect();
    let _ = std::fs::write(path, text);
}

/// app ID / X11 class -> the name in its .desktop file ("Sober")
pub fn app_names() -> HashMap<String, String> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(h) = std::env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
        dirs.push(PathBuf::from(h));
    } else if let Some(h) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(h).join(".local/share"));
    }
    let data = std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    dirs.extend(data.split(':').filter(|d| !d.is_empty()).map(PathBuf::from));
    let mut names = HashMap::new();
    // earlier directories win, as in the spec
    for d in dirs.iter().rev() {
        let Ok(rd) = std::fs::read_dir(d.join("applications")) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().is_none_or(|x| x != "desktop") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&p) else { continue };
            let (name, wmclass) = desktop_name(&text);
            let Some(name) = name else { continue };
            let stem = p.file_stem().unwrap_or_default().to_string_lossy().to_lowercase();
            names.insert(stem, name.clone());
            if let Some(w) = wmclass {
                names.insert(w.to_lowercase(), name);
            }
        }
    }
    names
}

/// Name= and StartupWMClass= from a .desktop file's main section
pub fn desktop_name(text: &str) -> (Option<String>, Option<String>) {
    let (mut name, mut wm, mut main) = (None, None, false);
    for l in text.lines() {
        let l = l.trim();
        if l.starts_with('[') {
            main = l == "[Desktop Entry]";
        } else if main {
            if let Some(v) = l.strip_prefix("Name=") {
                name.get_or_insert(v.to_string());
            } else if let Some(v) = l.strip_prefix("StartupWMClass=") {
                wm = Some(v.to_string());
            }
        }
    }
    (name, wm)
}

/// open windows, one per app: (WINDOW for repeat, app ID, title)
pub fn windows() -> Vec<(String, String, String)> {
    let out = Command::new("msqu-repeat").arg("windows").stderr(Stdio::null()).output();
    let text = out.map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
    text.lines()
        .filter_map(|l| {
            let (win, title) = l.split_once('\t')?;
            let id = win.split_once('=')?.1.to_string();
            Some((win.to_string(), id, title.to_string()))
        })
        .collect()
}

/// puts text on the clipboard. wl-copy stays behind to hand it out, in a
/// session of its own so it outlives the menu's terminal.
pub fn copy(text: &str) -> Result<(), String> {
    use std::io::Write;
    use std::os::unix::process::CommandExt;
    let mut cmd = Command::new("wl-copy");
    cmd.stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null());
    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    let mut child = cmd.spawn().map_err(|e| format!("can't run wl-copy: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
    }
    // (wl-copy hands the text to a copy of itself in the background, then
    // exits)
    if !child.wait().map_err(|e| e.to_string())?.success() {
        return Err("wl-copy couldn't take it (no Wayland session?)".into());
    }
    let _ = Command::new("notify-send").args(["-c", "osd", "-t", "1500", "copied", text]).stdin(Stdio::null()).status();
    Ok(())
}

/// runs msqu-TOOL with args; the error text if it failed
pub fn run(tool: &str, args: &[String]) -> Result<(), String> {
    let out = Command::new(format!("msqu-{tool}")).args(args).stdin(Stdio::null()).output();
    match out {
        Ok(o) if o.status.success() => Ok(()),
        Ok(o) => Err(String::from_utf8_lossy(&o.stderr).trim().to_string()),
        Err(e) => Err(format!("can't run msqu-{tool}: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations() {
        for (s, v) in [
            ("5", 300),
            ("2.5", 150),
            ("1:30", 90),
            ("1:02:05", 3725),
            ("90s", 90),
            ("1h30m", 5400),
            ("2m30s", 150),
            ("15m", 900),
        ] {
            assert_eq!(duration(s), Some(v), "{s}");
        }
        for s in ["", "0", "abc", "1x", "5m3", "1:300", "1.2.3", "h"] {
            assert_eq!(duration(s), None, "{s}");
        }
    }

    #[test]
    fn splitting() {
        assert_eq!(split_when("25 tea"), ("25".into(), "tea".into()));
        assert_eq!(split_when("1h 30m tea time"), ("1h30m".into(), "tea time".into()));
        assert_eq!(split_when("5 1h"), ("5".into(), "1h".into()));
        assert_eq!(split_when(""), ("".into(), "".into()));
    }

    #[test]
    fn state_files() {
        let r = read_running("repeat", "pause 300 900 app_id=org.vinegarhq.Sober roblox anti-AFK\n").unwrap();
        assert_eq!((r.paused, r.left, r.total, r.window.as_str(), r.label.as_str()), (true, 300, 900, "app_id=org.vinegarhq.Sober", "roblox anti-AFK"));
        let t = read_running("timer", &format!("run {} 600 tea time", now() + 100)).unwrap();
        assert!(!t.paused && (99..=100).contains(&t.left) && t.label == "tea time" && t.window == "-");
        assert_eq!(read_running("timer", "pause 5 5").unwrap().label, "");
        assert!(read_running("timer", "junk").is_none());
        let s = read_running("stopwatch", &format!("run {} 30", now() - 10)).unwrap();
        assert!(!s.paused && (40..=41).contains(&s.left));
        assert_eq!(read_running("stopwatch", "pause 75").unwrap().left, 75);
        let a = read_running("alarm", &format!("at {} wake up", now() + 3600)).unwrap();
        assert!(a.label == "wake up" && (3599..=3600).contains(&a.left) && a.at > now());
        let d = read_running("drink", "pause 120 2700").unwrap();
        assert_eq!((d.paused, d.left, d.total, d.label.as_str()), (true, 120, 2700, ""));
        assert!(read_running("alarm", "run 5").is_none());
    }

    #[test]
    fn sums() {
        for (q, v) in [
            ("4+4", "8"),
            ("1024x8", "8192"),
            ("1024 x 8", "8192"),
            ("1024*8", "8192"),
            ("10/4", "2.5"),
            ("10/3", "3.3333333333"),
            ("2^10", "1024"),
            ("2^3^2", "512"),
            ("(3+4)*2", "14"),
            ("-3+5", "2"),
            ("17%5", "2"),
            ("1.5×4", "6"),
            ("9÷3", "3"),
            ("100-150", "-50"),
            ("2^64", "1.8446744073709552e19"),
            ("-2^2", "-4"),
            ("2^-1", "0.5"),
            ("1,000x8", "8000"),
            ("0.1+0.2", "0.3"),
            ("2*-3", "-6"),
        ] {
            assert_eq!(calc(q).map(calc_text).as_deref(), Some(v), "{q}");
        }
        for q in ["25", "-5", "(5)", "1:30", "25 tea", "4+", "x4", "1/0", "2(3)", "a 7:30", "5m", "", "(1+2"] {
            assert_eq!(calc(q), None, "{q}");
        }
    }

    #[test]
    fn clocks() {
        for (s, v) in [("7:30", (7, 30)), ("19:45", (19, 45)), ("07:05", (7, 5)), ("7pm", (19, 0)), ("7:30pm", (19, 30)), ("7 PM", (19, 0)), ("12am", (0, 0)), ("12pm", (12, 0)), ("0:00", (0, 0)), ("7", (7, 0)), ("19", (19, 0))] {
            assert_eq!(clock(s), Some(v), "{s}");
        }
        for s in ["", "24", "25:00", "7:60", "13pm", "0am", "7:3", "abc", "1:30:00", "7:30x"] {
            assert_eq!(clock(s), None, "{s}");
        }
        assert_eq!(split_clock("7:30 pm wake up"), Some(((19, 30), "wake up".into())));
        assert_eq!(split_clock("6:45 gym"), Some(((6, 45), "gym".into())));
        assert_eq!(split_clock("gym 6:45"), None);
    }

    #[test]
    fn next_times() {
        let now = now();
        for (h, m) in [(0, 0), (7, 30), (23, 59), (12, 0)] {
            let at = next_at_from(now, h, m);
            assert!(at > now && at <= now + 86400 + 3600, "{h}:{m}");
            let t: libc::time_t = at as libc::time_t;
            let mut tm: libc::tm = unsafe { std::mem::zeroed() };
            unsafe { libc::localtime_r(&t, &mut tm) };
            assert_eq!((tm.tm_hour, tm.tm_min, tm.tm_sec), (h as i32, m as i32, 0));
        }
        assert_eq!(rough(30), "30s");
        assert_eq!(rough(61), "2m");
        assert_eq!(rough(3600), "1h");
        assert_eq!(rough(3 * 3600 + 700), "3h 12m");
    }

    #[test]
    fn config_and_recent() {
        let c = parse_config("color accent e5a440\npreset roblox anti-AFK|15m|app_id=org.vinegarhq.Sober\nslots timer 3\n");
        assert_eq!(c.slots["timer"], 3);
        assert_eq!(c.colors["accent"], "e5a440");
        assert_eq!(c.presets[0].window, "app_id=org.vinegarhq.Sober");
        let r = parse_recent("timer\t300\ttea\nrepeat\t900\t-\tstretch\nbad\nalarm\t07:30\twake\ndrink\t2700\n");
        assert_eq!(r.len(), 4);
        assert_eq!(recent_line(&r[2]), "alarm\t07:30\twake");
        assert_eq!(recent_line(&r[3]), "drink\t2700");
        assert_eq!(recent_line(&r[1]), "repeat\t900\t-\tstretch");
    }

    #[test]
    fn desktop_files() {
        let (n, w) = desktop_name("[Desktop Entry]\nName=Sober\nStartupWMClass=sober\n[Desktop Action x]\nName=Other\n");
        assert_eq!((n.as_deref(), w.as_deref()), (Some("Sober"), Some("sober")));
    }
}
