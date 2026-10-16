//! msqu timer: countdowns on the bar (yellow), up to MSQU_SLOTS at once,
//! each its own block. When one ends, a notification shows and the alarm
//! sound plays until it's closed.
//!
//!   msqu timer start SECONDS [LABEL]   in the first free slot
//!   msqu timer toggle SLOT   pause / resume (clicking its block)
//!   msqu timer cancel SLOT   (right-clicking its block)
//!   msqu timer add SLOT AMOUNT   more time, or less: +5m, -1m, 90s, 1:30
//!                            (a plain number is minutes)
//!   msqu timer more SLOT     pick how much in fuzzel (middle-clicking its
//!                            block)
//!   msqu timer running       exit 0 if any timer is set
//!
//! state file per slot ("1", "2", ...): "run END TOTAL LABEL" or
//! "pause LEFT TOTAL LABEL" (seconds; END is a unix time)

use super::{notify, num, stale, words, State};
use crate::model::{hms, now};
use std::io::Write;
use std::process::{Command, Stdio};

struct Timer {
    mode: String,
    /// as written (stale compares it with the alarm's time)
    t: String,
    total: i64,
    label: String,
}

impl Timer {
    /// the timer in the slot being worked on (None without a slot)
    fn read(st: &State) -> Option<Timer> {
        if st.slot.is_empty() {
            return None;
        }
        let (mode, t, rest) = words(&st.read(&st.slot));
        let (total, label) = match rest.split_once([' ', '\t']) {
            Some((total, label)) => (total, label.trim_start_matches([' ', '\t'])),
            None => (rest.as_str(), ""),
        };
        (!mode.is_empty()).then(|| Timer { mode, t, total: num(total), label: label.to_string() })
    }
}

/// "+5m" "-1m" "3" (minutes) "90s" "1h" "1:30" -> seconds, signed (spaces
/// ignored); None for anything else
pub fn signed_secs(s: &str) -> Option<i64> {
    let s: String = s.chars().filter(|&c| c != ' ').collect();
    let (sign, s) = match s.strip_prefix('+') {
        Some(rest) => (1, rest),
        None => match s.strip_prefix('-') {
            Some(rest) => (-1, rest),
            None => (1, s.as_str()),
        },
    };
    let digits = |d: &str| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit());
    let n = |d: &str| d.parse::<i64>().ok();
    let secs = if let Some((m, sec)) = s.split_once(':') {
        let ok = digits(m) && sec.len() == 2 && digits(sec) && sec.as_bytes()[0] <= b'5';
        if !ok {
            return None;
        }
        n(m)?.checked_mul(60)?.checked_add(n(sec)?)?
    } else if let Some(m) = s.strip_suffix("min").or_else(|| s.strip_suffix('m')).filter(|m| digits(m)) {
        n(m)?.checked_mul(60)?
    } else if digits(s) {
        n(s)?.checked_mul(60)?
    } else if let Some(sec) = s.strip_suffix('s').filter(|d| digits(d)) {
        n(sec)?
    } else {
        let h = s.strip_suffix('h').filter(|d| digits(d))?;
        n(h)?.checked_mul(3600)?
    };
    Some(sign * secs)
}

/// Adds `d` seconds to the timer in the slot (negative takes time off,
/// down to a second left); its total changes the same way. Ok(()) also
/// when there's no timer there; Err(exit code) if systemd-run failed.
fn add(st: &State, d: i64) -> Result<(), i32> {
    let Some(tm) = Timer::read(st) else { return Ok(()) };
    // saturating: an absurd amount can't wrap round
    let base = if tm.mode == "run" { num(&tm.t).saturating_sub(now()) } else { num(&tm.t) };
    let left = base.saturating_add(d).max(1);
    let total = tm.total.saturating_add(d).max(left);
    if tm.mode == "run" {
        let end = now().saturating_add(left);
        st.arm(end)?;
        st.put(&st.slot, &format!("run {end} {total} {}", tm.label));
    } else {
        st.put(&st.slot, &format!("pause {left} {total} {}", tm.label));
    }
    Ok(())
}

