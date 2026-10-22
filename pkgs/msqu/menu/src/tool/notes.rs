//! msqu notes: short notes, each with an optional reminder, kept across
//! reboots.
//!
//!   msqu notes add UNIXTIME|0 TEXT   0: no reminder
//!   msqu notes done ID       removes it
//!   msqu notes right-click   cancels the next reminder, the note stays
//!                            (middle-clicking the block)
//!   msqu notes middle-click  hides the block until the next reminder
//!   msqu notes rearm         after logging in: arms the reminders again,
//!                            sends any missed
//!   msqu notes running       exit 0 if a reminder is set
//!
//! the notes file: "ID\tUNIXTIME\tTEXT" lines (UNIXTIME 0: no reminder)

use super::{notify, State};
use crate::model::{now, parse_notes, Note};

const FILE: &str = "notes";

fn load(st: &State) -> Vec<Note> {
    parse_notes(&std::fs::read_to_string(st.dir.join(FILE)).unwrap_or_default())
}

fn save(st: &State, notes: &[Note]) {
    if notes.is_empty() {
        st.remove(FILE);
        return;
    }
    let text: Vec<String> = notes.iter().map(|n| format!("{}\t{}\t{}", n.id, n.at, n.text)).collect();
    st.put(FILE, &text.join("\n"));
}

/// each reminder has its own timer, named by the note's number
fn slot(st: &mut State, id: u64) {
    st.slot = id.to_string();
}

/// the reminder, then it's cleared (the note stays)
fn remind(st: &mut State, id: u64) {
    let mut notes = load(st);
    let Some(n) = notes.iter_mut().find(|n| n.id == id) else { return };
    n.at = 0;
    let text = n.text.clone();
    save(st, &notes);
    notify(&["-c", "msqu-notes", "note", &text]);
    st.bell();
}

pub fn main(args: &[String]) -> i32 {
    let mut st = State::open("notes");
    let arg = |i: usize| args.get(i).map(String::as_str).unwrap_or("");
    let id = || arg(1).parse::<u64>().ok();
    match arg(0) {
        "add" => {
            let text = args.get(2..).map(|a| a.join(" ")).unwrap_or_default();
            let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
            let at = match arg(1).parse::<i64>() {
                Ok(t) if t == 0 || t > now() => t,
                _ => -1,
            };
            if at < 0 || text.is_empty() {
                eprintln!("usage: msqu notes add UNIXTIME|0 TEXT (a time still to come)");
                return 2;
            }
            let mut notes = load(&st);
            let id = notes.iter().map(|n| n.id).max().unwrap_or(0) + 1;
            notes.push(Note { id, at, text });
            if at > 0 {
                crate::model::set_mark("hide", "notes", FILE, false);
                slot(&mut st, id);
                if let Err(code) = st.arm(at) {
                    return code;
                }
            }
            save(&st, &notes);
        }
        "done" | "remove" | "cancel" => {
            let Some(id) = id() else {
                eprintln!("usage: msqu notes done ID");
                return 2;
            };
            let mut notes = load(&st);
            notes.retain(|n| n.id != id);
            slot(&mut st, id);
            st.disarm();
            save(&st, &notes);
        }
        "ring" => {
            let Some(id) = id() else { return 2 };
            // replaced, removed or already done
            let at = std::env::var("MSQU_AT").unwrap_or_default();
            if !load(&st).iter().any(|n| n.id == id && n.at > 0 && (at.is_empty() || at == n.at.to_string())) {
                return 0;
            }
            remind(&mut st, id);
        }
        "rearm" => {
            for n in load(&st) {
                if n.at <= 0 {
                    continue;
                }
                slot(&mut st, n.id);
                if n.at > now() {
                    let _ = st.arm(n.at);
                } else {
                    // missed while this was off
                    remind(&mut st, n.id);
                    st.relock();
                }
            }
        }
        "running" => return if load(&st).iter().any(|n| n.at > 0) { 0 } else { 1 },
        "right-click" => {
            let mut notes = load(&st);
            let Some(n) = notes.iter_mut().filter(|n| n.at > 0).min_by_key(|n| n.at) else { return 0 };
            n.at = 0;
            let id = n.id;
            slot(&mut st, id);
            st.disarm();
            save(&st, &notes);
        }
        "middle-click" => crate::model::set_mark("hide", "notes", FILE, true),
        "click" => {}
        _ => {
            eprintln!("usage: msqu notes [add UNIXTIME|0 TEXT|done ID|rearm|running]");
            return 2;
        }
    }
    0
}
