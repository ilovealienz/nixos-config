//! group-drag: the rest of a tab group follows a window dragged with
//! Super+Shift.
//!
//! The "nop group-drag" binding arms it and snapshots the groups; while armed
//! (and 3s after, as the drag can continue) the layout is polled every 120ms.
//! A window that left its group gets the others moved to it. Keyboard moves
//! don't count: a binding within 150ms disarms it.

use crate::ipc::{Sway, BINDING_EVENT};
use crate::json::{binding_command, Node};
use crate::{trim, Config, STACK};
use std::collections::HashMap;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

const NAME: &str = "group-drag";
const ARM_FOR: Duration = Duration::from_secs(30); // in case a key release is missed
const GRACE: Duration = Duration::from_secs(3); // keys can be let go before the drop
const POLL: Duration = Duration::from_millis(120);
const SETTLE: Duration = Duration::from_millis(150);
const ARM_CMD: &str = "nop group-drag";
const OFF_CMD: &str = "nop group-drag-off";
const MARK: &str = "__grpfollow";

// ── snapshots of the tiled layout ──

#[derive(Default)]
struct Snapshot {
    parent: HashMap<i64, i64>,    // window or container -> its parent
    kids: HashMap<i64, Vec<i64>>, // container -> child ids, in order
    window: HashMap<i64, bool>,   // ids that are windows
    group: HashMap<i64, bool>,    // tabbed/stacked containers (not workspaces)
}

impl Snapshot {
    fn is_group(&self, id: i64) -> bool {
        self.group.get(&id).copied().unwrap_or(false)
    }
    fn is_window(&self, id: i64) -> bool {
        self.window.get(&id).copied().unwrap_or(false)
    }
}

fn walk(n: &Node, s: &mut Snapshot) {
    // a workspace with a tabbed layout isn't a group: dragging one window
    // off it shouldn't pull every other window along
    s.group.insert(n.id, n.kind == "con" && (n.layout == "tabbed" || n.layout == "stacked"));
    for c in &n.nodes {
        // tiled children only: floating windows are skipped
        s.kids.entry(n.id).or_default().push(c.id);
        s.parent.insert(c.id, n.id);
        s.window.insert(c.id, c.is_window());
        walk(c, s);
    }
}

fn take(sway: &mut Sway) -> Snapshot {
    let root = sway.tree();
    let mut s = Snapshot::default();
    for output in &root.nodes {
        for ws in &output.nodes {
            if !ws.is_scratchpad() {
                walk(ws, &mut s);
            }
        }
    }
    s
}

// a window that left its group while the rest stayed behind
struct Follow {
    win: i64,
    group: i64,
    all: Vec<i64>, // the whole group before the drag
}

fn detect(before: &Snapshot, now: &Snapshot) -> Option<Follow> {
    for (&g, members) in &before.kids {
        // members are windows, or split containers holding windows (a tab
        // can be a split); only a window can be dragged out
        if !before.is_group(g) || members.len() < 2 {
            continue;
        }
        let (mut left, mut stay) = (Vec::new(), 0);
        for &m in members {
            match now.parent.get(&m) {
                None => {} // closed, floated, or an emptied container
                Some(&p) if p == g => stay += 1,
                Some(_) if before.is_window(m) => left.push(m),
                Some(_) => {}
            }
        }
        if left.len() == 1 && stay >= 1 {
            return Some(Follow { win: left[0], group: g, all: members.clone() });
        }
    }
    None
}

