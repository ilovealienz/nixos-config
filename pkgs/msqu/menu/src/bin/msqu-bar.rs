//! msqu-bar: one msqu block on waybar. Prints the block (waybar's JSON)
//! whenever it changes, and is otherwise asleep: it wakes when the tool's
//! state files change (inotify), on each whole second while counting, and
//! at midnight for an alarm (whose block names the day if it isn't today).
//! It has no child processes, so there's nothing to leave behind when
//! waybar stops it.
//!
//! Usage: msqu-bar TOOL [SLOT]
//!   SLOT: which one, for a tool that runs several at once (timer 1, 2, 3)
//!
//! The look is waybar's CSS (sway/msqu.nix): each block has the classes
//! "msqu", its colour, and "paused" while paused.

use msqu_ui::model::{self, Running, clock_text, hms};
use std::ffi::CString;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;

const TIMER: &str = "󰔛";
const REPEAT: &str = "󰑖";
const STOPWATCH: &str = "󰁫";
const ALARM: &str = "󰀠";
const DRINK: &str = "󰆫";
const PAUSED: &str = "󰏤";

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

fn block(text: &str, colour: &str, tooltip: &str, paused: bool) -> String {
    format!(
        "{{\"text\":{},\"class\":[\"msqu\",\"{colour}\"{}],\"tooltip\":{}}}",
        json(&pango(text)),
        if paused { ",\"paused\"" } else { "" },
        json(&pango(tooltip))
    )
}

/// the block for what's in the state file (None: nothing, so hidden)
pub fn render(tool: &str, r: Option<&Running>) -> String {
    let Some(r) = r else { return "{\"text\":\"\"}".into() };
    let hint = if r.paused { "click to resume" } else { "click to pause" };
    let label = |sep: &str| if r.label.is_empty() { String::new() } else { format!("{}{sep}", r.label) };
    match tool {
        "timer" => block(
            &format!("{} {}", if r.paused { PAUSED } else { TIMER }, hms(r.left)),
            "yellow",
            &format!("{}{} of {}\n{hint}, right-click to cancel\nmiddle-click for more time", label(": "), hms(r.left), hms(r.total)),
            r.paused,
        ),
        "stopwatch" => block(&format!("{STOPWATCH} {}", hms(r.left)), "green", &format!("stopwatch\n{hint}, right-click to stop"), r.paused),
        "alarm" => {
            let when = clock_text(r.at);
            block(&format!("{ALARM} {when}"), "orange", &format!("{}{when}\nright-click to cancel", label(": ")), false)
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
                &format!("{}every {}{win}\nnext in {}\n{hint}, right-click to stop\nmiddle-click to start the wait over", label(": "), hms(r.total), hms(r.left)),
                r.paused,
            )
        }
        // drink: whole minutes, rounded up ("1m" until it goes off)
        _ => block(
            &format!("{DRINK} {}m", (r.left + 59) / 60),
            "magenta",
            &format!("drink water every {}\nnext in {}\n{hint}, right-click to stop", hms(r.total), hms(r.left)),
            r.paused,
        ),
    }
}

/// how long to sleep, in ms, if nothing changes (-1: until something does)
fn wait_ms(tool: &str, r: Option<&Running>) -> i32 {
    let ms_now = || std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0);
    match r {
        // counting: just after the next whole second
        Some(r) if !r.paused && tool != "alarm" => (1000 - ms_now() % 1000 + 5) as i32,
        // the alarm's day: just after midnight
        Some(_) if tool == "alarm" => {
            let now = model::now();
            ((model::next_at_from(now, 0, 0) - now) * 1000 + 500).min(i32::MAX as i64) as i32
        }
        _ => -1,
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let tool = args.next().unwrap_or_default();
    let Some(tool) = model::TOOLS.into_iter().find(|t| *t == tool) else {
        eprintln!("usage: msqu-bar TOOL [SLOT]   tools: {}", model::TOOLS.join(" "));
        std::process::exit(2);
    };
    let slot = args.next();
    let dir = model::state_dir().join(tool);
    let _ = std::fs::create_dir_all(&dir);
    let file = dir.join(slot.as_deref().unwrap_or(tool));

    // watched before the first read, so no change can slip by
    let fd = unsafe { libc::inotify_init1(libc::IN_CLOEXEC) };
    let path = CString::new(dir.as_os_str().as_bytes()).unwrap_or_default();
    let mask = libc::IN_CLOSE_WRITE | libc::IN_MOVED_TO | libc::IN_DELETE | libc::IN_DELETE_SELF | libc::IN_MOVE_SELF;
    if fd < 0 || unsafe { libc::inotify_add_watch(fd, path.as_ptr(), mask) } < 0 {
        eprintln!("msqu-bar: can't watch {}: {}", dir.display(), std::io::Error::last_os_error());
        std::process::exit(1);
    }

    let mut out = std::io::stdout().lock();
    let mut last = String::new();
    let mut buf = [0u8; 4096];
    loop {
        let r = std::fs::read_to_string(&file).ok().and_then(|t| model::read_running(tool, &t));
        let line = render(tool, r.as_ref());
        if line != last {
            // waybar gone: stop
            if writeln!(out, "{line}").and_then(|_| out.flush()).is_err() {
                std::process::exit(0);
            }
            last = line;
        }
        let mut p = libc::pollfd { fd, events: libc::POLLIN, revents: 0 };
        if unsafe { libc::poll(&mut p, 1, wait_ms(tool, r.as_ref())) } > 0 {
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
            r#"{"text":"󰏤 1:15","class":["msqu","yellow","paused"],"tooltip":"tea &amp; &lt;b&gt;: 1:15 of 5:00\nclick to resume, right-click to cancel\nmiddle-click for more time"}"#
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
    }
}
