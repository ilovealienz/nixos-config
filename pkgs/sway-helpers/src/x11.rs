//! Just enough of the X11 protocol to hear attention requests from
//! XWayland windows.
//!
//! Every packet from the server is 32 bytes, except replies and generic
//! events, which say how much more follows.

use std::env;
use std::io::{self, Read, Write};
use std::net::Shutdown;
use std::os::linux::net::SocketAddrExt;
use std::os::unix::net::{SocketAddr, UnixStream};

const REPLY: u8 = 1;
const ERROR: u8 = 0;
const CLIENT_MESSAGE: u8 = 33;
const GENERIC_EVENT: u8 = 35;

const CHANGE_WINDOW_ATTRIBUTES: u8 = 2;
const INTERN_ATOM: u8 = 16;
const QUERY_EXTENSION: u8 = 98;
const XFIXES_QUERY_VERSION: u8 = 0;
const XFIXES_SET_CLIENT_DISCONNECT_MODE: u8 = 33;
const DISCONNECT_TERMINATE: u32 = 1;

const CW_EVENT_MASK: u32 = 1 << 11;
const SUBSTRUCTURE_NOTIFY: u32 = 1 << 19;

pub struct X {
    s: UnixStream,
    seq: u16, // sequence number of the last request sent
    pub root: u32,
}

/// What happened to the connection's lifetime setting.
pub enum Detach {
    /// XWayland can exit with this connection still open.
    Done,
    /// The server is too old for it; this connection keeps XWayland running.
    Unsupported(String),
}

fn pad(n: usize) -> usize {
    (4 - n % 4) % 4
}

fn err(msg: impl Into<String>) -> io::Error {
    io::Error::other(msg.into())
}

/// The socket for $DISPLAY (":1" or ":1.0"): the abstract socket first,
/// as libxcb does, then the file in /tmp/.X11-unix.
fn socket() -> io::Result<UnixStream> {
    let display = env::var("DISPLAY").map_err(|_| err("DISPLAY is not set"))?;
    let num = display
        .strip_prefix(':')
        .and_then(|d| d.split('.').next())
        .filter(|n| !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()))
        .ok_or_else(|| err(format!("can't use DISPLAY={display}")))?;
    let path = format!("/tmp/.X11-unix/X{num}");
    if let Ok(addr) = SocketAddr::from_abstract_name(path.as_bytes()) {
        if let Ok(s) = UnixStream::connect_addr(&addr) {
            return Ok(s);
        }
    }
    UnixStream::connect(&path)
}

