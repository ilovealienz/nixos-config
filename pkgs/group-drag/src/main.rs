//! group-drag makes the rest of a tab group follow a window dragged with
//! Super+Shift.
//!
//! Sway's own drag only moves one window, so this watches what it does:
//! pressing Shift while Super is held (or Super while Shift is held) runs a
//! sway binding "nop group-drag", which arms this and takes a snapshot of
//! the tab groups; releasing it runs "nop group-drag-off". While armed (and
//! for 3 seconds after the keys are let go, since sway keeps dragging while
//! the mouse button is held) it checks the layout every 120ms. If one
//! window has left its tabbed or stacked group and the others are still
//! there, the others (windows, or splits holding windows) are moved to join
//! it, in their old order, as a tabbed group. If the window was dropped
//! onto the middle of another window, sway swaps the two; the window that
//! ended up in the old group is then turned back into a plain split.
//!
//! Keyboard moves never trigger it: any other binding disarms it, and a
//! change is only acted on if no binding follows it within 150ms (sway
//! reports a binding after its command has run).
//!
//! Standard library only (with a small JSON reader for sway's replies).
//! Talks to sway over its IPC socket, sleeps until a binding event, and
//! hands freed memory back to the system after each check.

use std::collections::HashMap;
use std::env;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::process::exit;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

const RUN_COMMAND: u32 = 0;
const SUBSCRIBE: u32 = 2;
const GET_TREE: u32 = 4;
const BINDING_EVENT: u32 = 0x8000_0005;

const ARM_FOR: Duration = Duration::from_secs(30); // in case a key release is missed
const GRACE: Duration = Duration::from_secs(3); // keys can be let go before the drop
const POLL: Duration = Duration::from_millis(120);
const SETTLE: Duration = Duration::from_millis(150);
const ARM_CMD: &str = "nop group-drag";
const OFF_CMD: &str = "nop group-drag-off";

extern "C" {
    // glibc: give freed heap memory back to the system
    fn malloc_trim(pad: usize) -> i32;
}

fn trim() {
    unsafe {
        malloc_trim(0);
    }
}

fn die(msg: &str) -> ! {
    eprintln!("group-drag: {msg}");
    exit(1) // systemd restarts us
}

// ── sway IPC: "i3-ipc" + u32 length + u32 type + payload ──

fn connect() -> UnixStream {
    let path = env::var("SWAYSOCK").unwrap_or_else(|_| die("SWAYSOCK is not set"));
    UnixStream::connect(path).unwrap_or_else(|e| die(&format!("connect to sway: {e}")))
}

fn send(s: &mut UnixStream, typ: u32, payload: &str) {
    let mut buf = Vec::with_capacity(14 + payload.len());
    buf.extend_from_slice(b"i3-ipc");
    buf.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    buf.extend_from_slice(&typ.to_le_bytes());
    buf.extend_from_slice(payload.as_bytes());
    s.write_all(&buf).unwrap_or_else(|e| die(&format!("ipc write: {e}")));
}

fn recv(s: &mut UnixStream) -> (u32, Vec<u8>) {
    let mut head = [0u8; 14];
    s.read_exact(&mut head).unwrap_or_else(|e| die(&format!("ipc read: {e}")));
    let len = u32::from_le_bytes(head[6..10].try_into().unwrap()) as usize;
    let typ = u32::from_le_bytes(head[10..14].try_into().unwrap());
    let mut body = vec![0u8; len];
    s.read_exact(&mut body).unwrap_or_else(|e| die(&format!("ipc read: {e}")));
    (typ, body)
}

fn run(s: &mut UnixStream, command: &str) {
    send(s, RUN_COMMAND, command);
    recv(s);
}

// ── a small JSON reader: just enough for sway's tree and events ──

