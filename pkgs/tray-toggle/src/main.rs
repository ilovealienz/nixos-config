//! tray-toggle: the toggle in front of waybar's tray drawer.
//!
//!   status [--compact]  the toggle for this bar (WAYBAR_OUTPUT_NAME), as
//!                       waybar JSON: [3] (tray apps, names in the tooltip)
//!                       while closed, [<] while open, nothing with no apps
//!   update              the tray's on-update hook: reads the tray apps and
//!                       refreshes the bars only if they changed
//!   flip                the toggle was clicked (on the focused screen's bar)
//!
//! Waybar doesn't say if a drawer is open, so it's tracked per screen in
//! $XDG_RUNTIME_DIR/waybar-tray. Nothing stays running: each call exits.

mod json;

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::fs;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::ptr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// waybar's signal for the toggle (signal = 12 in waybar.nix)
const SIGNAL: c_int = 12;
/// GTK's double-click time
const DOUBLE_CLICK: Duration = Duration::from_millis(400);
/// a tray app that doesn't answer in this long is left out
const DBUS_TIMEOUT_USEC: u64 = 300_000;

#[link(name = "systemd")]
extern "C" {
    fn sd_bus_open_user(bus: *mut *mut c_void) -> c_int;
    fn sd_bus_set_method_call_timeout(bus: *mut c_void, usec: u64) -> c_int;
    fn sd_bus_flush_close_unref(bus: *mut c_void) -> *mut c_void;
    fn sd_bus_get_property_strv(
        bus: *mut c_void,
        dest: *const c_char,
        path: *const c_char,
        iface: *const c_char,
        member: *const c_char,
        err: *mut c_void,
        ret: *mut *mut *mut c_char,
    ) -> c_int;
    fn sd_bus_get_property_string(
        bus: *mut c_void,
        dest: *const c_char,
        path: *const c_char,
        iface: *const c_char,
        member: *const c_char,
        err: *mut c_void,
        ret: *mut *mut c_char,
    ) -> c_int;
}

extern "C" {
    // glibc
    fn free(p: *mut c_void);
    fn kill(pid: c_int, sig: c_int) -> c_int;
    fn __libc_current_sigrtmin() -> c_int;
}

fn dir() -> PathBuf {
    let base = std::env::var_os("XDG_RUNTIME_DIR").filter(|v| !v.is_empty()).unwrap_or("/tmp".into());
    PathBuf::from(base).join("waybar-tray")
}

/// Writes through a temporary file, so a reader never sees half of it.
fn write_atomic(path: &Path, data: &[u8]) {
    let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
    if fs::write(&tmp, data).is_ok() && fs::rename(&tmp, path).is_err() {
        let _ = fs::remove_file(&tmp);
    }
}

// ── the tray apps, from the StatusNotifierWatcher (waybar runs it) ──

struct Bus(*mut c_void);

impl Bus {
    fn open() -> Option<Bus> {
        let mut b = ptr::null_mut();
        if unsafe { sd_bus_open_user(&mut b) } < 0 {
            return None;
        }
        unsafe { sd_bus_set_method_call_timeout(b, DBUS_TIMEOUT_USEC) };
        Some(Bus(b))
    }

    fn strv(&self, dest: &str, path: &str, iface: &str, member: &str) -> Option<Vec<String>> {
        let (d, p, i, m) = (cs(dest)?, cs(path)?, cs(iface)?, cs(member)?);
        let mut ret: *mut *mut c_char = ptr::null_mut();
        let r = unsafe {
            sd_bus_get_property_strv(self.0, d.as_ptr(), p.as_ptr(), i.as_ptr(), m.as_ptr(), ptr::null_mut(), &mut ret)
        };
        if r < 0 || ret.is_null() {
            return None;
        }
        let mut out = Vec::new();
        unsafe {
            let mut at = ret;
            while !(*at).is_null() {
                out.push(CStr::from_ptr(*at).to_string_lossy().into_owned());
                free(*at as *mut c_void);
                at = at.add(1);
            }
            free(ret as *mut c_void);
        }
        Some(out)
    }

