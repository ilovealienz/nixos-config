//! msqu alarm: goes off at a time of day, up to MSQU_SLOTS at once.
//!
//!   msqu alarm start UNIXTIME [LABEL [SLOT]]
//!                            in the first free slot, or in place of SLOT's
//!   msqu alarm cancel SLOT   (middle-clicking its block)
//!   msqu alarm running       exit 0 if any alarm is set
//!
//! state file per slot ("1", "2", ...): "at UNIXTIME LABEL"

use super::{hm, notify, stale, words, State};
use crate::model::now;

struct Alarm {
    mode: String,
    /// as written (stale compares it with the alarm's time)
    t: String,
    label: String,
}

impl Alarm {
    /// the alarm in the slot being worked on (None without a slot)
    fn read(st: &State) -> Option<Alarm> {
        if st.slot.is_empty() {
            return None;
        }
        let (mode, t, label) = words(&st.read(&st.slot));
        (!mode.is_empty()).then_some(Alarm { mode, t, label })
    }
}

pub fn main(args: &[String]) -> i32 {
    let mut st = State::open("alarm");
    let arg = |i: usize| args.get(i).map(String::as_str).unwrap_or("");
    match arg(0) {
        "start" => {
            let (at, label) = (arg(1), arg(2));
            let at = match at.parse::<i64>() {
                Ok(t) if at.bytes().all(|b| b.is_ascii_digit()) && t > now() => t,
                _ => {
                    eprintln!("usage: msqu alarm start UNIXTIME [LABEL] (a time still to come)");
                    return 2;
                }
            };
            if !arg(3).is_empty() {
                if let Err(code) = st.use_slot(arg(3)) {
                    return code;
                }
            } else {
                let Some(free) = st.free_slot() else {
                    let n = std::env::var("MSQU_SLOTS").unwrap_or_default();
                    eprintln!("{n} alarms are set already: cancel one first");
                    return 1;
                };
                st.slot = free;
            }
            if let Err(code) = st.arm(at) {
                return code;
            }
            st.put(&st.slot, &format!("at {at} {label}"));
            let label = if label.is_empty() { String::new() } else { format!(": {label}") };
            return notify(&["-c", "osd", "-t", "2000", "alarm", &format!("{}{label}", hm(at))]);
        }
        "cancel" | "right-click" => {
            if let Err(code) = st.use_slot(arg(1)) {
                return code;
            }
            st.disarm();
            st.remove(&st.slot);
        }
        "running" => return if st.any_slot() { 0 } else { 1 },
        "ring" => {
            // no slot: one set before alarms had slots (its state file is
            // "alarm")
            if !arg(1).is_empty() {
                if let Err(code) = st.use_slot(arg(1)) {
                    return code;
                }
            }
            let mut msg = "alarm".to_string();
            if let Some(a) = Alarm::read(&st) {
                if stale(&a.mode, &a.t) {
                    return 0;
                }
                let label = if a.label.is_empty() { "alarm" } else { &a.label };
                msg = format!("{label} ({})", hm(super::num(&a.t)));
            } else if std::env::var("MSQU_AT").is_ok_and(|v| !v.is_empty()) {
                return 0; // cancelled as it went off
            }
            let file = if st.slot.is_empty() { "alarm".to_string() } else { st.slot.clone() };
            st.remove(&file);
            st.ring("alarm", &msg);
        }
        "click" | "middle-click" => {}
        _ => {
            eprintln!("usage: msqu alarm [start UNIXTIME [LABEL [SLOT]]|cancel SLOT|running]");
            return 2;
        }
    }
    0
}
