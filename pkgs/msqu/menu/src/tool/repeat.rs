//! msqu repeat: a reminder every so often, optionally bringing a window to
//! the front.
//!
//!   msqu repeat start SECONDS WINDOW [LABEL]
//!                         WINDOW: app_id=ID, class=CLASS or - for none
//!   msqu repeat windows   the open windows, one per app: "WINDOW<tab>title"
//!   msqu repeat toggle    pause / resume (clicking the block)
//!   msqu repeat reset     start the wait over, from now (middle-clicking)
//!   msqu repeat cancel    stop (right-clicking the block)
//!
//! state file "repeat": "run END EVERY WINDOW LABEL" or
//! "pause LEFT EVERY WINDOW LABEL" (seconds; END is a unix time; WINDOW is
//! app_id=ID, class=CLASS or - for none)

use super::sway::{self, Json};
use super::{fields, notify, num, stale, State};
use crate::model::{hms, now};

const FILE: &str = "repeat";

struct Repeat {
    mode: String,
    /// as written (stale compares it with the alarm's time)
    t: String,
    every: i64,
    window: String,
    label: String,
}

impl Repeat {
    /// None if there's none, or no sensible interval (a broken file: it
    /// would go off over and over)
    fn read(st: &State) -> Option<Repeat> {
        let mut f = fields(&st.read(FILE), 5).into_iter();
        let mut next = || f.next().unwrap_or_default();
        let (mode, t, every, window, label) = (next(), next(), num(&next()), next(), next());
        (!mode.is_empty() && every > 0).then_some(Repeat { mode, t, every, window, label })
    }

    fn line(&self, mode: &str, t: i64) -> String {
        format!("{mode} {t} {} {} {}", self.every, self.window, self.label)
    }
}

/// "app_id=org.vinegarhq.Sober" -> "org.vinegarhq.Sober"; "-" -> "nothing"
fn window_name(w: &str) -> &str {
    if w == "-" { "nothing" } else { w.split_once('=').map_or(w, |(_, name)| name) }
}

/// The open windows, one per app, sorted: "app_id=ID<tab>title" (or
/// class=CLASS for XWayland).
fn windows() -> Option<Vec<(String, String)>> {
    let tree = sway::tree()?;
    let mut found: Vec<(String, String)> = Vec::new();
    tree.each(&mut |n| {
        if !matches!(n, Json::Obj(_)) || !Json::truthy(n.get("pid")) {
            return;
        }
        let app = n.get("app_id").filter(|v| Json::truthy(Some(v)));
        let class = n.get("window_properties").and_then(|p| p.get("class")).filter(|v| Json::truthy(Some(v)));
        let w = match (app, class) {
            (Some(a), _) => format!("app_id={}", a.as_str().unwrap_or_default()),
            (None, Some(c)) => format!("class={}", c.as_str().unwrap_or_default()),
            _ => return,
        };
        let title = n.get("name").and_then(Json::as_str).unwrap_or("").replace('\t', " ");
        found.push((w, title));
    });
    // sorted by WINDOW, the first of each kept (jq's unique_by)
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found.dedup_by(|b, a| a.0 == b.0);
    Some(found)
}

pub fn main(args: &[String]) -> i32 {
    let mut st = State::open("repeat");
    let arg = |i: usize| args.get(i).map(String::as_str).unwrap_or("");
    macro_rules! ok {
        ($r:expr) => {
            if let Err(code) = $r {
                return code;
            }
        };
    }
    match arg(0) {
        "start" => {
            let (secs, win, name) = (arg(1), if arg(2).is_empty() { "-" } else { arg(2) }, arg(3));
            let secs = match secs.parse::<i64>() {
                Ok(s) if secs.bytes().all(|b| b.is_ascii_digit()) && s > 0 && (win == "-" || win.starts_with("app_id=") || win.starts_with("class=")) => s,
                _ => {
                    eprintln!("usage: msqu repeat start SECONDS WINDOW [LABEL]");
                    return 2;
                }
            };
            let mut name = if name.is_empty() { window_name(win) } else { name };
            if name == "nothing" {
                name = "reminder";
            }
            let end = now().saturating_add(secs);
            ok!(st.arm(end));
            st.put(FILE, &format!("run {end} {secs} {win} {name}"));
            let brings = if win == "-" { String::new() } else { format!(", brings up {}", window_name(win)) };
            return notify(&["-c", "osd", "-t", "2000", name, &format!("every {}{brings}", hms(secs))]);
        }
        "windows" => {
            let Some(list) = windows() else {
                eprintln!("msqu repeat: can't reach sway");
                return 1;
            };
            let mut out = String::new();
            for (w, title) in list {
                out += &format!("{w}\t{title}\n");
            }
            print!("{out}");
        }
        "toggle" | "click" => {
            let Some(r) = Repeat::read(&st) else { return 0 };
            if r.mode == "run" {
                st.disarm();
                let left = num(&r.t).saturating_sub(now()).max(1);
                st.put(FILE, &r.line("pause", left));
            } else {
                let end = now().saturating_add(num(&r.t));
                ok!(st.arm(end));
                st.put(FILE, &r.line("run", end));
            }
        }
        "reset" | "middle-click" => {
            // the full wait again: from now if running, or for when it's resumed
            let Some(r) = Repeat::read(&st) else { return 0 };
            if r.mode == "run" {
                let end = now().saturating_add(r.every);
                ok!(st.arm(end));
                st.put(FILE, &r.line("run", end));
            } else {
                st.put(FILE, &r.line("pause", r.every));
            }
            return notify(&["-c", "osd", "-t", "1500", &r.label, &format!("back to {}", hms(r.every))]);
        }
        "cancel" | "right-click" => {
            st.disarm();
            st.remove(FILE);
        }
        "running" => return if Repeat::read(&st).is_some() { 0 } else { 1 },
        "ring" => {
            let Some(r) = Repeat::read(&st) else { return 0 };
            if r.mode != "run" || stale(&r.mode, &r.t) {
                return 0;
            }
            // the next one first, so it keeps going whatever happens below
            let end = now().saturating_add(r.every);
            ok!(st.arm(end));
            st.put(FILE, &r.line("run", end));
            st.unlock();
            let code = if r.window == "-" {
                notify(&["-c", "msqu-repeat", &r.label, &format!("every {}", hms(r.every))])
            } else {
                // as the shell's ${window%%=*} and ${window#*=}: without an
                // "=", both are the whole thing
                let (kind, id) = r.window.split_once('=').unwrap_or((&r.window, &r.window));
                if sway::run(&format!("[{kind}=\"{id}\"] focus")) {
                    notify(&["-c", "msqu-repeat", &r.label, &format!("brought up {}", window_name(&r.window))])
                } else {
                    notify(&["-c", "msqu-repeat", "-u", "critical", &r.label, &format!("{} isn't open", window_name(&r.window))])
                }
            };
            // the notification failed: stop there, as the shell version did
            if code != 0 {
                return code;
            }
            st.bell();
        }
        _ => {
            eprintln!("usage: msqu repeat [start SECONDS WINDOW [LABEL]|windows|toggle|reset|cancel|running]");
            return 2;
        }
    }
    0
}
