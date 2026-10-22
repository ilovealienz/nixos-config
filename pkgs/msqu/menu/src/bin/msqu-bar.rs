//! msqu-bar TOOL [SLOT]: one waybar block, printed on change. Wakes on state
//! file changes (inotify), each second while counting (each minute for the
//! clocks), and at midnight. TOOL is one of model::TOOLS, or notes.
//! Classes: "msqu", its colour, "paused".
//!
//! msqu-bar all: the one block, when the menu has the bar set to that: what
//! isn't pinned or hidden, as icons, and listed on hover.
//!
//! msqu-bar --set one|each: sets the bar to one block, or a block each.

use msqu_ui::model::{self, Running, clock_text, countdown_text, date_text, hms};
use std::ffi::CString;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;

const TIMER: &str = "󰔛";
const REPEAT: &str = "󰑖";
const STOPWATCH: &str = "󰁫";
const ALARM: &str = "󰀠";
const DRINK: &str = "󰆫";
const PAUSED: &str = "󰏤";
const COUNTDOWN: &str = "󰃭";
const CLOCK: &str = "󰥔";
const REMINDER: &str = "󰂞";
const MSQU: &str = "󰀻";

/// the tooltip's last line: what the clicks do
fn clicks(middle: &str) -> String {
    format!("\nclick: msqu · right-click: options · middle-click: {middle}")
}