pub fn main(args: &[String]) -> i32 {
    let mut st = State::open("timer");
    let arg = |i: usize| args.get(i).map(String::as_str).unwrap_or("");
    macro_rules! slot {
        ($n:expr) => {
            if let Err(code) = st.use_slot($n) {
                return code;
            }
        };
    }
    macro_rules! ok {
        ($r:expr) => {
            if let Err(code) = $r {
                return code;
            }
        };
    }
    match arg(0) {
        "start" => {
            let (secs, label) = (arg(1), arg(2));
            let secs = match secs.parse::<i64>() {
                Ok(s) if secs.bytes().all(|b| b.is_ascii_digit()) && s > 0 => s,
                _ => {
                    eprintln!("usage: msqu timer start SECONDS [LABEL]");
                    return 2;
                }
            };
            let Some(free) = st.free_slot() else {
                let n = std::env::var("MSQU_SLOTS").unwrap_or_default();
                eprintln!("{n} timers are running already: cancel one first");
                return 1;
            };
            st.slot = free;
            let end = now().saturating_add(secs);
            ok!(st.arm(end));
            st.put(&st.slot, &format!("run {end} {secs} {label}"));
            let label = if label.is_empty() { String::new() } else { format!(": {label}") };
            return notify(&["-c", "osd", "-t", "1500", "timer", &format!("{}{label}", hms(secs))]);
        }
        "toggle" | "click" => {
            slot!(arg(1));
            let Some(tm) = Timer::read(&st) else { return 0 };
            if tm.mode == "run" {
                st.disarm();
                let left = num(&tm.t).saturating_sub(now()).max(1);
                st.put(&st.slot, &format!("pause {left} {} {}", tm.total, tm.label));
            } else {
                let end = now().saturating_add(num(&tm.t));
                ok!(st.arm(end));
                st.put(&st.slot, &format!("run {end} {} {}", tm.total, tm.label));
            }
        }
        "cancel" | "right-click" => {
            slot!(arg(1));
            st.disarm();
            st.remove(&st.slot);
        }
        "add" => {
            slot!(arg(1));
            let Some(d) = signed_secs(arg(2)) else {
                eprintln!("usage: msqu timer add SLOT AMOUNT   (+5m, -1m, 90s, 1:30)");
                return 2;
            };
            ok!(add(&st, d));
        }
        "more" | "middle-click" => {
            slot!(arg(1));
            let Some(tm) = Timer::read(&st) else { return 0 };
            let left = if tm.mode == "run" { num(&tm.t).saturating_sub(now()) } else { num(&tm.t) };
            // not locked while choosing; add reads the timer again after
            st.unlock();
            let name = if tm.label.is_empty() { "timer" } else { &tm.label };
            let prompt = format!("{name} {}  ", hms(left));
            let Some(pick) = fuzzel(&prompt) else { return 0 };
            if pick.is_empty() {
                return 0;
            }
            st.relock();
            match signed_secs(&pick) {
                Some(d) => ok!(add(&st, d)),
                None => {
                    let msg = format!("can't read \"{pick}\": try +3, -2, 90s or 1:30");
                    return notify(&["-c", "osd", "-t", "2000", "timer", &msg]);
                }
            }
        }
        "running" => return if st.any_slot() { 0 } else { 1 },
        "ring" => {
            // no slot: one set before timers had slots (its state file is
            // "timer"; nothing reads it now)
            if !arg(1).is_empty() {
                slot!(arg(1));
            }
            let mut msg = "done".to_string();
            if let Some(tm) = Timer::read(&st) {
                if stale(&tm.mode, &tm.t) {
                    return 0;
                }
                let label = if tm.label.is_empty() { "done" } else { &tm.label };
                msg = format!("{label} ({})", hms(tm.total));
            } else if std::env::var("MSQU_AT").is_ok_and(|v| !v.is_empty()) {
                return 0; // cancelled as it went off
            }
            let file = if st.slot.is_empty() { "timer".to_string() } else { st.slot.clone() };
            st.remove(&file);
            st.ring("timer", &msg);
        }
        _ => {
            eprintln!("usage: msqu timer [start SECONDS [LABEL]|toggle SLOT|cancel SLOT|add SLOT AMOUNT|more SLOT|running]");
            return 2;
        }
    }
    0
}

/// The "more time" picker: fuzzel offering a few amounts, or typed. What
/// was picked (its last newline dropped), or None if it was closed.
fn fuzzel(prompt: &str) -> Option<String> {
    let child = Command::new("fuzzel")
        .args(["--dmenu", "--lines", "6", "--width", "34", "--prompt", prompt, "--placeholder", "or type: +3, -2, 90s, 1:30"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            eprintln!("fuzzel: {e}");
            return None;
        }
    };
    if let Some(mut input) = child.stdin.take() {
        let _ = input.write_all(b"+1m\n+5m\n+10m\n+15m\n-1m\n-5m\n");
    }
    let out = child.wait_with_output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim_end_matches('\n').to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amounts() {
        for (s, v) in [
            ("+5m", Some(300)),
            ("-1m", Some(-60)),
            ("3", Some(180)),
            ("90s", Some(90)),
            ("1h", Some(3600)),
            ("1:30", Some(90)),
            ("+1:05", Some(65)),
            ("-0:59", Some(-59)),
            ("10min", Some(600)),
            ("+ 5 m", Some(300)),
            ("007", Some(420)),
            ("+0", Some(0)),
            ("1:60", None),
            ("1:5", None),
            (":30", None),
            ("1:", None),
            ("", None),
            ("+", None),
            ("-", None),
            ("+-5", None),
            ("5x", None),
            ("1.5", None),
            ("5mins", None),
            ("5 h", Some(18000)),
            ("1h30m", None),
            ("\t5", None),
        ] {
            assert_eq!(signed_secs(s), v, "{s:?}");
        }
    }
}