impl X {
    pub fn connect() -> io::Result<X> {
        let mut s = socket()?;
        // little-endian, protocol 11.0, no authorization (wlroots starts
        // XWayland without any)
        s.write_all(&[b'l', 0, 11, 0, 0, 0, 0, 0, 0, 0, 0, 0])?;
        let mut head = [0u8; 8];
        s.read_exact(&mut head)?;
        let mut body = vec![0u8; u16::from_le_bytes([head[6], head[7]]) as usize * 4];
        s.read_exact(&mut body)?;
        if head[0] != 1 {
            let reason_len = (head[1] as usize).min(body.len());
            let reason = String::from_utf8_lossy(&body[..reason_len]);
            return Err(err(format!("X server refused the connection: {reason}")));
        }
        // body starts at byte 8 of the setup reply
        let u16_at = |o: usize| u16::from_le_bytes([body[o - 8], body[o - 7]]) as usize;
        if body.len() < 32 {
            return Err(err("short X setup reply"));
        }
        let vendor_len = u16_at(24);
        let formats = body[29 - 8] as usize;
        let screen = 40 + vendor_len + pad(vendor_len) + 8 * formats - 8;
        let root = body
            .get(screen..screen + 4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
            .ok_or_else(|| err("short X setup reply"))?;
        Ok(X { s, seq: 0, root })
    }

    fn request(&mut self, data: &[u8]) -> io::Result<u16> {
        self.s.write_all(data)?;
        self.seq = self.seq.wrapping_add(1);
        Ok(self.seq)
    }

    /// One packet from the server.
    fn packet(&mut self) -> io::Result<Vec<u8>> {
        let mut p = vec![0u8; 32];
        self.s.read_exact(&mut p)?;
        if p[0] == REPLY || p[0] & 0x7f == GENERIC_EVENT {
            let extra = u32::from_le_bytes(p[4..8].try_into().unwrap()) as usize * 4;
            if extra > 16 << 20 {
                return Err(err("X packet too big"));
            }
            p.resize(32 + extra, 0);
            self.s.read_exact(&mut p[32..])?;
        }
        Ok(p)
    }

    /// Waits for the reply to request `seq`; events before it are dropped
    /// (nothing is listened for until setup is done).
    fn reply(&mut self, seq: u16) -> io::Result<Vec<u8>> {
        loop {
            let p = self.packet()?;
            let pseq = u16::from_le_bytes([p[2], p[3]]);
            match p[0] {
                REPLY if pseq == seq => return Ok(p),
                ERROR if pseq == seq => {
                    return Err(err(format!("X error {} for request {}", p[1], p[10])));
                }
                ERROR => eprintln!("attention: X error {} for request {}", p[1], p[10]),
                _ => {}
            }
        }
    }

    pub fn intern_atom(&mut self, name: &str) -> io::Result<u32> {
        let n = name.len();
        let mut r = vec![INTERN_ATOM, 0];
        r.extend_from_slice(&((2 + (n + pad(n)) / 4) as u16).to_le_bytes());
        r.extend_from_slice(&(n as u16).to_le_bytes());
        r.extend_from_slice(&[0, 0]);
        r.extend_from_slice(name.as_bytes());
        r.resize(r.len() + pad(n), 0);
        let seq = self.request(&r)?;
        let p = self.reply(seq)?;
        Ok(u32::from_le_bytes(p[8..12].try_into().unwrap()))
    }

    /// Lets XWayland exit when the last real app closes, even though this
    /// connection is still open (XFixes 6: SetClientDisconnectMode).
    pub fn detach(&mut self) -> io::Result<Detach> {
        let name = b"XFIXES";
        let mut r = vec![QUERY_EXTENSION, 0];
        r.extend_from_slice(&((2 + (name.len() + pad(name.len())) / 4) as u16).to_le_bytes());
        r.extend_from_slice(&(name.len() as u16).to_le_bytes());
        r.extend_from_slice(&[0, 0]);
        r.extend_from_slice(name);
        r.resize(r.len() + pad(name.len()), 0);
        let seq = self.request(&r)?;
        let p = self.reply(seq)?;
        if p[8] == 0 {
            return Ok(Detach::Unsupported("no XFIXES extension".into()));
        }
        let opcode = p[9];

        // the server only allows version 6 requests after this
        let mut r = vec![opcode, XFIXES_QUERY_VERSION];
        r.extend_from_slice(&3u16.to_le_bytes());
        r.extend_from_slice(&6u32.to_le_bytes());
        r.extend_from_slice(&0u32.to_le_bytes());
        let seq = self.request(&r)?;
        let p = self.reply(seq)?;
        let major = u32::from_le_bytes(p[8..12].try_into().unwrap());
        if major < 6 {
            return Ok(Detach::Unsupported(format!("XFIXES {major} is older than 6")));
        }

        let mut r = vec![opcode, XFIXES_SET_CLIENT_DISCONNECT_MODE];
        r.extend_from_slice(&2u16.to_le_bytes());
        r.extend_from_slice(&DISCONNECT_TERMINATE.to_le_bytes());
        let seq = self.request(&r)?;
        self.check(seq)?;
        Ok(Detach::Done)
    }

    /// Makes sure request `seq` (which has no reply) didn't fail, using a
    /// request that does have one.
    fn check(&mut self, seq: u16) -> io::Result<()> {
        let mut r = vec![INTERN_ATOM, 1];
        r.extend_from_slice(&3u16.to_le_bytes());
        r.extend_from_slice(&1u16.to_le_bytes());
        r.extend_from_slice(&[0, 0, b'_', 0, 0, 0]);
        let probe = self.request(&r)?;
        loop {
            let p = self.packet()?;
            let pseq = u16::from_le_bytes([p[2], p[3]]);
            match p[0] {
                ERROR if pseq == seq => {
                    return Err(err(format!("X error {} for request {}", p[1], p[10])));
                }
                REPLY | ERROR if pseq == probe => return Ok(()),
                _ => {}
            }
        }
    }

    /// Hears client messages sent to the root window (where EWMH requests
    /// like _NET_WM_STATE go).
    pub fn listen_root(&mut self) -> io::Result<()> {
        let mut r = vec![CHANGE_WINDOW_ATTRIBUTES, 0];
        r.extend_from_slice(&4u16.to_le_bytes());
        r.extend_from_slice(&self.root.to_le_bytes());
        r.extend_from_slice(&CW_EVENT_MASK.to_le_bytes());
        r.extend_from_slice(&SUBSTRUCTURE_NOTIFY.to_le_bytes());
        let seq = self.request(&r)?;
        self.check(seq)
    }

    /// The next client message: (window, message type, format, data).
    /// Err when the connection ends (XWayland exited).
    pub fn client_message(&mut self) -> io::Result<(u32, u32, u8, [u32; 5])> {
        loop {
            let p = self.packet()?;
            // the top bit means "sent by another client", which is how
            // these requests always arrive
            if p[0] & 0x7f != CLIENT_MESSAGE {
                if p[0] == ERROR {
                    eprintln!("attention: X error {} for request {}", p[1], p[10]);
                }
                continue;
            }
            let u32_at = |o: usize| u32::from_le_bytes(p[o..o + 4].try_into().unwrap());
            let data = [u32_at(12), u32_at(16), u32_at(20), u32_at(24), u32_at(28)];
            return Ok((u32_at(4), u32_at(8), p[1], data));
        }
    }

    /// A handle that closes the connection (ending a blocked read).
    pub fn closer(&self) -> io::Result<XCloser> {
        Ok(XCloser(self.s.try_clone()?))
    }
}

pub struct XCloser(UnixStream);

impl Drop for XCloser {
    fn drop(&mut self) {
        let _ = self.0.shutdown(Shutdown::Both);
    }
}