enum Json {
    Null,
    Bool,
    Num(i64), // sway's ids are integers; fractions are kept as their whole part
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    fn str(&self, key: &str) -> &str {
        match self.get(key) {
            Some(Json::Str(s)) => s,
            _ => "",
        }
    }
    fn int(&self, key: &str) -> i64 {
        match self.get(key) {
            Some(Json::Num(n)) => *n,
            _ => 0,
        }
    }
    fn list(&self, key: &str) -> &[Json] {
        match self.get(key) {
            Some(Json::Arr(a)) => a,
            _ => &[],
        }
    }
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.i < self.b.len() && self.b[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }
    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }
    fn eat(&mut self, c: u8) -> Option<()> {
        self.ws();
        if self.peek()? == c {
            self.i += 1;
            Some(())
        } else {
            None
        }
    }
    fn value(&mut self) -> Option<Json> {
        self.ws();
        match self.peek()? {
            b'{' => {
                self.i += 1;
                let mut fields = Vec::new();
                self.ws();
                if self.peek()? == b'}' {
                    self.i += 1;
                    return Some(Json::Obj(fields));
                }
                loop {
                    self.ws();
                    let k = self.string()?;
                    self.eat(b':')?;
                    fields.push((k, self.value()?));
                    self.ws();
                    match self.peek()? {
                        b',' => self.i += 1,
                        b'}' => {
                            self.i += 1;
                            return Some(Json::Obj(fields));
                        }
                        _ => return None,
                    }
                }
            }
            b'[' => {
                self.i += 1;
                let mut items = Vec::new();
                self.ws();
                if self.peek()? == b']' {
                    self.i += 1;
                    return Some(Json::Arr(items));
                }
                loop {
                    items.push(self.value()?);
                    self.ws();
                    match self.peek()? {
                        b',' => self.i += 1,
                        b']' => {
                            self.i += 1;
                            return Some(Json::Arr(items));
                        }
                        _ => return None,
                    }
                }
            }
            b'"' => self.string().map(Json::Str),
            b't' => self.word("true").map(|_| Json::Bool),
            b'f' => self.word("false").map(|_| Json::Bool),
            b'n' => self.word("null").map(|_| Json::Null),
            _ => self.number(),
        }
    }
    fn word(&mut self, w: &str) -> Option<()> {
        if self.b[self.i..].starts_with(w.as_bytes()) {
            self.i += w.len();
            Some(())
        } else {
            None
        }
    }
    fn number(&mut self) -> Option<Json> {
        let start = self.i;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.' | b'e' | b'E') {
                self.i += 1;
            } else {
                break;
            }
        }
        let text = std::str::from_utf8(&self.b[start..self.i]).ok()?;
        let whole = text.split(['.', 'e', 'E']).next()?;
        Some(Json::Num(whole.parse().unwrap_or(0)))
    }
    fn string(&mut self) -> Option<String> {
        if self.peek()? != b'"' {
            return None;
        }
        self.i += 1;
        let mut out = Vec::new();
        loop {
            let c = self.peek()?;
            self.i += 1;
            match c {
                b'"' => return String::from_utf8(out).ok(),
                b'\\' => {
                    let e = self.peek()?;
                    self.i += 1;
                    match e {
                        b'n' => out.push(b'\n'),
                        b't' => out.push(b'\t'),
                        b'r' => out.push(b'\r'),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'u' => {
                            let hex = std::str::from_utf8(self.b.get(self.i..self.i + 4)?).ok()?;
                            self.i += 4;
                            let mut cp = u32::from_str_radix(hex, 16).ok()?;
                            // surrogate pair (emoji and the like in window titles)
                            if (0xD800..0xDC00).contains(&cp) && self.b[self.i..].starts_with(b"\\u") {
                                let lo = std::str::from_utf8(self.b.get(self.i + 2..self.i + 6)?).ok()?;
                                let lo = u32::from_str_radix(lo, 16).ok()?;
                                self.i += 6;
                                cp = 0x10000 + ((cp - 0xD800) << 10) + (lo.wrapping_sub(0xDC00) & 0x3FF);
                            }
                            let ch = char::from_u32(cp).unwrap_or('\u{FFFD}');
                            let mut tmp = [0u8; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut tmp).as_bytes());
                        }
                        other => out.push(other), // \" \\ \/
                    }
                }
                _ => out.push(c),
            }
        }
    }
}

