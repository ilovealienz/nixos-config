//! attention: sway ignores X11 _NET_WM_STATE_DEMANDS_ATTENTION, so this
//! hears it on the XWayland root window and shows "(N) <title>"; with
//! $XDG_STATE_HOME/xwayland-attention/urgent it also marks the window urgent.
//! Focusing resets it.
//!
//! Only connects while an X11 window is open, with XFixes disconnect mode,
//! so XWayland can still exit.

use crate::ipc::{Sway, WINDOW_EVENT};
use crate::json::{Node, WindowEvent};
use crate::x11::{Detach, XCloser, X};
use crate::{state_file, trim, Config, STACK};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::sync::mpsc::{self, Sender};
use std::thread;

const NAME: &str = "attention";

const ACTION_ADD: u32 = 1;
const ACTION_TOGGLE: u32 = 2;

enum Msg {
    /// a sway window event: change, X11 window id (0 if not X11), whether
    /// it's an XWayland window
    Window(String, u32, bool),
    /// an X11 window asked for attention
    Attention(u32),
    /// the X connection with this number ended (XWayland exited)
    XGone(u64),
}

struct State {
    sway: Sway,
    suffix: String,         // added after %title, quotes escaped
    counts: HashMap<u32, u32>, // X11 window -> attention requests
    focused: u32,           // X11 id of the focused window, 0 if not X11
    x: Option<XCloser>,     // the open X connection, if any
    x_num: u64,             // number of the current X connection
    tx: Sender<Msg>,
}

fn urgent_on() -> bool {
    state_file("xwayland-attention/urgent").exists()
}

/// Whether sway has any XWayland window, and the X11 id of the focused one.
fn xwayland_windows(root: &Node) -> (bool, u32) {
    let (mut any, mut focused) = (false, 0);
    root.each(&mut |n| {
        if n.shell == "xwayland" {
            any = true;
            if n.focused {
                focused = n.window;
            }
        }
    });
    (any, focused)
}