/// a JSON string
fn json(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            c if (c as u32) < 0x20 => o.push(' '),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// waybar reads text and tooltips as Pango markup: & < > escaped
fn pango(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

const EMPTY: &str = "{\"text\":\"\"}";

struct Block {
    text: String,
    colour: &'static str,
    tooltip: String,
    paused: bool,
}

impl Block {
    fn json(&self) -> String {
        format!(
            "{{\"text\":{},\"class\":[\"msqu\",\"{}\"{}],\"tooltip\":{}}}",
            json(&pango(&self.text)),
            self.colour,
            if self.paused { ",\"paused\"" } else { "" },
            json(&pango(&self.tooltip))
        )
    }
}

fn block(text: &str, colour: &'static str, tooltip: &str, paused: bool) -> Block {
    Block { text: text.into(), colour, tooltip: tooltip.into(), paused }
}

/// the block for what's in the state file, as JSON
pub fn render(tool: &str, r: Option<&Running>) -> String {
    show(tool, r).map_or(EMPTY.into(), |b| b.json())
}

/// the block for what's in the state file (None: nothing to show)
fn show(tool: &str, r: Option<&Running>) -> Option<Block> {
    let r = r?;
    let label = |sep: &str| if r.label.is_empty() { String::new() } else { format!("{}{sep}", r.label) };
    Some(match tool {
        "timer" => block(
            &format!("{} {}", if r.paused { PAUSED } else { TIMER }, hms(r.left)),
            "yellow",
            &format!("{}{} of {}{}", label(": "), hms(r.left), hms(r.total), clicks("cancel")),
            r.paused,
        ),
        "stopwatch" => block(&format!("{STOPWATCH} {}", hms(r.left)), "green", &format!("stopwatch{}{}", if r.paused { " (paused)" } else { "" }, clicks("stop")), r.paused),
        "alarm" => {
            let when = clock_text(r.at);
            block(&format!("{ALARM} {when}"), "orange", &format!("{}{when}{}", label(": "), clicks("cancel")), false)
        }
        "countdown" => {
            let left = countdown_text(r.at, r.timed)?;
            let time = if r.timed { format!(" {}", msqu_ui::tool::hm(r.at)) } else { String::new() };
            block(
                &format!("{COUNTDOWN} {}{left}", label(" ")),
                "accent",
                &format!("{}{}{time}{}", label(": "), date_text(r.at), clicks("remove")),
                false,
            )
        }
        "clock" => {
            let now = model::now();
            let (h, m, _, off) = model::zone_time(&r.zone, now)?;
            let name = if r.label.is_empty() { model::zone_city(&r.zone) } else { r.label.clone() };
            block(
                &format!("{CLOCK} {name} {h:02}:{m:02}"),
                "fg",
                &format!("{}, {}{}", r.zone, model::offset_text(off), clicks("remove")),
                false,
            )
        }
        "repeat" => {
            // "app_id=org.vinegarhq.Sober" -> "org.vinegarhq.Sober"
            let win = match r.window.split_once('=') {
                Some((_, id)) => format!(", brings up {id}"),
                None => String::new(),
            };
            block(
                &format!("{} {}", if r.paused { PAUSED } else { REPEAT }, hms(r.left)),
                "blue",
                &format!("{}every {}{win}\nnext in {}{}", label(": "), hms(r.total), hms(r.left), clicks("stop")),
                r.paused,
            )
        }
        // drink: whole minutes, rounded up ("1m" until it goes off)
        _ => block(
            &format!("{DRINK} {}m", (r.left + 59) / 60),
            "magenta",
            &format!("drink water every {}\nnext in {}{}", hms(r.total), hms(r.left), clicks("stop")),
            r.paused,
        ),
    })
}

/// the notes block, as JSON
pub fn render_notes(text: &str) -> String {
    show_notes(text).map_or(EMPTY.into(), |b| b.json())
}

/// the notes block: the next reminder, None without one
fn show_notes(text: &str) -> Option<Block> {
    let mut due: Vec<model::Note> = model::parse_notes(text).into_iter().filter(|n| n.at > 0).collect();
    due.sort_by_key(|n| n.at);
    let next = due.first()?;
    let more = if due.len() > 1 { format!(" +{}", due.len() - 1) } else { String::new() };
    let list: Vec<String> = due.iter().map(|n| format!("{}  {}", clock_text(n.at), n.text)).collect();
    let tip = format!("{}{}", list.join("\n"), clicks("cancel the next reminder"));
    Some(block(&format!("{REMINDER} {}{more}", msqu_ui::tool::hm(next.at)), "orange", &tip, false))
}

/// the one block, and how long until it changes: icons for what's folded
/// into it (not pinned or hidden), each listed in the tooltip; with nothing
/// folded, just msqu's icon
fn show_all(bar: &model::Bar) -> (Option<Block>, i32) {
    if !bar.one {
        return (None, -1);
    }
    let mut icons: Vec<String> = Vec::new();
    let mut lines = Vec::new();
    let mut wait = -1;
    let mut fold = |b: Block, label: &str, ms: i32| {
        let icon = b.text.split(' ').next().unwrap_or("").to_string();
        if !icons.contains(&icon) {
            icons.push(icon);
        }
        let label = if label.is_empty() || b.text.contains(label) { String::new() } else { format!("  {label}") };
        lines.push(format!("{}{label}", b.text));
        if ms >= 0 && (wait < 0 || ms < wait) {
            wait = ms;
        }
    };
    for r in model::running() {
        let name = model::bar_name(&r);
        if bar.has("hide", r.tool, &name) || bar.has("pin", r.tool, &name) {
            continue;
        }
        if let Some(b) = show(r.tool, Some(&r)) {
            fold(b, &r.label, wait_ms(r.tool, Some(&r)));
        }
    }
    if !bar.has("hide", "notes", "notes") && !bar.has("pin", "notes", "notes") {
        if let Some(b) = show_notes(&std::fs::read_to_string(model::notes_file()).unwrap_or_default()) {
            fold(b, "", wait_ms("notes", None));
        }
    }
    if lines.is_empty() {
        return (Some(block(MSQU, "accent", "msqu\nclick: msqu · right-click: options", false)), wait);
    }
    let tip = format!("{}\nclick: msqu · right-click: options", lines.join("\n"));
    (Some(block(&icons.join(" "), "accent", &tip, false)), wait)
}

/// how long to sleep, in ms, if nothing changes (-1: until something does)
fn wait_ms(tool: &str, r: Option<&Running>) -> i32 {
    let ms_now = || std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0);
    let midnight = || {
        let now = model::now();
        ((model::next_at_from(now, 0, 0) - now) * 1000 + 500).min(i32::MAX as i64) as i32
    };
    // just after the next whole minute
    let minute = || (60_000 - ms_now() % 60_000 + 5) as i32;
    match r {
        Some(_) if tool == "clock" => minute(),
        // hours or minutes, under a day away
        Some(r) if tool == "countdown" && r.timed && r.left > 0 && r.left < 86400 => minute(),
        // midnight, or when it gets under a day away
        Some(r) if tool == "countdown" && r.timed && r.left >= 86400 => midnight().min(((r.left - 86400) * 1000 + 500).min(i32::MAX as i64) as i32),
        Some(_) if tool == "countdown" => midnight(),
        // the next reminder's day ("tomorrow" becomes today)
        _ if tool == "notes" => midnight(),
        // counting: just after the next whole second
        Some(r) if !r.paused && tool != "alarm" => (1000 - ms_now() % 1000 + 5) as i32,
        // the alarm's day: just after midnight
        Some(_) if tool == "alarm" => midnight(),
        _ => -1,
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let tool = args.next().unwrap_or_default();
    if tool == "--set" {
        match args.next().as_deref() {
            Some("one") => model::set_bar_one(true),
            Some("each") => model::set_bar_one(false),
            _ => {
                eprintln!("usage: msqu-bar --set one|each");
                std::process::exit(2);
            }
        }
        return;
    }
    let Some(tool) = model::TOOLS.into_iter().chain(["notes", "all"]).find(|t| *t == tool) else {
        eprintln!("usage: msqu-bar TOOL [SLOT]   tools: {} notes, or all", model::TOOLS.join(" "));
        std::process::exit(2);
    };
    let slot = args.next();
    let name = slot.clone().unwrap_or_else(|| tool.to_string());
    let file = model::tool_dir(tool).join(&name);
    // its state directory (all: every tool's), and the bar settings'
    let mut dirs: Vec<std::path::PathBuf> = if tool == "all" {
        model::TOOLS.into_iter().chain(["notes"]).map(model::tool_dir).collect()
    } else {
        vec![model::tool_dir(tool)]
    };
    dirs.push(model::lasting_dir());

    // watched before the first read, so no change can slip by
    let fd = unsafe { libc::inotify_init1(libc::IN_CLOEXEC) };
    let mask = libc::IN_CLOSE_WRITE | libc::IN_MOVED_TO | libc::IN_DELETE | libc::IN_DELETE_SELF | libc::IN_MOVE_SELF;
    for dir in &dirs {
        let _ = std::fs::create_dir_all(dir);
        let path = CString::new(dir.as_os_str().as_bytes()).unwrap_or_default();
        if fd < 0 || unsafe { libc::inotify_add_watch(fd, path.as_ptr(), mask) } < 0 {
            eprintln!("msqu-bar: can't watch {}: {}", dir.display(), std::io::Error::last_os_error());
            std::process::exit(1);
        }
    }

    let mut out = std::io::stdout().lock();
    let mut last = String::new();
    let mut buf = [0u8; 4096];
    loop {
        let bar = model::load_bar();
        let (line, wait) = if tool == "all" {
            let (b, wait) = show_all(&bar);
            (b.map_or(EMPTY.into(), |b| b.json()), wait)
        } else {
            let text = std::fs::read_to_string(&file).ok();
            let r = text.as_deref().filter(|_| tool != "notes").and_then(|t| model::read_running(tool, t));
            let line = match tool {
                // pinned, or a block each; and not hidden
                _ if !bar.shown(tool, &name) => EMPTY.into(),
                "notes" => render_notes(text.as_deref().unwrap_or("")),
                _ => render(tool, r.as_ref()),
            };
            (line, wait_ms(tool, r.as_ref()))
        };
        if line != last {
            // waybar gone: stop
            if writeln!(out, "{line}").and_then(|_| out.flush()).is_err() {
                std::process::exit(0);
            }
            last = line;
        }
        let mut p = libc::pollfd { fd, events: libc::POLLIN, revents: 0 };
        if unsafe { libc::poll(&mut p, 1, wait) } > 0 {
            let n = unsafe { libc::read(fd, buf.as_mut_ptr().cast(), buf.len()) };
            if n <= 0 {
                std::process::exit(1);
            }
            // the directory itself went (logging out): stop, and let waybar
            // start this again
            let mut i = 0;
            while i + std::mem::size_of::<libc::inotify_event>() <= n as usize {
                let ev = unsafe { std::ptr::read_unaligned(buf.as_ptr().add(i).cast::<libc::inotify_event>()) };
                if ev.mask & (libc::IN_DELETE_SELF | libc::IN_MOVE_SELF | libc::IN_IGNORED) != 0 {
                    std::process::exit(1);
                }
                i += std::mem::size_of::<libc::inotify_event>() + ev.len as usize;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks() {
        let t = model::read_running("timer", "pause 75 300 tea & <b>").unwrap();
        assert_eq!(
            render("timer", Some(&t)),
            r#"{"text":"󰏤 1:15","class":["msqu","yellow","paused"],"tooltip":"tea &amp; &lt;b&gt;: 1:15 of 5:00\nclick: msqu · right-click: options · middle-click: cancel"}"#
        );
        assert_eq!(render("timer", None), r#"{"text":""}"#);
        let d = model::read_running("drink", "pause 61 2700").unwrap();
        assert!(render("drink", Some(&d)).starts_with(r#"{"text":"󰆫 2m","class":["msqu","magenta","paused"]"#));
        let r = model::read_running("repeat", "pause 60 900 app_id=org.vinegarhq.Sober roblox \"afk\"").unwrap();
        assert!(render("repeat", Some(&r)).contains(r#""tooltip":"roblox \"afk\": every 15:00, brings up org.vinegarhq.Sober\nnext in 1:00"#));
        let s = model::read_running("stopwatch", "pause 3725").unwrap();
        assert!(render("stopwatch", Some(&s)).starts_with(r#"{"text":"󰁫 1:02:05","class":["msqu","green","paused"]"#));
        assert_eq!(wait_ms("stopwatch", Some(&s)), -1);
        assert_eq!(wait_ms("timer", None), -1);
        let at = model::now() + 10 * 86400;
        let c = model::read_running("countdown", &format!("at {at} 0 0 xmas & co")).unwrap();
        assert!(render("countdown", Some(&c)).starts_with(r#"{"text":"󰃭 xmas &amp; co 10d","class":["msqu","accent"]"#));
        let gone = model::read_running("countdown", &format!("at {} 0 1 old", model::now() - 3 * 86400)).unwrap();
        assert_eq!(render("countdown", Some(&gone)), r#"{"text":""}"#);
        let k = model::read_running("clock", "zone UTC UTC").unwrap();
        // the build sandbox has no time zone files
        if model::zone_time("UTC", 0).is_some() {
            assert!(render("clock", Some(&k)).starts_with(r#"{"text":"󰥔 UTC "#));
        }
        assert!(wait_ms("clock", Some(&k)) <= 60_005);
        assert_eq!(render_notes("1\t0\tmilk\n"), r#"{"text":""}"#);
        let soon = model::now() + 3600;
        let b = render_notes(&format!("1\t0\tmilk\n2\t{soon}\tcall mum\n3\t{}\tbins\n", soon + 60));
        assert!(b.starts_with(&format!(r#"{{"text":"󰂞 {} +1","class":["msqu","orange"]"#, msqu_ui::tool::hm(soon))), "{b}");
    }
}
