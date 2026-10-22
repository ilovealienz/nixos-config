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

/// something running or paused, from its state file. left: what the bar
/// shows; total: length or interval; at: the alarm's unix time.
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
    /// clock: the time zone ("Asia/Tokyo")
    pub zone: String,
    /// countdown: a time of day was given, not just a date
    pub timed: bool,
}

/// every tool with running ones, in the bar's order (notes have a block of
/// their own, besides these)
pub const TOOLS: [&str; 7] = ["timer", "stopwatch", "alarm", "repeat", "drink", "countdown", "clock"];
/// the tools that run several at once (`slots` in desktop/msqu.nix)
pub const SLOTTED: [&str; 4] = ["timer", "alarm", "countdown", "clock"];
/// the tools whose state outlives a reboot
pub const LASTING: [&str; 3] = ["countdown", "clock", "notes"];

/// where the tools keep their state: $XDG_RUNTIME_DIR/msqu/TOOL/ (/tmp if
/// it's unset or empty, like the scripts' ${XDG_RUNTIME_DIR:-/tmp})
pub fn state_dir() -> PathBuf {
    let run = std::env::var_os("XDG_RUNTIME_DIR").filter(|v| !v.is_empty());
    PathBuf::from(run.unwrap_or_else(|| "/tmp".into())).join("msqu")
}

/// what outlives a reboot: $XDG_STATE_HOME/msqu (~/.local/state/msqu)
pub fn lasting_dir() -> PathBuf {
    let base = std::env::var_os("XDG_STATE_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state"));
    base.join("msqu")
}

/// a tool's state files: countdowns, clocks and notes last, the rest go
/// with the session
pub fn tool_dir(tool: &str) -> PathBuf {
    if LASTING.contains(&tool) { lasting_dir().join(tool) } else { state_dir().join(tool) }
}