fn apply(sway: &mut Sway, f: &Follow) {
    let now = take(sway);
    let Some(&p) = now.parent.get(&f.win) else { return };
    let mut cmds: Vec<String> = Vec::new();
    if !now.is_group(p) {
        // wrap the dropped window in a new tabbed container (made from
        // splith: sway remembers it, so untabbing goes side by side)
        cmds.push(format!("[con_id={}] splith", f.win));
        cmds.push(format!("[con_id={}] layout tabbed", f.win));
    }
    // rebuild the old order around the dropped window: the ones that were
    // before it go after it first, then it moves to after the last of them
    let (mut pre, mut post) = (Vec::new(), Vec::new());
    let mut seen = false;
    for &m in &f.all {
        if m == f.win {
            seen = true;
        } else if now.parent.get(&m) != Some(&f.group) {
            // gone or moved elsewhere meanwhile
        } else if seen {
            post.push(m);
        } else {
            pre.push(m);
        }
    }
    let mark = |id: i64| format!("[con_id={id}] mark --add {MARK}");
    let move_after = |cmds: &mut Vec<String>, ids: &[i64]| {
        for id in ids.iter().rev() {
            // each lands right after the mark
            cmds.push(format!("[con_id={id}] move container to mark {MARK}"));
        }
    };
    cmds.push(mark(f.win));
    move_after(&mut cmds, &pre);
    if let Some(&last) = pre.last() {
        cmds.push(mark(last));
        cmds.push(format!("[con_id={}] move container to mark {MARK}", f.win));
        cmds.push(mark(f.win));
    }
    move_after(&mut cmds, &post);
    cmds.push(format!("unmark {MARK}"));
    cmds.push(format!("[con_id={}] focus", f.win));
    sway.run(&cmds.join("; "));
    eprintln!("{NAME}: group {} followed window {}", f.group, f.win);

    // after a swap the other window sits alone in the old group: untab it
    let after = take(sway);
    if let Some(kids) = after.kids.get(&f.group) {
        if kids.len() == 1 && after.is_group(f.group) && !f.all.contains(&kids[0]) {
            sway.run(&format!("[con_id={}] layout splith; [con_id={}] focus", kids[0], f.win));
        }
    }
}

pub fn run(_cfg: &Config) {
    let mut sway = Sway::connect(NAME);
    // a mark left behind if this stopped halfway through a move
    sway.run_quiet(&format!("unmark {MARK}"));

    let mut events = Sway::connect(NAME).subscribe(r#"["binding"]"#);
    let _closer = events.closer(); // ends the reader if this returns or panics
    let (tx, rx) = mpsc::channel::<String>();
    thread::Builder::new()
        .name("group-drag-ev".into())
        .stack_size(STACK)
        .spawn(move || {
            while let Some((typ, body)) = events.next() {
                if typ != BINDING_EVENT {
                    continue;
                }
                if let Some(c) = binding_command(&body) {
                    if tx.send(c.trim().to_string()).is_err() {
                        return;
                    }
                }
            }
        })
        .expect("start event reader");

    let mut active = false; // checking the layout every POLL
    let mut armed = false; // Super+Shift held
    let mut armed_till = Instant::now();
    let mut grace_till = Instant::now();
    let mut before = Snapshot::default();
    let mut pending: Option<(Follow, Instant)> = None;
    let mut next_tick = Instant::now();

    loop {
        // sleep until a binding event, or the next check while active
        let got = if active {
            rx.recv_timeout(next_tick.saturating_duration_since(Instant::now()))
        } else {
            rx.recv().map_err(|_| RecvTimeoutError::Disconnected)
        };
        match got {
            Ok(c) if c == ARM_CMD => {
                before = take(&mut sway);
                trim();
                armed = true;
                armed_till = Instant::now() + ARM_FOR;
                pending = None;
                if !active {
                    active = true;
                    next_tick = Instant::now() + POLL;
                }
            }
            Ok(c) if c == OFF_CMD => {
                // keys let go: sway keeps dragging if the mouse button is
                // still held, so keep watching a little longer for the drop
                if armed {
                    armed = false;
                    grace_till = Instant::now() + GRACE;
                }
            }
            Ok(_) => {
                // a keyboard action: whatever moved, it wasn't a drag
                active = false;
                armed = false;
                pending = None;
                before = Snapshot::default();
                trim();
            }
            Err(RecvTimeoutError::Disconnected) => panic!("event reader stopped"),
            Err(RecvTimeoutError::Timeout) => {
                let t = Instant::now();
                next_tick = t + POLL;
                if let Some((f, at)) = &pending {
                    if t.duration_since(*at) >= SETTLE {
                        apply(&mut sway, f);
                        pending = None;
                        before = take(&mut sway);
                        if !armed {
                            active = false;
                            before = Snapshot::default();
                        }
                        trim();
                    }
                    continue;
                }
                if (armed && t > armed_till) || (!armed && t > grace_till) {
                    active = false;
                    before = Snapshot::default();
                    trim();
                    continue;
                }
                let now = take(&mut sway);
                match detect(&before, &now) {
                    Some(f) => pending = Some((f, t)),
                    None => before = now,
                }
                trim();
            }
        }
    }
}
