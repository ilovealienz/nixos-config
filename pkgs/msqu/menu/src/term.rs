//! The terminal: raw mode, size, keys, colours.

use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

/// the window changed size (SIGWINCH) since `resized` last said so
static RESIZED: AtomicBool = AtomicBool::new(false);
/// a pipe the resize handler writes a byte to, which the wait for keys
/// also watches (read end, write end)
static WAKE_R: AtomicI32 = AtomicI32::new(-1);
static WAKE_W: AtomicI32 = AtomicI32::new(-1);

extern "C" fn on_winch(_: libc::c_int) {
    RESIZED.store(true, Ordering::SeqCst);
    let fd = WAKE_W.load(Ordering::SeqCst);
    if fd >= 0 {
        unsafe { libc::write(fd, b"w".as_ptr().cast(), 1) };
    }
}

/// True once after each resize (sway resizes the window right after start).
pub fn resized() -> bool {
    RESIZED.swap(false, Ordering::SeqCst)
}

pub struct Raw {
    old: libc::termios,
}

impl Raw {
    pub fn enter() -> std::io::Result<Raw> {
        let fd = std::io::stdin().as_raw_fd();
        let mut t: libc::termios = unsafe { std::mem::zeroed() };
        if unsafe { libc::tcgetattr(fd, &mut t) } != 0 {
            return Err(std::io::Error::last_os_error());
        }
        let old = t;
        unsafe { libc::cfmakeraw(&mut t) };
        // keep \n -> \r\n off; we write \r\n ourselves
        if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &t) } != 0 {
            return Err(std::io::Error::last_os_error());
        }
        // a resize wakes the wait for keys through a pipe, so one that
        // comes while drawing isn't missed until the next second either
        unsafe {
            let mut fds = [-1; 2];
            if libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) == 0 {
                WAKE_R.store(fds[0], Ordering::SeqCst);
                WAKE_W.store(fds[1], Ordering::SeqCst);
            }
            let mut sa: libc::sigaction = std::mem::zeroed();
            sa.sa_sigaction = on_winch as extern "C" fn(libc::c_int) as libc::sighandler_t;
            sa.sa_flags = libc::SA_RESTART;
            libc::sigemptyset(&mut sa.sa_mask);
            libc::sigaction(libc::SIGWINCH, &sa, std::ptr::null_mut());
        }
        // alternate screen, hide cursor
        out("\x1b[?1049h\x1b[?25l\x1b[2J");
        Ok(Raw { old })
    }
}

impl Drop for Raw {
    fn drop(&mut self) {
        out("\x1b[0m\x1b[?25h\x1b[?1049l");
        unsafe { libc::tcsetattr(std::io::stdin().as_raw_fd(), libc::TCSANOW, &self.old) };
    }
}

pub fn out(s: &str) {
    let mut o = std::io::stdout().lock();
    let _ = o.write_all(s.as_bytes());
    let _ = o.flush();
}

/// (rows, cols)
pub fn size() -> (usize, usize) {
    let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
    if unsafe { libc::ioctl(std::io::stdout().as_raw_fd(), libc::TIOCGWINSZ, &mut ws) } != 0 || ws.ws_col == 0 {
        return (24, 80);
    }
    (ws.ws_row as usize, ws.ws_col as usize)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Key {
    Char(char),
    Enter,
    Esc,
    Tab,
    BackTab,
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
    Backspace,
    Delete,
    CtrlU,
    CtrlW,
    CtrlX,
    CtrlP,
    CtrlS,
    Quit,
}

/// waits up to `ms` milliseconds for input; false at once on a resize
fn ready(ms: i32) -> bool {
    let wake = WAKE_R.load(Ordering::SeqCst);
    let mut p = [
        libc::pollfd { fd: std::io::stdin().as_raw_fd(), events: libc::POLLIN, revents: 0 },
        libc::pollfd { fd: wake, events: libc::POLLIN, revents: 0 },
    ];
    let n = if wake >= 0 { 2 } else { 1 };
    if unsafe { libc::poll(p.as_mut_ptr(), n, ms) } <= 0 {
        return false;
    }
    if p[1].revents != 0 {
        // resized: empty the pipe and draw
        let mut buf = [0u8; 64];
        while unsafe { libc::read(wake, buf.as_mut_ptr().cast(), buf.len()) } > 0 {}
        return false;
    }
    p[0].revents != 0
}

/// keys typed within `ms` milliseconds (none if nothing was typed)
pub fn keys(ms: i32) -> Vec<Key> {
    if !ready(ms) {
        return Vec::new();
    }
    let mut buf = [0u8; 256];
    let n = match std::io::stdin().read(&mut buf) {
        Ok(0) | Err(_) => return vec![Key::Quit],
        Ok(n) => n,
    };
    let mut bytes = buf[..n].to_vec();
    // a lone ESC may be the start of a sequence still on its way
    if bytes == [0x1b] && ready(30) {
        if let Ok(m) = std::io::stdin().read(&mut buf) {
            bytes.extend_from_slice(&buf[..m]);
        }
    }
    parse(&bytes)
}

pub fn parse(b: &[u8]) -> Vec<Key> {
    let mut keys = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let rest = &b[i..];
        let seq: &[(&[u8], Key)] = &[
            (b"\x1b[A", Key::Up),
            (b"\x1bOA", Key::Up),
            (b"\x1b[B", Key::Down),
            (b"\x1bOB", Key::Down),
            (b"\x1b[Z", Key::BackTab),
            (b"\x1b[3~", Key::Delete),
            (b"\x1b[5~", Key::PageUp),
            (b"\x1b[6~", Key::PageDown),
            (b"\x1b[H", Key::Home),
            (b"\x1b[1~", Key::Home),
            (b"\x1b[F", Key::End),
            (b"\x1b[4~", Key::End),
        ];
        if let Some((s, k)) = seq.iter().find(|(s, _)| rest.starts_with(s)) {
            keys.push(k.clone());
            i += s.len();
            continue;
        }
        match rest[0] {
            0x1b if rest.len() > 1 && (rest[1] == b'[' || rest[1] == b'O') => {
                // an unknown sequence: skip to its final byte
                let mut j = 2;
                while j < rest.len() && !(0x40..=0x7e).contains(&rest[j]) {
                    j += 1;
                }
                i += j + 1;
            }
            0x1b => {
                keys.push(Key::Esc);
                i += 1;
            }
            b'\r' | b'\n' => {
                keys.push(Key::Enter);
                i += 1;
            }
            b'\t' => {
                keys.push(Key::Tab);
                i += 1;
            }
            0x7f | 0x08 => {
                keys.push(Key::Backspace);
                i += 1;
            }
            0x15 => {
                keys.push(Key::CtrlU);
                i += 1;
            }
            0x17 => {
                keys.push(Key::CtrlW);
                i += 1;
            }
            0x18 => {
                keys.push(Key::CtrlX);
                i += 1;
            }
            0x10 => {
                keys.push(Key::CtrlP);
                i += 1;
            }
            0x13 => {
                keys.push(Key::CtrlS);
                i += 1;
            }
            0x03 | 0x04 => {
                keys.push(Key::Quit);
                i += 1;
            }
            c if c < 0x20 => i += 1,
            _ => {
                // one UTF-8 character
                let len = match rest[0] {
                    0xf0..=0xf7 => 4,
                    0xe0..=0xef => 3,
                    0xc0..=0xdf => 2,
                    _ => 1,
                };
                if let Some(c) = rest.get(..len).and_then(|s| std::str::from_utf8(s).ok()).and_then(|s| s.chars().next()) {
                    keys.push(Key::Char(c));
                }
                i += len;
            }
        }
    }
    keys
}