/// the state files:
///   timer      "run END TOTAL LABEL" / "pause LEFT TOTAL LABEL"
///   repeat     "run END EVERY WINDOW LABEL" / "pause LEFT EVERY WINDOW LABEL"
///   drink      "run END EVERY" / "pause LEFT EVERY"
///   stopwatch  "run START SO_FAR" / "pause SO_FAR"
///   alarm      "at UNIXTIME LABEL"
///   countdown  "at UNIXTIME TIMED NOTIFIED LABEL" (TIMED, NOTIFIED: 0 or 1)
///   clock      "zone ZONE LABEL"
pub fn read_running(tool: &'static str, text: &str) -> Option<Running> {
    let now = now();
    let num = |s: Option<&str>| s?.parse::<i64>().ok();
    let mut r = Running {
        tool,
        paused: false,
        left: 0,
        total: 0,
        at: 0,
        window: "-".into(),
        label: String::new(),
        slot: String::new(),
        zone: String::new(),
        timed: false,
    };
    match tool {
        "countdown" => {
            let mut w = text.trim().splitn(5, ' ');
            (w.next()? == "at").then_some(())?;
            r.at = num(w.next())?;
            r.timed = w.next()? == "1";
            w.next()?;
            r.label = w.next().unwrap_or("").to_string();
            r.left = r.at - now;
        }
        "clock" => {
            let mut w = text.trim().splitn(3, ' ');
            (w.next()? == "zone").then_some(())?;
            r.zone = w.next()?.to_string();
            r.label = w.next().unwrap_or("").to_string();
        }
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
        let dir = tool_dir(tool);
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

unsafe extern "C" {
    // (not in the libc crate) rereads TZ
    fn tzset();
}

pub fn local_tm(at: i64) -> libc::tm {
    let t = at as libc::time_t;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    unsafe { libc::localtime_r(&t, &mut tm) };
    tm
}

/// y-m-d h:mi in local time as a unix time; None if there's no such day
pub fn local_time(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> Option<i64> {
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    tm.tm_year = y - 1900;
    tm.tm_mon = mo as i32 - 1;
    tm.tm_mday = d as i32;
    tm.tm_hour = h as i32;
    tm.tm_min = mi as i32;
    tm.tm_isdst = -1;
    let at = unsafe { libc::mktime(&mut tm) } as i64;
    // mktime moves a day that doesn't exist (31/2) on: refuse those
    (tm.tm_year == y - 1900 && tm.tm_mon == mo as i32 - 1 && tm.tm_mday == d as i32).then_some(at)
}

/// a unix time's local date: (year, month, day)
pub fn ymd(at: i64) -> (i32, u32, u32) {
    let tm = local_tm(at);
    (tm.tm_year + 1900, tm.tm_mon as u32 + 1, tm.tm_mday as u32)
}

/// calendar days from one time's date to another's (0: the same day)
pub fn days_between(from: i64, to: i64) -> i64 {
    let midnight = |t: i64| {
        let (y, m, d) = ymd(t);
        local_time(y, m, d, 0, 0).unwrap_or(t)
    };
    // rounded: a day with a clock change isn't 86400s
    ((midnight(to) - midnight(from)) as f64 / 86400.0).round() as i64
}

/// just after the midnight that ends `at`'s day
pub fn day_after(at: i64) -> i64 {
    let (y, m, d) = ymd(at);
    let start = local_time(y, m, d, 0, 0).unwrap_or(at);
    // 30h on: into the next day whatever the clocks do
    let (y, m, d) = ymd(start + 30 * 3600);
    local_time(y, m, d, 0, 0).unwrap_or(start + 86400) + 1
}

/// "25/12" "25/12/26" "25/12/2026" -> (y, m, d); without a year, the next
/// time that date comes round (today included)
pub fn date(s: &str) -> Option<(i32, u32, u32)> {
    let parts: Vec<&str> = s.split('/').collect();
    if !(2..=3).contains(&parts.len()) || parts.iter().any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit())) {
        return None;
    }
    let (d, m): (u32, u32) = (parts[0].parse().ok()?, parts[1].parse().ok()?);
    if parts[0].len() > 2 || parts[1].len() > 2 {
        return None;
    }
    let (this_year, ..) = ymd(now());
    let y = match parts.get(2) {
        Some(y) if y.len() == 2 => 2000 + y.parse::<i32>().ok()?,
        Some(y) if y.len() == 4 => y.parse().ok()?,
        Some(_) => return None,
        None => {
            let this = local_time(this_year, m, d, 0, 0);
            // 29/02 in a year without one: the next leap year's
            let mut y = this_year;
            if this.is_none_or(|t| days_between(now(), t) < 0) {
                y += 1;
                while local_time(y, m, d, 0, 0).is_none() && y < this_year + 8 {
                    y += 1;
                }
            }
            y
        }
    };
    local_time(y, m, d, 0, 0).map(|_| (y, m, d))
}

/// "xmas 25/12", "gig 14/11/2026 19:30", "25/12 7pm party" -> (unix time,
/// whether a time was given, label); None without a date still to come
pub fn parse_countdown(s: &str) -> Option<(i64, bool, String)> {
    let words: Vec<&str> = s.split_whitespace().collect();
    let i = words.iter().position(|w| date(w).is_some())?;
    let (y, m, d) = date(words[i])?;
    let mut used = vec![i];
    let mut hm = None;
    if let Some(next) = words.get(i + 1) {
        let ampm = words.get(i + 2).filter(|w| matches!(w.to_lowercase().as_str(), "am" | "pm"));
        if let Some(c) = ampm.and_then(|ap| clock(&format!("{next}{ap}"))) {
            hm = Some(c);
            used.extend([i + 1, i + 2]);
        } else if next.contains(':') || next.to_lowercase().ends_with("am") || next.to_lowercase().ends_with("pm") {
            if let Some(c) = clock(next) {
                hm = Some(c);
                used.push(i + 1);
            }
        }
    }
    let (h, mi) = hm.unwrap_or((0, 0));
    let at = local_time(y, m, d, h, mi)?;
    let gone = if hm.is_some() { at <= now() } else { days_between(now(), at) < 0 };
    if gone {
        return None;
    }
    let label = words.iter().enumerate().filter(|(n, _)| !used.contains(n)).map(|(_, w)| *w).collect::<Vec<_>>().join(" ");
    Some((at, hm.is_some(), label))
}

/// what a countdown's block says: "63d", "1d", "5h", "40m", "today"; None
/// once its day is over
pub fn countdown_text(at: i64, timed: bool) -> Option<String> {
    let now = now();
    let days = days_between(now, at);
    if days < 0 {
        return None;
    }
    let left = at - now;
    if timed && left > 0 && left < 86400 {
        return Some(if left >= 3600 { format!("{}h", left / 3600) } else { format!("{}m", (left + 59) / 60) });
    }
    Some(if days == 0 { "today".into() } else { format!("{days}d") })
}

/// unix time -> "Thu 25 Dec", with the year if it isn't this one
pub fn date_text(at: i64) -> String {
    let tm = local_tm(at);
    let day = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"][tm.tm_wday.clamp(0, 6) as usize];
    let mon = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"][tm.tm_mon.clamp(0, 11) as usize];
    let year = if tm.tm_year == local_tm(now()).tm_year { String::new() } else { format!(" {}", tm.tm_year + 1900) };
    format!("{day} {} {mon}{year}", tm.tm_mday)
}

/// the time zone files: $TZDIR, /etc/zoneinfo or /usr/share/zoneinfo
pub fn zoneinfo() -> PathBuf {
    std::env::var_os("TZDIR")
        .map(PathBuf::from)
        .into_iter()
        .chain(["/etc/zoneinfo".into(), "/usr/share/zoneinfo".into()])
        .find(|d| d.join("Europe").is_dir())
        .unwrap_or_else(|| "/usr/share/zoneinfo".into())
}

/// every "Region/City" zone, and UTC
pub fn zones() -> Vec<String> {
    let root = zoneinfo();
    let mut out = vec!["UTC".to_string()];
    let mut todo = vec![PathBuf::new()];
    while let Some(rel) = todo.pop() {
        let Ok(rd) = std::fs::read_dir(root.join(&rel)) else { continue };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            // regions start with a capital; skip the posix/right copies and Etc
            if rel.as_os_str().is_empty() && (!name.starts_with(|c: char| c.is_ascii_uppercase()) || name == "Etc") {
                continue;
            }
            let path = rel.join(&name);
            match e.file_type() {
                Ok(t) if t.is_dir() => todo.push(path),
                Ok(_) if !rel.as_os_str().is_empty() => out.push(path.to_string_lossy().into_owned()),
                _ => {}
            }
        }
    }
    out.sort();
    out
}

