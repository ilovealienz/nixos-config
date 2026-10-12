//! app-group puts related apps into tabbed groups when they open.
//!
//! Each group is a list of apps. When a window from a group opens tiled,
//! it is moved into the tabbed group the other windows of that group are
//! in (on whatever workspace that is). If another window from the group is
//! open but not grouped yet, a tabbed group is made around it first. This
//! only happens when a window opens; after that the windows are left alone,
//! so they can be dragged out of the group.
//!
//! APP is a Wayland app_id or XWayland class, matched case-insensitively.
//! Grouping is paused while $XDG_STATE_HOME/app-group/off exists (toggled
//! by app-group-toggle).
//!
//! Sway sends the "new" event before it applies floating and for_window
//! rules and before it focuses the window, so whether the window ended up
//! tiled and focused is read from the tree fetched afterwards.

use crate::ipc::{Sway, WINDOW_EVENT};
use crate::json::{Node, WindowEvent};
use crate::{state_file, trim, Config};

const NAME: &str = "app-group";

fn group_of<'a>(cfg: &'a Config, n: &Node) -> Option<&'a str> {
    let app = n.app().to_lowercase();
    cfg.groups
        .iter()
        .find(|(_, apps)| apps.contains(&app))
        .map(|(name, _)| name.as_str())
}

/// The new window as the tree has it now: whether it is tiled (not
/// floating or in the scratchpad), and whether it has focus.
fn placed(root: &Node, id: i64) -> Option<(bool, bool)> {
    fn find(n: &Node, id: i64, tiled: bool) -> Option<(bool, bool)> {
        if n.is_scratchpad() {
            return find_in(n, id, false);
        }
        for c in &n.nodes {
            if c.id == id {
                return Some((tiled, c.focused));
            }
            if let Some(r) = find(c, id, tiled) {
                return Some(r);
            }
        }
        for c in &n.floating_nodes {
            if c.id == id {
                return Some((false, c.focused));
            }
            if let Some(r) = find(c, id, false) {
                return Some(r);
            }
        }
        None
    }
    fn find_in(n: &Node, id: i64, tiled: bool) -> Option<(bool, bool)> {
        for c in n.nodes.iter().chain(&n.floating_nodes) {
            if c.id == id {
                return Some((tiled, c.focused));
            }
            if let Some(r) = find_in(c, id, tiled) {
                return Some(r);
            }
        }
        None
    }
    find(root, id, true)
}

/// Another tiled window from group `g` to join, preferring one already in
/// a tabbed or stacked container: (its id, whether it is grouped).
fn target(cfg: &Config, root: &Node, skip: i64, g: &str) -> Option<(i64, bool)> {
    fn walk(cfg: &Config, n: &Node, skip: i64, g: &str, best: &mut Option<(i64, bool)>) {
        if n.is_scratchpad() {
            return;
        }
        for c in &n.nodes {
            // tiled children only
            if c.id != skip && c.nodes.is_empty() && c.is_window() && group_of(cfg, c) == Some(g) {
                let grouped = n.layout == "tabbed" || n.layout == "stacked";
                if best.is_none() || (grouped && !best.unwrap().1) {
                    *best = Some((c.id, grouped));
                }
            }
            walk(cfg, c, skip, g, best);
        }
    }
    let mut best = None;
    walk(cfg, root, skip, g, &mut best);
    best
}

fn paused() -> bool {
    state_file("app-group/off").exists()
}

pub fn run(cfg: &Config) {
    let mut sway = Sway::connect(NAME);
    // marks left behind if this stopped halfway through a move
    for (g, _) in &cfg.groups {
        sway.run_quiet(&format!("unmark __app_group_{g}"));
    }
    let mut events = Sway::connect(NAME).subscribe(r#"["window"]"#);

    // read events as they come: everything here is quick, and sway drops
    // a client that falls far behind
    while let Some((typ, body)) = events.next() {
        if typ != WINDOW_EVENT {
            continue;
        }
        let Some(ev) = WindowEvent::from(&body) else { continue };
        drop(body);
        if ev.change != "new" {
            continue;
        }
        let w = ev.container;
        let Some(g) = group_of(cfg, &w) else { continue };
        if paused() {
            continue;
        }

        let root = sway.tree();
        let found = placed(&root, w.id);
        let tgt = target(cfg, &root, w.id, g);
        drop(root);
        trim();
        let Some((tiled, focused)) = found else { continue }; // already closed
        if !tiled {
            continue; // floating windows and dialogs are left alone
        }
        let Some((t, grouped)) = tgt else { continue }; // first window of its group

        let mark = format!("__app_group_{g}");
        let mut c = String::new();
        if !grouped {
            // wrap the existing window in its own tabbed container
            c += &format!("[con_id={t}] splitv; [con_id={t}] layout tabbed; ");
        }
        c += &format!("[con_id={t}] mark --add {mark}; ");
        c += &format!("[con_id={}] move container to mark {mark}; ", w.id);
        c += &format!("[con_id={t}] unmark {mark}");
        if focused {
            c += &format!("; [con_id={}] focus", w.id);
        }
        sway.run(&c);
    }
}
