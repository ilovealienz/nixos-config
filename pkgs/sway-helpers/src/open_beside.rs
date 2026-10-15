//! open-beside keeps new windows from opening underneath another.
//!
//! Sway opens a new window next to the focused one, in its container's
//! direction. After a window is moved up or down (Super+Shift+j/k, which
//! also steps it out of a tab group), the windows there are in a vertical
//! container, and later windows opened from them would go underneath. A
//! new tiled window that lands in a vertical container is moved to the
//! right of that container instead: beside, full height. If the container
//! is left with one window, it is removed (`split none`) so it doesn't get
//! in the way of later moves. Sway focuses a window moved this way, so for
//! one that opened without focus (assigned to another workspace, say) the
//! focus is put back where it was.
//!
//! Choosing a split with a key binding (Super+s, Super+t: any binding
//! whose command has "split" in it, other than an exec) lets the next new
//! window open where sway puts it, below included, unless the focus moves
//! to another window first.
//!
//! Runs in app-group's thread, on the same "new" events and tree read, so
//! a window app-group puts in a tab group is left to it.

use crate::json::Node;

/// The container holding the tiled window `id` (a workspace or a con).
fn parent_of(n: &Node, id: i64) -> Option<&Node> {
    if n.is_scratchpad() {
        return None;
    }
    if n.nodes.iter().any(|c| c.id == id) {
        return Some(n);
    }
    // tiled only: floating containers are left alone
    n.nodes.iter().find_map(|c| parent_of(c, id))
}

/// Whether the binding's command chooses a split by hand.
pub fn chooses_split(command: &str) -> bool {
    !command.trim_start().starts_with("exec") && command.contains("split")
}

/// The command that focuses what has focus now: a window, or an empty
/// workspace.
fn refocus(root: &Node) -> Option<String> {
    let mut out = None;
    root.each(&mut |n| {
        if n.focused && out.is_none() {
            out = Some(if n.kind == "workspace" {
                let name = n.name.replace('\\', "\\\\").replace('"', "\\\"");
                format!("workspace --no-auto-back-and-forth \"{name}\"")
            } else {
                format!("[con_id={}] focus", n.id)
            });
        }
    });
    out
}

/// The sway commands that move tiled window `id` out from under the
/// others, if it opened in a vertical container with another window.
pub fn command(root: &Node, id: i64) -> Option<String> {
    let parent = parent_of(root, id)?;
    if parent.layout != "splitv" || parent.nodes.len() < 2 {
        return None;
    }
    let mut c = format!("[con_id={id}] move right");
    if parent.nodes.len() == 2 {
        // the one left: take away the container around it
        let other = parent.nodes.iter().find(|n| n.id != id)?;
        c += &format!("; [con_id={}] split none", other.id);
    }
    // the move focuses it: if it opened without focus, put the focus back
    if !parent.nodes.iter().any(|n| n.id == id && n.focused) {
        if let Some(f) = refocus(root) {
            c += &format!("; {f}");
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(json: &str) -> Node {
        Node::from(json.as_bytes()).unwrap()
    }

    #[test]
    fn commands() {
        let ws = |layout: &str, nodes: &str| {
            tree(&format!(
                r#"{{"id":1,"type":"root","nodes":[{{"id":2,"type":"output","nodes":[{{"id":3,"type":"workspace","name":"1","layout":"{layout}","nodes":[{nodes}],"floating_nodes":[]}}],"floating_nodes":[]}}],"floating_nodes":[]}}"#
            ))
        };
        let win = |id: i64| format!(r#"{{"id":{id},"type":"con","app_id":"a","layout":"none","nodes":[],"floating_nodes":[]}}"#);
        // two in a vertical workspace: moved, and the one left unwrapped
        let t = ws("splitv", &format!("{},{}", win(10), win(11)));
        assert_eq!(command(&t, 11).as_deref(), Some("[con_id=11] move right; [con_id=10] split none"));
        // three: just moved
        let t = ws("splitv", &format!("{},{},{}", win(10), win(11), win(12)));
        assert_eq!(command(&t, 12).as_deref(), Some("[con_id=12] move right"));
        // side by side, or alone: left alone
        assert_eq!(command(&ws("splith", &format!("{},{}", win(10), win(11))), 11), None);
        assert_eq!(command(&ws("splitv", &win(10)), 10), None);
        // in a tab group: left alone
        let g = format!(r#"{{"id":20,"type":"con","layout":"tabbed","nodes":[{},{}],"floating_nodes":[]}}"#, win(10), win(11));
        assert_eq!(command(&ws("splith", &g), 11), None);
        // in a vertical column beside another window
        let col = format!(r#"{{"id":20,"type":"con","layout":"splitv","nodes":[{},{}],"floating_nodes":[]}}"#, win(10), win(11));
        let t = ws("splith", &format!("{},{col}", win(9)));
        assert_eq!(command(&t, 11).as_deref(), Some("[con_id=11] move right; [con_id=10] split none"));
        // opened without focus: the focus goes back to what had it
        let focused = r#"{"id":30,"type":"con","app_id":"b","focused":true,"layout":"none","nodes":[],"floating_nodes":[]}"#;
        let t = ws("splith", &format!(r#"{focused},{{"id":20,"type":"con","layout":"splitv","nodes":[{},{},{}],"floating_nodes":[]}}"#, win(10), win(11), win(12)));
        assert_eq!(command(&t, 11).as_deref(), Some("[con_id=11] move right; [con_id=30] focus"));
        let empty = r#"{"id":5,"type":"workspace","name":"say \"hi\"","focused":true,"layout":"splith","nodes":[],"floating_nodes":[]}"#;
        let t = tree(&format!(
            r#"{{"id":1,"type":"root","nodes":[{{"id":2,"type":"output","nodes":[{empty},{{"id":3,"type":"workspace","name":"2","layout":"splitv","nodes":[{},{}],"floating_nodes":[]}}],"floating_nodes":[]}}],"floating_nodes":[]}}"#,
            win(10),
            win(11)
        ));
        assert_eq!(
            command(&t, 11).as_deref(),
            Some(r#"[con_id=11] move right; [con_id=10] split none; workspace --no-auto-back-and-forth "say \"hi\"""#)
        );
        assert!(chooses_split("split toggle"));
        assert!(chooses_split("layout toggle split"));
        assert!(!chooses_split("exec /nix/store/x-split-thing"));
        assert!(!chooses_split("nop group-drag"));
    }
}