/// "tokyo", "new york", "la paz" -> the best matching zones (up to 5)
pub fn find_zones(all: &[String], q: &str) -> Vec<String> {
    let q = q.trim().to_lowercase().replace([' ', '-'], "_");
    if q.is_empty() {
        return Vec::new();
    }
    let mut hits: Vec<(u8, &String)> = all
        .iter()
        .filter_map(|z| {
            let full = z.to_lowercase();
            let city = full.rsplit('/').next().unwrap_or(&full);
            let rank = if city == q {
                0
            } else if city.starts_with(&q) {
                1
            } else if city.contains(&q) {
                2
            } else if full.contains(&q) {
                3
            } else {
                return None;
            };
            Some((rank, z))
        })
        .collect();
    hits.sort();
    hits.into_iter().take(5).map(|(_, z)| z.clone()).collect()
}

/// "America/New_York" -> "New York"
pub fn zone_city(zone: &str) -> String {
    zone.rsplit('/').next().unwrap_or(zone).replace('_', " ")
}

/// the time in a zone: (hour, minute, weekday 0-6, offset from UTC in
/// seconds); None if there's no such zone
pub fn zone_time(zone: &str, at: i64) -> Option<(u32, u32, u32, i64)> {
    let file = zoneinfo().join(zone);
    if zone.contains("..") || !file.is_file() {
        return None;
    }
    let old = std::env::var_os("TZ");
    // glibc reads TZ=":/path/to/zone" as that file
    unsafe { std::env::set_var("TZ", format!(":{}", file.display())) };
    unsafe { tzset() };
    let tm = local_tm(at);
    match old {
        Some(v) => unsafe { std::env::set_var("TZ", v) },
        None => unsafe { std::env::remove_var("TZ") },
    }
    unsafe { tzset() };
    Some((tm.tm_hour as u32, tm.tm_min as u32, tm.tm_wday as u32, tm.tm_gmtoff))
}

