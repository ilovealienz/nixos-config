//! group-end ends a tab group that is left with one window.
//!
//! When a tabbed or stacked container (however it was made: app-group,
//! group-drag, Super+g) goes from two or more windows down to one, the
//! container is removed and its window put in its place (`split none`):
//! it acts like any other, and new windows open beside it rather than as
//! tabs. If the other was closed (Super+q) that happens at once; if it was
//! moved out or floated (Super+Shift+hjkl, a drag, Super+v, another
//! workspace), the group is kept for GRACE first, so it can be put back.
//! Floating groups are left alone.
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

enum Msg {
    Binding, // a key binding ran
    Closed,  // a window closed
    Changed, // a window opened, moved or floated
}

#[derive(Default)]
struct State {
    seen: HashMap<i64, (usize, i64)>, // the groups as last read
    ends: HashMap<i64, Instant>,      // group down to one -> when it ends
}

impl State {
    /// Reads the tree: a group down to one from two or more gets its time,
    /// or after a window closed, is ended now.
    fn update(&mut self, sway: &mut Sway, closed: bool) {
        let root = sway.tree();
        let now = groups(&root);
        drop(root);
        trim();
        let t = Instant::now();
        let mut end_now = Vec::new();
        for (id, &(tabs, tab)) in &now {
            let before = self.seen.get(id).map_or(0, |g| g.0);
            if tabs >= 2 {
                self.ends.remove(id);
            } else if before >= 2 && closed {
                end_now.push((*id, tab));
            } else if before >= 2 {
                self.ends.entry(*id).or_insert(t + GRACE);
            }
        }
        self.ends.retain(|id, _| now.contains_key(id));
        self.seen = now;
        self.end(sway, end_now);
    }

    /// Removes groups, leaving their one tab in their place: (group, tab).
    fn end(&mut self, sway: &mut Sway, groups: Vec<(i64, i64)>) {
        if groups.is_empty() {
            return;
        }
        // `split none` on a container's only child removes the container;
        // a plain `layout splith` would leave it there, wrapped around the
        // one window, and the next move would only take it out of that
        let c: Vec<String> = groups.iter().map(|(_, tab)| format!("[con_id={tab}] split none")).collect();
        for (id, _) in &groups {
            self.ends.remove(id);
            self.seen.remove(id);
        }
        sway.run(&c.join("; "));
    }

    /// Ends the groups whose time is up and still have one window.
    fn end_due(&mut self, sway: &mut Sway) {
        self.update(sway, false);
        let t = Instant::now();
        let due: Vec<i64> = self.ends.iter().filter(|(_, &at)| at <= t).map(|(&id, _)| id).collect();
        let mut groups = Vec::new();
        for id in due {
            self.ends.remove(&id);
            if let Some(&(1, tab)) = self.seen.get(&id) {
                groups.push((id, tab));
            }
        }
        self.end(sway, groups);
    }
}

pub fn run(_cfg: &Config) {
    let mut sway = Sway::connect(NAME);
    let mut events = Sway::connect(NAME).subscribe(r#"["window","binding"]"#);
    let _closer = events.closer(); // ends the reader if this returns or panics
    let (tx, rx) = mpsc::channel::<Msg>();
    thread::Builder::new()
        .name("group-end-ev".into())
        .stack_size(STACK)
        .spawn(move || {
            while let Some((typ, body)) = events.next() {
                let msg = match typ {
                    BINDING_EVENT => Msg::Binding,
                    WINDOW_EVENT => match WindowEvent::from(&body) {
                        Some(ev) if ev.change == "close" => Msg::Closed,
                        Some(ev) if matches!(ev.change.as_str(), "new" | "move" | "floating") => Msg::Changed,
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
    st.update(&mut sway, false);
    let mut reread: Option<Instant> = None; // after a binding

    loop {
        // sleep until an event, a re-read after a binding, or a group's time
        let wake = st.ends.values().copied().chain(reread).min();
        let got = match wake {
            Some(at) => rx.recv_timeout(at.saturating_duration_since(Instant::now())),
            None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        match got {
            Ok(Msg::Binding) => reread = Some(Instant::now() + AFTER_BINDING),
            Ok(Msg::Closed) => st.update(&mut sway, true),
            Ok(Msg::Changed) => st.update(&mut sway, false),
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