impl State {
    fn attention(&mut self, wid: u32) {
        if wid == self.focused {
            return;
        }
        let n = self.counts.entry(wid).or_insert(0);
        *n += 1;
        let mut c = format!(r#"[id={wid}] title_format "({n}) %title{}""#, self.suffix);
        if urgent_on() {
            c += ", urgent enable";
        }
        self.sway.run(&c);
    }

    fn focus(&mut self, wid: u32) {
        self.focused = wid;
        if self.counts.remove(&wid).is_some() {
            self.sway.run(&format!(r#"[id={wid}] title_format "%title{}""#, self.suffix));
        }
    }

    /// Connects to XWayland while an X11 window is open.
    /// Whether sway has an XWayland window open; also notes which one
    /// has focus.
    fn any_x_windows(&mut self) -> bool {
        let root = self.sway.tree();
        let (any, focused) = xwayland_windows(&root);
        drop(root);
        trim();
        self.focused = focused;
        any
    }

    /// Connects to XWayland if it isn't already, as long as it is running
    /// with an X11 window open.
    fn connect_x(&mut self) {
        if self.x.is_some() {
            return;
        }
        // connecting to a stopped XWayland starts it again, and then only
        // another X11 app coming and going would let it exit
        if !self.any_x_windows() || !xwayland_running() {
            return;
        }
        let mut x = match open_x() {
            Ok(x) => x,
            Err(e) => {
                eprintln!("{NAME}: XWayland: {e}");
                return;
            }
        };
        // the last X11 app may have closed in the meantime
        if !self.any_x_windows() {
            return; // dropping x disconnects
        }
        let (atoms, closer) = match listen(&mut x) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("{NAME}: XWayland: {e}");
                return;
            }
        };
        self.x_num += 1;
        let (num, tx) = (self.x_num, self.tx.clone());
        let started = thread::Builder::new()
            .name("attention-x".into())
            .stack_size(STACK)
            .spawn(move || read_x(x, atoms, num, tx));
        match started {
            Ok(_) => self.x = Some(closer),
            Err(e) => eprintln!("{NAME}: can't start thread: {e}"),
        }
    }
}

/// Whether an Xwayland process of this user is running.
fn xwayland_running() -> bool {
    let Ok(me) = fs::metadata("/proc/self") else { return true };
    let Ok(dir) = fs::read_dir("/proc") else { return true };
    dir.flatten().any(|e| {
        e.metadata().is_ok_and(|m| m.uid() == me.uid())
            && fs::read(e.path().join("comm")).is_ok_and(|c| c == b"Xwayland\n")
    })
}

/// Connects and lets XWayland exit regardless of this connection.
fn open_x() -> io::Result<X> {
    let mut x = X::connect()?;
    match x.detach()? {
        Detach::Done => {}
        // stays connected like before; XWayland just won't exit on its own
        Detach::Unsupported(why) => eprintln!("{NAME}: XWayland will stay running: {why}"),
    }
    Ok(x)
}

/// Starts listening on the root window: (_NET_WM_STATE atoms, closer).
fn listen(x: &mut X) -> io::Result<((u32, u32), XCloser)> {
    x.listen_root()?;
    let state = x.intern_atom("_NET_WM_STATE")?;
    let demands = x.intern_atom("_NET_WM_STATE_DEMANDS_ATTENTION")?;
    Ok(((state, demands), x.closer()?))
}

fn read_x(mut x: X, (state, demands): (u32, u32), num: u64, tx: Sender<Msg>) {
    loop {
        match x.client_message() {
            Ok((wid, typ, format, d)) => {
                if typ == state
                    && format == 32
                    && (d[0] == ACTION_ADD || d[0] == ACTION_TOGGLE)
                    && (d[1] == demands || d[2] == demands)
                    && tx.send(Msg::Attention(wid)).is_err()
                {
                    return;
                }
            }
            Err(_) => {
                let _ = tx.send(Msg::XGone(num));
                return;
            }
        }
    }
}

pub fn run(cfg: &Config) {
    let (tx, rx) = mpsc::channel();

    // sway's window events, read on their own so they never pile up
    let mut events = Sway::connect(NAME).subscribe(r#"["window"]"#);
    let _closer = events.closer(); // ends the reader if this returns or panics
    let ev_tx = tx.clone();
    thread::Builder::new()
        .name("attention-ev".into())
        .stack_size(STACK)
        .spawn(move || {
            while let Some((typ, body)) = events.next() {
                if typ != WINDOW_EVENT {
                    continue;
                }
                let Some(ev) = WindowEvent::from(&body) else { continue };
                let c = ev.container;
                let msg = Msg::Window(ev.change, c.window, c.shell == "xwayland");
                if ev_tx.send(msg).is_err() {
                    return;
                }
            }
        })
        .expect("start event reader");

    let mut s = State {
        sway: Sway::connect(NAME),
        suffix: cfg.title_suffix.replace('"', "\\\""),
        counts: HashMap::new(),
        focused: 0,
        x: None,
        x_num: 0,
        tx,
    };

    if s.any_x_windows() {
        // counts set before a restart would otherwise never be cleared
        s.sway.run_quiet(&format!(r#"[shell="xwayland"] title_format "%title{}""#, s.suffix));
        s.connect_x();
    }

    for msg in rx {
        match msg {
            Msg::Window(change, wid, xwayland) => match change.as_str() {
                "new" if xwayland => s.connect_x(),
                "focus" => s.focus(wid),
                "close" if wid != 0 => {
                    s.counts.remove(&wid);
                }
                _ => {}
            },
            Msg::Attention(wid) => s.attention(wid),
            Msg::XGone(num) if num == s.x_num && s.x.is_some() => {
                // XWayland exited: every X11 window is gone with it
                eprintln!("{NAME}: XWayland exited");
                s.x = None;
                s.counts.clear();
                // an X11 app opened while it was exiting starts a new one
                s.connect_x();
            }
            Msg::XGone(_) => {}
        }
    }
}
