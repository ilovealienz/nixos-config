//! sway-helpers: small additions to sway, in one process.
//!
//!   group-drag  Super+Shift+drag moves a whole tab group
//!   app-group   related apps join one tabbed group when they open
//!   group-end   a tab group left with one window ends (10s later if moved)
//!   attention   XWayland attention requests are counted in the title
//!
//! Each runs in its own thread with its own sway connections. If one
//! panics it is started again (after 1s, then longer if it keeps failing)
//! and the others carry on. Losing sway itself ends the program, so
//! systemd can start it again for the new sway.
//!
//! Standard library only.
//!
//! Usage: sway-helpers [--group NAME=APP,APP...]... [--title-suffix TEXT]
//!                     [--no-group-drag] [--no-app-group] [--no-group-end]
//!                     [--no-attention]

mod app_group;
mod attention;
mod group_drag;
mod group_end;
mod ipc;
mod json;
mod x11;

use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;
use std::process::exit;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

extern "C" {
    // glibc
    fn malloc_trim(pad: usize) -> i32;
    fn mallopt(param: i32, value: i32) -> i32;
}

const M_ARENA_MAX: i32 = -8;

/// Gives freed heap memory back to the system (after reading a tree).
pub fn trim() {
    unsafe {
        malloc_trim(0);
    }
}

/// Ends the whole program; systemd starts it again.
pub fn die(feature: &str, msg: &str) -> ! {
    eprintln!("{feature}: {msg}");
    exit(1)
}

/// $XDG_STATE_HOME/<rel>, for the flag files the toggles create.
pub fn state_file(rel: &str) -> PathBuf {
    let base = std::env::var_os("XDG_STATE_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var_os("HOME").unwrap_or_default();
            PathBuf::from(home).join(".local/state")
        });
    base.join(rel)
}

pub struct Config {
    pub groups: Vec<(String, Vec<String>)>, // name -> lower-cased apps
    pub title_suffix: String,
}

/// Small stacks: nothing here recurses deeper than sway's tree.
pub const STACK: usize = 256 * 1024;

/// Runs `feature` in its own thread, starting it again if it panics.
fn supervise(name: &'static str, cfg: Arc<Config>, feature: fn(&Config)) {
    thread::Builder::new()
        .name(name.into())
        .stack_size(STACK)
        .spawn(move || {
            let mut wait = Duration::from_secs(1);
            loop {
                let started = Instant::now();
                match panic::catch_unwind(AssertUnwindSafe(|| feature(&cfg))) {
                    Ok(()) => return,
                    Err(_) => {
                        // the panic message was already printed
                        if started.elapsed() > Duration::from_secs(60) {
                            wait = Duration::from_secs(1);
                        }
                        eprintln!("{name}: restarting in {}s", wait.as_secs());
                        thread::sleep(wait);
                        wait = (wait * 2).min(Duration::from_secs(30));
                    }
                }
            }
        })
        .unwrap_or_else(|e| die(name, &format!("can't start thread: {e}")));
}

fn usage(msg: &str) -> ! {
    eprintln!("sway-helpers: {msg}");
    eprintln!("usage: sway-helpers [--group NAME=APP,APP...]... [--title-suffix TEXT]");
    eprintln!("                    [--no-group-drag] [--no-app-group] [--no-group-end]");
    eprintln!("                    [--no-attention]");
    exit(2)
}

fn main() {
    // one malloc arena for all threads: they're idle nearly all the time,
    // and an arena each would keep more memory around
    unsafe {
        mallopt(M_ARENA_MAX, 1);
    }

    let mut cfg = Config { groups: Vec::new(), title_suffix: String::new() };
    let (mut drag, mut group, mut end, mut attention) = (true, true, true, true);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--group" => {
                let g = args.next().unwrap_or_else(|| usage("--group needs NAME=APP,APP..."));
                let (name, list) = g
                    .split_once('=')
                    .filter(|(n, l)| !n.is_empty() && !l.is_empty())
                    .unwrap_or_else(|| usage(&format!("bad group {g:?}, expected NAME=APP,APP...")));
                let apps = list
                    .split(',')
                    .map(|a| a.trim().to_lowercase())
                    .filter(|a| !a.is_empty())
                    .collect();
                cfg.groups.push((name.to_string(), apps));
            }
            "--title-suffix" => {
                cfg.title_suffix = args.next().unwrap_or_else(|| usage("--title-suffix needs a value"));
            }
            "--no-group-drag" => drag = false,
            "--no-app-group" => group = false,
            "--no-group-end" => end = false,
            "--no-attention" => attention = false,
            other => usage(&format!("unknown argument {other:?}")),
        }
    }
    if cfg.groups.is_empty() {
        group = false;
    }
    if !(drag || group || end || attention) {
        usage("nothing to do");
    }

    let cfg = Arc::new(cfg);
    if drag {
        supervise("group-drag", cfg.clone(), group_drag::run);
    }
    if group {
        supervise("app-group", cfg.clone(), app_group::run);
    }
    if end {
        supervise("group-end", cfg.clone(), group_end::run);
    }
    if attention {
        supervise("attention", cfg.clone(), attention::run);
    }
    // the features end the program themselves if sway goes away
    loop {
        thread::park();
    }
}
