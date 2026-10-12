//! sway's IPC socket: "i3-ipc" + u32 length + u32 type + JSON payload.
//!
//! Losing sway's socket means sway exited or restarted (with a new socket
//! path), so any read or write error ends the whole program and systemd
//! starts it again.

use crate::json::{self, Node};
use crate::die;
use std::env;
use std::io::{Read, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub const RUN_COMMAND: u32 = 0;
pub const SUBSCRIBE: u32 = 2;
pub const GET_TREE: u32 = 4;
pub const WINDOW_EVENT: u32 = 0x8000_0003;
pub const BINDING_EVENT: u32 = 0x8000_0005;

/// Bigger than any tree sway will send; a length past this means the
/// stream is out of step.
const MAX_MESSAGE: usize = 64 << 20;

pub struct Sway {
    s: UnixStream,
    name: &'static str, // which feature, for messages
}

impl Sway {
    pub fn connect(name: &'static str) -> Sway {
        let path = env::var("SWAYSOCK").unwrap_or_else(|_| die(name, "SWAYSOCK is not set"));
        let s = UnixStream::connect(&path)
            .unwrap_or_else(|e| die(name, &format!("connect to sway ({path}): {e}")));
        Sway { s, name }
    }

    fn send(&mut self, typ: u32, payload: &str) {
        let mut buf = Vec::with_capacity(14 + payload.len());
        buf.extend_from_slice(b"i3-ipc");
        buf.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        buf.extend_from_slice(&typ.to_le_bytes());
        buf.extend_from_slice(payload.as_bytes());
        if let Err(e) = self.s.write_all(&buf) {
            die(self.name, &format!("sway ipc write: {e}"));
        }
    }

    /// The next message, or None once the connection is closed.
    fn try_recv(&mut self) -> Option<(u32, Vec<u8>)> {
        let mut head = [0u8; 14];
        self.s.read_exact(&mut head).ok()?;
        if &head[..6] != b"i3-ipc" {
            die(self.name, "sway ipc: bad message header");
        }
        let len = u32::from_le_bytes(head[6..10].try_into().unwrap()) as usize;
        let typ = u32::from_le_bytes(head[10..14].try_into().unwrap());
        if len > MAX_MESSAGE {
            die(self.name, &format!("sway ipc: message too big ({len} bytes)"));
        }
        let mut body = vec![0u8; len];
        self.s.read_exact(&mut body).ok()?;
        Some((typ, body))
    }

    fn recv(&mut self) -> (u32, Vec<u8>) {
        self.try_recv().unwrap_or_else(|| die(self.name, "lost the connection to sway"))
    }

    /// Runs sway commands (separated by ";" or ","), logging any that fail.
    pub fn run(&mut self, command: &str) {
        self.send(RUN_COMMAND, command);
        let (_, body) = self.recv();
        // a window that closed meanwhile is nothing to worry about
        for e in json::reply_errors(&body).iter().filter(|e| !e.starts_with("No matching node")) {
            eprintln!("{}: sway: {e} (running: {command})", self.name);
        }
    }

    /// Runs commands whose failure is expected and harmless (e.g. criteria
    /// that match nothing).
    pub fn run_quiet(&mut self, command: &str) {
        self.send(RUN_COMMAND, command);
        self.recv();
    }

    pub fn tree(&mut self) -> Node {
        self.send(GET_TREE, "");
        let (_, body) = self.recv();
        Node::from(&body).unwrap_or_else(|| die(self.name, "couldn't read sway's tree"))
    }

    /// Turns this connection into an event stream.
    pub fn subscribe(mut self, events: &str) -> Events {
        self.send(SUBSCRIBE, events);
        let (_, body) = self.recv();
        if !json::reply_errors(&body).is_empty() {
            die(self.name, &format!("sway refused the subscription {events}"));
        }
        let s = self.s.try_clone().unwrap_or_else(|e| die(self.name, &format!("socket: {e}")));
        Events {
            sway: self,
            closer: Closer { s, closing: Arc::new(AtomicBool::new(false)) },
        }
    }
}

/// Subscribed events. Read them promptly: sway drops a client that lets
/// 4MB of unread events pile up.
pub struct Events {
    sway: Sway,
    closer: Closer,
}

impl Events {
    /// The next event, or None if `closer()` closed the stream.
    pub fn next(&mut self) -> Option<(u32, Vec<u8>)> {
        match self.sway.try_recv() {
            Some(ev) => Some(ev),
            None if self.closer.closing.load(Ordering::SeqCst) => None,
            None => die(self.sway.name, "lost the connection to sway"),
        }
    }

    /// Something that closes this stream when dropped, so a reader thread
    /// blocked in `next()` ends when the feature that started it stops
    /// (or panics).
    pub fn closer(&self) -> Closer {
        Closer {
            s: self.closer.s.try_clone().unwrap_or_else(|e| die(self.sway.name, &format!("socket: {e}"))),
            closing: self.closer.closing.clone(),
        }
    }
}

pub struct Closer {
    s: UnixStream,
    closing: Arc<AtomicBool>,
}

impl Drop for Closer {
    fn drop(&mut self) {
        self.closing.store(true, Ordering::SeqCst);
        let _ = self.s.shutdown(Shutdown::Both);
    }
}
