//! msqu drink: a drink reminder every so often, skipped while locked. A new
//! one replaces one still showing (x-canonical-private-synchronous).
//!
//!   msqu drink start SECONDS
//!   msqu drink toggle   pause / resume
//!   msqu drink reset    start the wait over
//!   msqu drink stop     (middle-clicking the block)
//!   msqu drink running  exit 0 if it's on
//!
//! state file "drink": "run END EVERY" or "pause LEFT EVERY"

use super::{notify, num, screen_locked, stale, words, State};
use crate::model::{hms, now};

const FILE: &str = "drink";
/// the old shell version's notification id file, no longer used
const OLD_LAST: &str = ".notify";

struct Drink {
    mode: String,
    /// as written (stale compares it with the alarm's time)
    t: String,
    every: i64,
}

impl Drink {
    /// None if there's none, or no sensible interval (a broken file: it
    /// would go off over and over)
    fn read(st: &State) -> Option<Drink> {
        let (mode, t, rest) = words(&st.read(FILE));
        let every = num(rest.split_whitespace().next().unwrap_or(""));
        (!mode.is_empty() && every > 0).then_some(Drink { mode, t, every })
    }
}

/// arm, or end the command with systemd-run's exit code
macro_rules! arm {
    ($st:expr, $at:expr) => {
        if let Err(code) = $st.arm($at) {
            return code;
        }
    };
}

pub fn main(args: &[String]) -> i32 {
    let mut st = State::open("drink");
    match args.first().map(String::as_str).unwrap_or("") {
        "start" => {
            let secs = args.get(1).map(String::as_str).unwrap_or("");
            let Some(secs) = secs.parse::<i64>().ok().filter(|&s| s > 0 && secs.bytes().all(|b| b.is_ascii_digit())) else {
                eprintln!("usage: msqu drink start SECONDS");
                return 2;
            };
            let end = now().saturating_add(secs);
            arm!(st, end);
            st.put(FILE, &format!("run {end} {secs}"));
            return notify(&["-c", "osd", "-t", "2000", "drink reminder", &format!("every {}", hms(secs))]);
        }
        "toggle" | "click" => {
            let Some(d) = Drink::read(&st) else { return 0 };
            if d.mode == "run" {
                st.disarm();
                let left = num(&d.t).saturating_sub(now()).max(1);
                st.put(FILE, &format!("pause {left} {}", d.every));
            } else {
                let end = now().saturating_add(num(&d.t));
                arm!(st, end);
                st.put(FILE, &format!("run {end} {}", d.every));
            }
        }
        "reset" => {
            // the full wait again: from now if running, or for when it's resumed
            let Some(d) = Drink::read(&st) else { return 0 };
            if d.mode == "run" {
                let end = now().saturating_add(d.every);
                arm!(st, end);
                st.put(FILE, &format!("run {end} {}", d.every));
            } else {
                st.put(FILE, &format!("pause {} {}", d.every, d.every));
            }
        }
        "stop" | "cancel" | "right-click" => {
            st.disarm();
            st.remove(FILE);
        }
        "running" => return if Drink::read(&st).is_some() { 0 } else { 1 },
        "ring" => {
            let Some(d) = Drink::read(&st) else { return 0 };
            if d.mode != "run" || stale(&d.mode, &d.t) {
                return 0;
            }
            let end = now().saturating_add(d.every);
            arm!(st, end);
            st.put(FILE, &format!("run {end} {}", d.every));
            let _ = std::fs::remove_file(st.dir.join(OLD_LAST));
            st.unlock();
            if screen_locked() {
                return 0;
            }
            // stays until clicked away (-t 0), in place of one still showing
            notify(&[
                "-t",
                "0",
                "-c",
                "msqu-drink",
                "-h",
                "string:x-canonical-private-synchronous:msqu-drink",
                "drink some water",
                &format!("it's been {}", hms(d.every)),
            ]);
            st.bell();
        }
        "middle-click" => {}
        _ => {
            eprintln!("usage: msqu drink [start SECONDS|toggle|reset|stop|running]");
            return 2;
        }
    }
    0
}
