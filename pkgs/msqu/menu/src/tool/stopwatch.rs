//! msqu stopwatch: counts up.
//!
//!   msqu stopwatch start    from 0 (a running one starts again)
//!   msqu stopwatch toggle   pause / resume (clicking the block)
//!   msqu stopwatch stop     stop, and show the time (right-clicking)
//!   msqu stopwatch running  exit 0 if one is going
//!
//! state file "stopwatch": "run START SO_FAR" or "pause SO_FAR" (seconds;
//! START is a unix time; SO_FAR is what was counted before the last pause)

use super::{notify, num, words, State};
use crate::model::{hms, now};

const FILE: &str = "stopwatch";

struct Stopwatch {
    mode: String,
    t: i64,
    sofar: i64,
}

impl Stopwatch {
    /// None when there's no stopwatch
    fn read(st: &State) -> Option<Stopwatch> {
        let (mode, t, sofar) = words(&st.read(FILE));
        (!mode.is_empty()).then(|| Stopwatch { mode, t: num(&t), sofar: num(&sofar) })
    }

    /// seconds counted so far
    fn elapsed(&self) -> i64 {
        if self.mode == "run" { self.sofar.saturating_add(now().saturating_sub(self.t)) } else { self.t }
    }
}

pub fn main(args: &[String]) -> i32 {
    let st = State::open("stopwatch");
    match args.first().map(String::as_str).unwrap_or("") {
        "start" => st.put(FILE, &format!("run {} 0", now())),
        "toggle" | "click" => {
            let Some(s) = Stopwatch::read(&st) else { return 0 };
            if s.mode == "run" {
                st.put(FILE, &format!("pause {}", s.elapsed()));
            } else {
                st.put(FILE, &format!("run {} {}", now(), s.t));
            }
        }
        "stop" | "cancel" | "right-click" => {
            let Some(s) = Stopwatch::read(&st) else { return 0 };
            let total = s.elapsed();
            st.remove(FILE);
            return notify(&["stopwatch", &hms(total)]);
        }
        "running" => return if Stopwatch::read(&st).is_some() { 0 } else { 1 },
        "middle-click" => {}
        _ => {
            eprintln!("usage: msqu stopwatch [start|toggle|stop|running]");
            return 2;
        }
    }
    0
}