    fn string(&self, dest: &str, path: &str, iface: &str, member: &str) -> Option<String> {
        let (d, p, i, m) = (cs(dest)?, cs(path)?, cs(iface)?, cs(member)?);
        let mut ret: *mut c_char = ptr::null_mut();
        let r = unsafe {
            sd_bus_get_property_string(self.0, d.as_ptr(), p.as_ptr(), i.as_ptr(), m.as_ptr(), ptr::null_mut(), &mut ret)
        };
        if r < 0 || ret.is_null() {
            return None;
        }
        let s = unsafe { CStr::from_ptr(ret).to_string_lossy().into_owned() };
        unsafe { free(ret as *mut c_void) };
        Some(s)
    }
}

impl Drop for Bus {
    fn drop(&mut self) {
        unsafe { sd_bus_flush_close_unref(self.0) };
    }
}

fn cs(s: &str) -> Option<CString> {
    CString::new(s).ok()
}

/// One name per tray app (its title, else its id), or None if there's no
/// tray to ask.
fn tray_apps() -> Option<Vec<String>> {
    let bus = Bus::open()?;
    let items = bus.strv(
        "org.kde.StatusNotifierWatcher",
        "/StatusNotifierWatcher",
        "org.kde.StatusNotifierWatcher",
        "RegisteredStatusNotifierItems",
    )?;
    const ITEM: &str = "org.kde.StatusNotifierItem";
    Some(
        items
            .iter()
            .map(|it| {
                // "<bus name>/<object path>"
                let (dest, path) = match it.find('/') {
                    Some(at) => (&it[..at], &it[at..]),
                    None => (it.as_str(), "/StatusNotifierItem"),
                };
                let title = bus.string(dest, path, ITEM, "Title").map(|t| t.trim().to_string());
                title
                    .filter(|t| !t.is_empty())
                    .or_else(|| bus.string(dest, path, ITEM, "Id").map(|i| tidy_id(i.trim())))
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| dest.to_string())
            })
            .collect(),
    )
}

