//! group-end ends a tab group 10 seconds after it is left with one window.
//!
//! When a tabbed or stacked container (however it was made: app-group,
//! group-drag, Super+g) goes from two or more windows down to one, its
//! others closed, moved out or floated, it is kept for GRACE, so a window
//! taken out can be put back (Super+Shift+k, or dragged). If it still has
//! one window then, it becomes a plain side-by-side split: that window acts
//! like any other, and new windows open beside it rather than as tabs.
//!
//! A group made with one window on purpose (Super+g, to open windows into)
//! is left alone until it has had two. Sway reports no event when a group
//! is made, so the tree is also read shortly after each key binding, which
//! is how groups made by Super+g are seen.
//!
//! Sway handles a run_command request completely before the next request,
//! so a tree read never shows another program's commands half done
//! (app-group or group-drag building a group, say).

use crate::ipc::{Sway, BINDING_EVENT, WINDOW_EVENT};
use crate::json::{Node, WindowEvent};
use crate::{trim, Config, STACK};
use std::collections::HashMap;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

const NAME: &str = "group-end";
/// how long a group left with one window is kept
const GRACE: Duration = Duration::from_secs(10);
/// a binding's command (Super+g's script, say) has had time to run
const AFTER_BINDING: Duration = Duration::from_millis(300);

/// Tab groups (workspaces aren't groups): id -> (tabs, the first tab).
fn groups(root: &Node) -> HashMap<i64, (usize, i64)> {
    let mut out = HashMap::new();
    root.each(&mut |n| {
        if n.kind == "con" && (n.layout == "tabbed" || n.layout == "stacked") && !n.nodes.is_empty() {
            out.insert(n.id, (n.nodes.len(), n.nodes[0].id));
        }
    });
    out
}

#[derive(Default)]
struct State {
    seen: HashMap<i64, (usize, i64)>, // the groups as last read
    ends: HashMap<i64, Instant>,      // group down to one -> when it ends
}

impl State {
    /// Reads the tree: a group down to one from two or more gets its time.
    fn update(&mut self, sway: &mut Sway) {
        let root = sway.tree();
        let now = groups(&root);
        drop(root);
        trim();
        let t = Instant::now();
        for (id, &(tabs, _)) in &now {
            let before = self.seen.get(id).map_or(0, |g| g.0);
            if tabs >= 2 {
                self.ends.remove(id);
            } else if before >= 2 {
                self.ends.entry(*id).or_insert(t + GRACE);
            }
        }
        self.ends.retain(|id, _| now.contains_key(id));
        self.seen = now;
    }

    /// Ends the groups whose time is up and still have one window.
    fn end_due(&mut self, sway: &mut Sway) {
        self.update(sway);
        let t = Instant::now();
        let due: Vec<i64> = self.ends.iter().filter(|(_, &at)| at <= t).map(|(&id, _)| id).collect();
        let mut c = Vec::new();
        for id in due {
            self.ends.remove(&id);
            if let Some(&(1, tab)) = self.seen.get(&id) {
                // a tab's layout command sets its group's layout
                c.push(format!("[con_id={tab}] layout splith"));
                self.seen.remove(&id);
            }
        }
        if !c.is_empty() {
            sway.run(&c.join("; "));
        }
    }
}

pub fn run(_cfg: &Config) {
    let mut sway = Sway::connect(NAME);
    let mut events = Sway::connect(NAME).subscribe(r#"["window","binding"]"#);
    let _closer = events.closer(); // ends the reader if this returns or panics
    let (tx, rx) = mpsc::channel::<bool>(); // true: a binding, false: a window change
    thread::Builder::new()
        .name("group-end-ev".into())
        .stack_size(STACK)
        .spawn(move || {
            while let Some((typ, body)) = events.next() {
                let msg = match typ {
                    BINDING_EVENT => true,
                    WINDOW_EVENT => match WindowEvent::from(&body) {
                        Some(ev) if matches!(ev.change.as_str(), "new" | "close" | "move" | "floating") => false,
                        _ => continue,
                    },
                    _ => continue,
                };
                if tx.send(msg).is_err() {
                    return;
                }
            }
        })
        .expect("start event reader");

    let mut st = State::default();
    st.update(&mut sway);
    let mut reread: Option<Instant> = None; // after a binding

    loop {
        // sleep until an event, a re-read after a binding, or a group's time
        let wake = st.ends.values().copied().chain(reread).min();
        let got = match wake {
            Some(at) => rx.recv_timeout(at.saturating_duration_since(Instant::now())),
            None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        match got {
            Ok(true) => reread = Some(Instant::now() + AFTER_BINDING),
            Ok(false) => st.update(&mut sway),
            Err(RecvTimeoutError::Disconnected) => panic!("event reader stopped"),
            Err(RecvTimeoutError::Timeout) => {
                if reread.is_some_and(|at| at <= Instant::now()) {
                    reread = None;
                }
                st.end_due(&mut sway);
            }
        }
    }
}