/// a zone's offset from here: "+9h", "-5h", "+5h 30m", "same time"
pub fn offset_text(zone_off: i64) -> String {
    let d = zone_off - local_tm(now()).tm_gmtoff;
    if d == 0 {
        return "same time".into();
    }
    let sign = if d > 0 { "+" } else { "-" };
    let (h, m) = (d.abs() / 3600, d.abs() % 3600 / 60);
    if m == 0 { format!("{sign}{h}h") } else { format!("{sign}{h}h {m}m") }
}

/// a note: its number, reminder time (0: none) and text
#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub id: u64,
    pub at: i64,
    pub text: String,
}

/// the notes file: "ID\tAT\tTEXT" lines
pub fn notes_file() -> PathBuf {
    tool_dir("notes").join("notes")
}

pub fn parse_notes(text: &str) -> Vec<Note> {
    text.lines()
        .filter_map(|l| {
            let mut f = l.splitn(3, '\t');
            Some(Note { id: f.next()?.parse().ok()?, at: f.next()?.parse().ok()?, text: f.next()?.to_string() })
        })
        .collect()
}

pub fn load_notes() -> Vec<Note> {
    parse_notes(&std::fs::read_to_string(notes_file()).unwrap_or_default())
}

/// "call mum 6pm", "bins at 7:30" -> ("call mum", reminder): a time at the
/// end is a reminder (with a colon or am/pm, so "chapter 7" stays a note)
pub fn split_note(s: &str) -> (String, Option<i64>) {
    let (text, at) = split_note_time(s);
    match (text.strip_suffix(" at"), at) {
        (Some(t), Some(at)) => (t.to_string(), Some(at)),
        _ => (text, at),
    }
}

fn split_note_time(s: &str) -> (String, Option<i64>) {
    let words: Vec<&str> = s.split_whitespace().collect();
    let is_time = |w: &str| {
        let l = w.to_lowercase();
        (l.contains(':') || l.ends_with("am") || l.ends_with("pm")) && clock(w).is_some()
    };
    let n = words.len();
    if n >= 3 && matches!(words[n - 1].to_lowercase().as_str(), "am" | "pm") {
        if let Some((h, m)) = clock(&format!("{}{}", words[n - 2], words[n - 1])) {
            return (words[..n - 2].join(" "), Some(next_at(h, m)));
        }
    }
    if n >= 2 && is_time(words[n - 1]) {
        if let Some((h, m)) = clock(words[n - 1]) {
            return (words[..n - 1].join(" "), Some(next_at(h, m)));
        }
    }
    (words.join(" "), None)
}

/// a sum typed into the menu ("1024x8", "(3+4)*2^3", "17%5"); None if it
/// isn't one or has no answer.
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