/// An id as a name: Electron's "vesktop_status_icon_1" becomes "Vesktop".
fn tidy_id(id: &str) -> String {
    let id = match id.rfind("_status_icon_") {
        Some(at) if id[at + 13..].bytes().all(|b| b.is_ascii_digit()) => &id[..at],
        _ => id,
    };
    let mut c = id.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

/// The apps as last read, one per line (written by `update`).
fn cached_apps() -> Option<Vec<String>> {
    let s = fs::read_to_string(dir().join(".apps")).ok()?;
    Some(s.lines().map(String::from).collect())
}

fn save_apps(apps: &[String]) -> bool {
    let path = dir().join(".apps");
    let text: String = apps.iter().map(|a| format!("{}\n", a.replace('\n', " "))).collect();
    if fs::read_to_string(&path).ok().as_deref() == Some(text.as_str()) {
        return false;
    }
    let _ = fs::create_dir_all(dir());
    write_atomic(&path, text.as_bytes());
    true
}

// ── waybar ──

/// Sends the toggle's signal to every waybar.
fn refresh_bars() {
    let sig = unsafe { __libc_current_sigrtmin() } + SIGNAL;
    let Ok(procs) = fs::read_dir("/proc") else { return };
    for p in procs.flatten() {
        let Some(pid) = p.file_name().to_str().and_then(|n| n.parse::<c_int>().ok()) else { continue };
        let comm = fs::read_to_string(p.path().join("comm")).unwrap_or_default();
        if matches!(comm.trim_end(), "waybar" | ".waybar-wrapped") {
            unsafe { kill(pid, sig) };
        }
    }
}

/// Text for pango markup.
fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// A JSON string.
fn js(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn status(compact: bool) {
    let out = std::env::var("WAYBAR_OUTPUT_NAME").unwrap_or_default();
    let (l, r) = if compact { ("", "") } else { ("[", "]") };
    if !out.is_empty() && dir().join(&out).exists() {
        println!(r#"{{"text":"{l}&lt;{r}","tooltip":"click to hide","class":"open"}}"#);
        return;
    }
    let apps = cached_apps().or_else(|| {
        // not read yet: the tray's first update hasn't run
        let a = tray_apps()?;
        save_apps(&a);
        Some(a)
    });
    // no tray apps: no output hides the toggle
    let Some(apps) = apps.filter(|a| !a.is_empty()) else { return };
    let tip = apps.iter().map(|a| esc(a)).collect::<Vec<_>>().join("\n");
    println!(
        r#"{{"text":"{l}{}{r}","tooltip":{},"class":"closed"}}"#,
        apps.len(),
        js(&tip)
    );
}

fn update() {
    // the bars are refreshed only when something changed (each bar's tray
    // runs this, so a change arrives several times)
    if let Some(apps) = tray_apps() {
        if save_apps(&apps) {
            refresh_bars();
        }
    }
}

// ── flip ──

/// The focused screen, which is the clicked bar's: sway focuses the screen
/// under the pointer.
fn focused_output() -> Option<String> {
    let path = std::env::var("SWAYSOCK").ok()?;
    let mut s = UnixStream::connect(path).ok()?;
    s.set_read_timeout(Some(Duration::from_secs(2))).ok()?;
    // GET_OUTPUTS
    let mut msg = Vec::from(&b"i3-ipc"[..]);
    msg.extend_from_slice(&0u32.to_le_bytes());
    msg.extend_from_slice(&3u32.to_le_bytes());
    s.write_all(&msg).ok()?;
    let mut head = [0u8; 14];
    s.read_exact(&mut head).ok()?;
    if &head[..6] != b"i3-ipc" {
        return None;
    }
    let len = u32::from_le_bytes(head[6..10].try_into().ok()?) as usize;
    let mut body = vec![0u8; len.min(16 << 20)];
    s.read_exact(&mut body).ok()?;

    let mut found = None;
    let mut p = json::Parser::new(&body);
    p.array(|p| {
        let (mut name, mut focused) = (String::new(), false);
        p.object(|p, k| match k {
            b"name" => {
                name = p.string()?;
                Some(())
            }
            b"focused" => {
                focused = p.bool()?;
                Some(())
            }
            _ => p.skip(),
        })?;
        if focused {
            found = Some(name);
        }
        Some(())
    })?;
    found.filter(|n| !n.is_empty() && !n.contains('/'))
}

fn now_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0)
}

fn flip() {
    let now = now_ms();
    let Some(out) = focused_output() else { exit(1) };
    let d = dir();
    let _ = fs::create_dir_all(&d);
    let last_file = d.join(format!(".{out}.last"));
    let last = fs::read_to_string(&last_file).ok().and_then(|s| s.trim().parse::<u128>().ok());
    write_atomic(&last_file, now.to_string().as_bytes());
    // a double or triple click toggles the drawer an odd number of times,
    // so a click within the double-click time of the last isn't a new one
    if last.is_some_and(|l| now.saturating_sub(l) < DOUBLE_CLICK.as_millis()) {
        return;
    }
    let state = d.join(&out);
    if state.exists() {
        let _ = fs::remove_file(&state);
    } else {
        let _ = fs::write(&state, b"");
    }
    refresh_bars();
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["status"] => status(false),
        ["status", "--compact"] => status(true),
        ["update"] => update(),
        ["flip"] => flip(),
        _ => {
            eprintln!("usage: tray-toggle status [--compact] | update | flip");
            exit(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::tidy_id;

    #[test]
    fn ids() {
        assert_eq!(tidy_id("vesktop_status_icon_1"), "Vesktop");
        assert_eq!(tidy_id("Claude_status_icon_12"), "Claude");
        assert_eq!(tidy_id("steam"), "Steam");
        assert_eq!(tidy_id("my_status_icon_x"), "My_status_icon_x");
        assert_eq!(tidy_id("élan"), "Élan");
        assert_eq!(tidy_id(""), "");
    }
}