fn parse(b: &[u8]) -> Option<Json> {
    Parser { b, i: 0 }.value()
}

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

fn walk(n: &Json, s: &mut Snapshot) {
    let id = n.int("id");
    let layout = n.str("layout");
    // a workspace with a tabbed layout isn't a group: dragging one window
    // off it shouldn't pull every other window along
    s.group.insert(id, n.str("type") == "con" && (layout == "tabbed" || layout == "stacked"));
    for c in n.list("nodes") {
        // tiled children only: floating windows are skipped
        let cid = c.int("id");
        s.kids.entry(id).or_default().push(cid);
        s.parent.insert(cid, id);
        s.window.insert(cid, !c.str("shell").is_empty());
        walk(c, s);
    }
}

fn take(sock: &mut UnixStream) -> Snapshot {
    send(sock, GET_TREE, "");
    let (_, body) = recv(sock);
    let root = parse(&body).unwrap_or_else(|| die("couldn't read sway's tree"));
    drop(body);
    let mut s = Snapshot::default();
    for output in root.list("nodes") {
        for ws in output.list("nodes") {
            if ws.str("name") != "__i3_scratch" {
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

fn apply(sock: &mut UnixStream, f: &Follow) {
    let now = take(sock);
    let Some(&p) = now.parent.get(&f.win) else { return };
    let mut cmds: Vec<String> = Vec::new();
    if !now.is_group(p) {
        // wrap the dropped window in a new tabbed container
        cmds.push(format!("[con_id={}] splitv", f.win));
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
    let mark = |id: i64| format!("[con_id={id}] mark --add __grpfollow");
    let move_after = |cmds: &mut Vec<String>, ids: &[i64]| {
        for id in ids.iter().rev() {
            // each lands right after the mark
            cmds.push(format!("[con_id={id}] move container to mark __grpfollow"));
        }
    };
    cmds.push(mark(f.win));
    move_after(&mut cmds, &pre);
    if let Some(&last) = pre.last() {
        cmds.push(mark(last));
        cmds.push(format!("[con_id={}] move container to mark __grpfollow", f.win));
        cmds.push(mark(f.win));
    }
    move_after(&mut cmds, &post);
    cmds.push("unmark __grpfollow".into());
    cmds.push(format!("[con_id={}] focus", f.win));
    run(sock, &cmds.join("; "));
    eprintln!("group {} followed window {}", f.group, f.win);

    // after a swap the other window sits alone in the old group: untab it
    let after = take(sock);
    if let Some(kids) = after.kids.get(&f.group) {
        if kids.len() == 1 && after.is_group(f.group) && !f.all.contains(&kids[0]) {
            run(sock, &format!("[con_id={}] layout splith; [con_id={}] focus", kids[0], f.win));
        }
    }
}

fn main() {
    let mut sock = connect();
    let mut events = connect();
    send(&mut events, SUBSCRIBE, r#"["binding"]"#);
    recv(&mut events);

    let (tx, rx) = mpsc::channel::<String>();
    thread::spawn(move || loop {
        let (typ, body) = recv(&mut events);
        if typ != BINDING_EVENT {
            continue;
        }
        if let Some(ev) = parse(&body) {
            if let Some(b) = ev.get("binding") {
                if tx.send(b.str("command").trim().to_string()).is_err() {
                    return;
                }
            }
        }
    });

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
                before = take(&mut sock);
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
            Err(RecvTimeoutError::Disconnected) => die("lost the event connection"),
            Err(RecvTimeoutError::Timeout) => {
                let t = Instant::now();
                next_tick = t + POLL;
                if let Some((f, at)) = &pending {
                    if t.duration_since(*at) >= SETTLE {
                        apply(&mut sock, f);
                        pending = None;
                        before = take(&mut sock);
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
                let now = take(&mut sock);
                match detect(&before, &now) {
                    Some(f) => pending = Some((f, t)),
                    None => before = now,
                }
                trim();
            }
        }
    }
}
