//! msqu countdown: days to a date, up to MSQU_SLOTS at once, kept across
//! reboots. A notification on the day: at 9:00, or at the time given.
//!
//!   msqu countdown start UNIXTIME TIMED [LABEL [SLOT]]
//!                            TIMED: 1 if a time of day was given, else 0
//!   msqu countdown cancel SLOT   (middle-clicking its block)
//!   msqu countdown middle-click SLOT   hides its block
//!   msqu countdown rearm     after logging in: arms the notifications again,
//!                            sends any missed, removes ones that are over
//!   msqu countdown running   exit 0 if any countdown is set
//!
//! state file per slot ("1", "2", ...): "at UNIXTIME TIMED NOTIFIED LABEL"

use super::{fields, hm, notify, num, State};
use crate::model::{date_text, day_after, days_between, local_time, now, ymd};

struct Countdown {
    at: i64,
    timed: bool,
    notified: bool,
    label: String,
}

impl Countdown {
    fn read(st: &State) -> Option<Countdown> {
        let f = fields(&st.read(&st.slot), 5);
        (f[0] == "at" && !f[1].is_empty()).then(|| Countdown {
            at: num(&f[1]),
            timed: f[2] == "1",
            notified: f[3] == "1",
            label: f[4].clone(),
        })
    }

    fn save(&self, st: &State) {
        st.put(&st.slot, &format!("at {} {} {} {}", self.at, self.timed as u8, self.notified as u8, self.label));
    }

    /// when to notify: the time given, or 9:00 on the day
    fn notify_at(&self) -> i64 {
        if self.timed {
            return self.at;
        }
        let (y, m, d) = ymd(self.at);
        local_time(y, m, d, 9, 0).unwrap_or(self.at)
    }

    /// its day is over
    fn over(&self) -> bool {
        days_between(now(), self.at) < 0
    }

    fn name(&self) -> &str {
        if self.label.is_empty() { "countdown" } else { &self.label }
    }

    /// arms whatever comes next: the notification, or the clean-up after
    /// its day
    fn arm(&self, st: &State) -> Result<(), i32> {
        if self.notified { st.arm_as(day_after(self.at), "expire") } else { st.arm(self.notify_at()) }
    }
}

/// the notification and the bell, then arms the clean-up
fn ring(st: &mut State, c: &mut Countdown) {
    c.notified = true;
    c.save(st);
    let _ = c.arm(st);
    let when = if c.timed { format!("today, {}", hm(c.at)) } else { "today".into() };
    let name = c.name().to_string();
    notify(&["-c", "msqu-countdown", &name, &when]);
    st.bell();
}

pub fn main(args: &[String]) -> i32 {
    let mut st = State::open("countdown");
    let arg = |i: usize| args.get(i).map(String::as_str).unwrap_or("");
    match arg(0) {
        "start" => {
            let at = match arg(1).parse::<i64>() {
                Ok(t) if arg(1).bytes().all(|b| b.is_ascii_digit()) && days_between(now(), t) >= 0 => t,
                _ => {
                    eprintln!("usage: msqu countdown start UNIXTIME TIMED [LABEL] (a day still to come)");
                    return 2;
                }
            };
            if !arg(4).is_empty() {
                if let Err(code) = st.use_slot(arg(4)) {
                    return code;
                }
            } else {
                let Some(free) = st.free_slot() else {
                    let n = std::env::var("MSQU_SLOTS").unwrap_or_default();
                    eprintln!("{n} countdowns are set already: remove one first");
                    return 1;
                };
                st.slot = free;
            }
            let mut c = Countdown { at, timed: arg(2) == "1", notified: false, label: arg(3).to_string() };
            let msg = match days_between(now(), at) {
                0 => "today".to_string(),
                1 => "tomorrow".to_string(),
                n => format!("{n} days, {}", date_text(at)),
            };
            notify(&["-c", "osd", "-t", "2000", c.name(), &msg]);
            // today, after its time: nothing to send, just the clean-up
            c.notified = c.notify_at() <= now();
            if let Err(code) = c.arm(&st) {
                return code;
            }
            c.save(&st);
        }
        "cancel" | "right-click" => {
            if let Err(code) = st.use_slot(arg(1)) {
                return code;
            }
            st.disarm();
            st.remove(&st.slot);
        }
        "ring" => {
            if let Err(code) = st.use_slot(arg(1)) {
                return code;
            }
            let Some(mut c) = Countdown::read(&st) else { return 0 };
            // replaced, or already done
            let at = std::env::var("MSQU_AT").unwrap_or_default();
            if c.notified || (!at.is_empty() && at != c.notify_at().to_string()) {
                return 0;
            }
            ring(&mut st, &mut c);
        }
        "expire" => {
            if let Err(code) = st.use_slot(arg(1)) {
                return code;
            }
            if Countdown::read(&st).is_some_and(|c| c.over()) {
                st.remove(&st.slot);
            }
        }
        "rearm" => {
            let slots: Vec<String> = (1..=9).map(|i| i.to_string()).filter(|s| st.dir.join(s).exists()).collect();
            for slot in slots {
                st.slot = slot;
                let Some(mut c) = Countdown::read(&st) else { continue };
                if c.over() {
                    st.disarm();
                    st.remove(&st.slot);
                } else if c.notified || c.notify_at() > now() {
                    let _ = c.arm(&st);
                } else {
                    // missed while this was off
                    ring(&mut st, &mut c);
                    st.relock();
                }
            }
        }
        "running" => return if st.any_slot() { 0 } else { 1 },
        "middle-click" => {
            if let Err(code) = st.use_slot(arg(1)) {
                return code;
            }
            crate::model::set_mark("hide", st.tool, &st.slot, true);
        }
        "click" => {}
        _ => {
            eprintln!("usage: msqu countdown [start UNIXTIME TIMED [LABEL [SLOT]]|cancel SLOT|rearm|running]");
            return 2;
        }
    }
    0
}