/// colours from the theme
#[derive(Clone)]
pub struct Pal {
    pub bg: String,
    pub fg: String,
    pub bright: String,
    pub muted: String,
    pub accent: String,
    pub surface_bg: String,
    pub line: String,
    pub red: String,
    pub green: String,
    pub yellow: String,
    pub blue: String,
    pub orange: String,
    pub magenta: String,
}

pub const RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";

fn rgb(hex: &str) -> (u8, u8, u8) {
    let h = hex.trim_start_matches('#');
    let p = |i: usize| h.get(i..i + 2).and_then(|s| u8::from_str_radix(s, 16).ok()).unwrap_or(200);
    (p(0), p(2), p(4))
}

pub fn fgc(hex: &str) -> String {
    let (r, g, b) = rgb(hex);
    format!("\x1b[38;2;{r};{g};{b}m")
}

pub fn bgc(hex: &str) -> String {
    let (r, g, b) = rgb(hex);
    format!("\x1b[48;2;{r};{g};{b}m")
}

impl Pal {
    pub fn new(c: &std::collections::HashMap<String, String>) -> Pal {
        let get = |k: &str, d: &str| c.get(k).cloned().unwrap_or_else(|| d.to_string());
        Pal {
            bg: bgc(&get("bg", "24221c")),
            fg: fgc(&get("fg", "d4b07b")),
            bright: fgc(&get("fgBright", "ede0c8")),
            muted: fgc(&get("muted", "87765d")),
            accent: fgc(&get("accent", "e5a440")),
            surface_bg: bgc(&get("surface", "473f31")),
            line: fgc(&get("surface", "473f31")),
            red: fgc(&get("red", "e56b55")),
            green: fgc(&get("green", "99b05f")),
            yellow: fgc(&get("yellow", "bfab36")),
            blue: fgc(&get("blue", "949fb4")),
            orange: fgc(&get("orange", "e18245")),
            magenta: fgc(&get("magenta", "d261a5")),
        }
    }
}

/// display width (one column per character; good enough for this text)
pub fn width(s: &str) -> usize {
    s.chars().count()
}

pub fn cut(s: &str, n: usize) -> String {
    if width(s) <= n {
        return s.to_string();
    }
    if n == 0 {
        return String::new();
    }
    let mut r: String = s.chars().take(n - 1).collect();
    r.push('…');
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_parse() {
        assert_eq!(parse(b"ab\r"), [Key::Char('a'), Key::Char('b'), Key::Enter]);
        assert_eq!(parse(b"\x1b[A\x1b[B\x1b[Z\t"), [Key::Up, Key::Down, Key::BackTab, Key::Tab]);
        assert_eq!(parse(b"\x1b"), [Key::Esc]);
        assert_eq!(parse("é→".as_bytes()), [Key::Char('é'), Key::Char('→')]);
        assert_eq!(parse(b"\x1b[1;5C x"), [Key::Char(' '), Key::Char('x')]);
        assert_eq!(parse(b"\x7f\x15\x17\x18\x03"), [Key::Backspace, Key::CtrlU, Key::CtrlW, Key::CtrlX, Key::Quit]);
        assert_eq!(parse(b"\x10\x13"), [Key::CtrlP, Key::CtrlS]);
    }

    #[test]
    fn cutting() {
        assert_eq!(cut("hello", 10), "hello");
        assert_eq!(cut("hello world", 6), "hello…");
    }
}