/// the file desktop/msqu.nix writes: "color NAME HEX",
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
/// "repeat\tSECS\tWINDOW\tLABEL", "alarm\tHH:MM\tLABEL", "drink\tSECS",
/// "clock\tZONE\tLABEL"
#[derive(Debug, Clone, PartialEq)]
pub enum Recent {
    Timer { secs: i64, label: String },
    Repeat { secs: i64, window: String, label: String },
    Alarm { h: u32, m: u32, label: String },
    Drink { secs: i64 },
    Clock { zone: String, label: String },
}

fn recent_path() -> PathBuf {
    lasting_dir().join("recent")
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
                ["clock", z, label] => Some(Recent::Clock { zone: z.to_string(), label: label.to_string() }),
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
        Recent::Clock { zone, label } => format!("clock\t{zone}\t{label}"),
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

/// how msqu shows on the bar: a block each, or one block (`one`) with the
/// pinned ones beside it; and the hidden ones. The file: "mode one",
/// "pin TOOL NAME" and "hide TOOL NAME" lines (NAME: the slot, or the
/// tool's own name)
#[derive(Debug, Default, PartialEq)]
pub struct Bar {
    pub one: bool,
    pub marks: Vec<(String, String, String)>,
}

impl Bar {
    pub fn has(&self, kind: &str, tool: &str, name: &str) -> bool {
        self.marks.iter().any(|(k, t, n)| k == kind && t == tool && n == name)
    }

    /// it gets a block of its own
    pub fn shown(&self, tool: &str, name: &str) -> bool {
        !self.has("hide", tool, name) && (!self.one || self.has("pin", tool, name))
    }
}

/// a running one's NAME in the bar file
pub fn bar_name(r: &Running) -> String {
    if r.slot.is_empty() { r.tool.to_string() } else { r.slot.clone() }
}

pub fn bar_file() -> PathBuf {
    lasting_dir().join("bar")
}

pub fn parse_bar(text: &str) -> Bar {
    let mut b = Bar::default();
    for l in text.lines() {
        match l.split_whitespace().collect::<Vec<_>>().as_slice() {
            ["mode", "one"] => b.one = true,
            [k @ ("pin" | "hide"), t, n] => b.marks.push((k.to_string(), t.to_string(), n.to_string())),
            _ => {}
        }
    }
    b
}

pub fn load_bar() -> Bar {
    parse_bar(&std::fs::read_to_string(bar_file()).unwrap_or_default())
}

/// written in one go, so a block never reads half of it
fn save_bar(b: &Bar) {
    let path = bar_file();
    let _ = std::fs::create_dir_all(lasting_dir());
    let mut text = if b.one { "mode one\n".to_string() } else { String::new() };
    for (k, t, n) in &b.marks {
        text += &format!("{k} {t} {n}\n");
    }
    let tmp = lasting_dir().join(".bar.tmp");
    if std::fs::write(&tmp, text).is_ok() {
        let _ = std::fs::rename(&tmp, path);
    }
}

pub fn set_bar_one(on: bool) {
    let mut b = load_bar();
    b.one = on;
    save_bar(&b);
}

/// pins or hides (`kind`) one, or stops
pub fn set_mark(kind: &str, tool: &str, name: &str, on: bool) {
    let mut b = load_bar();
    if b.has(kind, tool, name) == on {
        return;
    }
    b.marks.retain(|(k, t, n)| !(k == kind && t == tool && n == name));
    if on {
        b.marks.push((kind.into(), tool.into(), name.into()));
    }
    save_bar(&b);
}

/// it's gone: no longer pinned or hidden, so the next in its slot isn't
pub fn unmark(tool: &str, name: &str) {
    let mut b = load_bar();
    let n = b.marks.len();
    b.marks.retain(|(_, t, m)| !(t == tool && m == name));
    if b.marks.len() != n {
        save_bar(&b);
    }
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

    #[test]
    fn dates_and_countdowns() {
        let (y, m, d) = ymd(now());
        // without a year: the next time it comes round, today included
        assert_eq!(date(&format!("{d}/{m}")), Some((y, m, d)));
        assert_eq!(date("31/2"), None);
        assert_eq!(date("25/12/26").map(|t| t.0), Some(2026));
        assert_eq!(date("chapter"), None);
        let (at, timed, label) = parse_countdown(&format!("gig {d}/{m}/{} 23:59 at the hall", y + 1)).unwrap();
        assert_eq!((timed, label.as_str()), (true, "gig at the hall"));
        assert_eq!(days_between(now(), at), days_between(now(), local_time(y + 1, m, d, 12, 0).unwrap()));
        let (_, timed, label) = parse_countdown(&format!("xmas 25/12/{} 7 pm", y + 1)).unwrap();
        assert_eq!((timed, label.as_str()), (true, "xmas"));
        assert_eq!(parse_countdown(&format!("1/1/{}", y - 1)), None);
        assert_eq!(parse_countdown("no date"), None);
        // today, with no time: still to come all day
        let (today, ..) = parse_countdown(&format!("{d}/{m}")).unwrap();
        assert_eq!(countdown_text(today, false).as_deref(), Some("today"));
        assert_eq!(countdown_text(now() + 3 * 86400 + 60, false).as_deref(), Some("3d"));
        assert_eq!(countdown_text(now() + 90 * 60, true).as_deref().map(|s| s.ends_with('h') || s.ends_with('m')), Some(true));
        assert_eq!(countdown_text(now() - 3 * 86400, false), None);
        let after = day_after(now());
        assert!(after > now() && after - now() <= 25 * 3600 && days_between(now(), after) == 1);
    }

    #[test]
    fn zones_and_notes() {
        let all = vec!["America/New_York".to_string(), "Asia/Tokyo".into(), "Europe/London".into(), "UTC".into()];
        assert_eq!(find_zones(&all, "tokyo"), ["Asia/Tokyo"]);
        assert_eq!(find_zones(&all, "new york"), ["America/New_York"]);
        assert_eq!(find_zones(&all, "europe"), ["Europe/London"]);
        assert!(find_zones(&all, "").is_empty());
        assert_eq!(zone_city("America/New_York"), "New York");
        if zoneinfo().join("Asia/Tokyo").is_file() {
            // Tokyo has no summer time: 9 hours ahead of UTC
            let (h, m, _, off) = zone_time("Asia/Tokyo", 0).unwrap();
            assert_eq!((h, m, off), (9, 0, 32400));
            assert!(zones().iter().any(|z| z == "Europe/London"));
        }
        assert_eq!(zone_time("../../etc/passwd", 0), None);
        assert_eq!(split_note("call mum 6pm").0, "call mum");
        assert!(split_note("call mum 6 pm").1.is_some());
        assert_eq!(split_note("bins 7:30").0, "bins");
        assert_eq!(split_note("call mum at 6pm").0, "call mum");
        assert_eq!(split_note("look at"), ("look at".into(), None));
        assert_eq!(split_note("read chapter 7"), ("read chapter 7".into(), None));
        assert_eq!(split_note("7pm").1, None);
        assert_eq!(
            parse_notes("1\t0\tmilk\n2\t99\tcall\tmum\nbad\n"),
            [Note { id: 1, at: 0, text: "milk".into() }, Note { id: 2, at: 99, text: "call\tmum".into() }]
        );
        assert_eq!(parse_recent("clock\tAsia/Tokyo\tTokyo\n"), [Recent::Clock { zone: "Asia/Tokyo".into(), label: "Tokyo".into() }]);
        let b = parse_bar("mode one\npin timer 1\nhide clock 2\nbad line\n");
        assert!(b.one && b.shown("timer", "1") && !b.shown("timer", "2") && !b.shown("clock", "2"));
        let each = parse_bar("hide clock 2\npin timer 1\n");
        assert!(!each.one && each.shown("timer", "2") && !each.shown("clock", "2"));
    }
}
