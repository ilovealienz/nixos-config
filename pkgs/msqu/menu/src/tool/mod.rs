//! The tools (timer, stopwatch, alarm, repeat, drink), one module each,
//! and what they share (once lib.sh): the state directory and its lock,
//! writing state files in one go, slots (for tools that run several at
//! once), the alarm that runs a tool's "ring" at a set time (`arm`,
//! `disarm`, `stale`), the bell, a ringing notification (`ring`), and
//! running the programs they use (notify-send, systemd-run, systemctl,
//! pw-play, makoctl, fuzzel).
//!
//! desktop/msqu.nix runs each tool through a launcher that sets MSQU_TOOL,
//! MSQU_BELL, MSQU_SELF (the launcher itself, which an alarm runs again)
//! and, for tools that have them, MSQU_SLOTS and MSQU_ALARM.
//!
//! Each tool is a module here with a `main` taking its arguments and
//! returning the exit code; msqu-tool (src/bin/msqu-tool.rs) picks one.
//! Their commands, exit codes and state files are the same as the shell
//! scripts' they replaced. sway.rs talks to sway for repeat.

pub mod alarm;
pub mod drink;
pub mod repeat;
pub mod stopwatch;
pub mod sway;
pub mod timer;

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A tool's state directory, $XDG_RUNTIME_DIR/msqu/TOOL, held locked.
///
/// One command of a tool at a time: two clicks close together (or a click
/// while an alarm goes off) would otherwise interleave. The lock is held
/// until the command ends, or `unlock` before anything slow. It's a file
/// beside the directory (msqu/TOOL.lock), so taking it doesn't wake the
/// bar blocks watching the directory. Programs the tool runs don't get it
/// (close-on-exec), so one left running can't hold it.
pub struct State {
    pub tool: &'static str,
    pub dir: PathBuf,
    /// which one, for a tool that runs several at once ("" for the others);
    /// the alarm goes by it
    pub slot: String,
    lock: File,
}

impl State {
    pub fn open(tool: &'static str) -> State {
        let base = crate::model::state_dir();
        let dir = base.join(tool);
        if let Err(e) = fs::create_dir_all(&dir) {
            die(tool, &format!("can't make {}: {e}", dir.display()));
        }
        let path = base.join(format!("{tool}.lock"));
        // std opens files close-on-exec
        let lock = OpenOptions::new().write(true).create(true).truncate(true).open(&path);
        let lock = lock.unwrap_or_else(|e| die(tool, &format!("can't open {}: {e}", path.display())));
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX) } != 0 {
            die(tool, &format!("can't lock {}: {}", path.display(), std::io::Error::last_os_error()));
        }
        State { tool, dir, slot: String::new(), lock }
    }

    /// Lets the next command of this tool run.
    pub fn unlock(&mut self) {
        unsafe { libc::flock(self.lock.as_raw_fd(), libc::LOCK_UN) };
    }

    /// Takes the lock again after `unlock`, waiting for any command that
    /// has it.
    pub fn relock(&mut self) {
        if unsafe { libc::flock(self.lock.as_raw_fd(), libc::LOCK_EX) } != 0 {
            die(self.tool, &format!("can't lock: {}", std::io::Error::last_os_error()));
        }
    }

    /// A state file's first line ("" if there's none).
    pub fn read(&self, name: &str) -> String {
        let text = fs::read_to_string(self.dir.join(name)).unwrap_or_default();
        text.lines().next().unwrap_or("").to_string()
    }

    /// Writes a state file in one go (to .NAME.tmp, then renamed), so the
    /// bar never reads half of it.
    pub fn put(&self, name: &str, content: &str) {
        let tmp = self.dir.join(format!(".{name}.tmp"));
        let done = File::create(&tmp)
            .and_then(|mut f| f.write_all(format!("{content}\n").as_bytes()))
            .and_then(|_| fs::rename(&tmp, self.dir.join(name)));
        if let Err(e) = done {
            die(self.tool, &format!("can't write {name}: {e}"));
        }
    }

    pub fn remove(&self, name: &str) {
        match fs::remove_file(self.dir.join(name)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => die(self.tool, &format!("can't remove {name}: {e}")),
            _ => {}
        }
    }

    /// How many this tool runs at once (MSQU_SLOTS; 1 if it isn't set, as
    /// the scripts' free_slot and any_slot had it).
    fn slots(&self) -> u32 {
        std::env::var("MSQU_SLOTS").ok().and_then(|s| s.parse().ok()).unwrap_or(1)
    }

    /// The first slot with nothing in it; None if they're all in use.
    pub fn free_slot(&self) -> Option<String> {
        (1..=self.slots()).map(|i| i.to_string()).find(|i| !self.dir.join(i).exists())
    }

    /// Something is in one of the slots.
    pub fn any_slot(&self) -> bool {
        (1..=self.slots()).any(|i| self.dir.join(i.to_string()).exists())
    }

    /// Works on slot `n`, if it's one of this tool's; otherwise says so and
    /// gives the exit code (2).
    pub fn use_slot(&mut self, n: &str) -> Result<(), i32> {
        let max = std::env::var("MSQU_SLOTS").ok().and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
        let ok = n.bytes().all(|b| b.is_ascii_digit()) && !n.starts_with('0') && n.parse::<u64>().is_ok_and(|i| i <= max);
        if !ok {
            let n = if n.is_empty() { "(none given)" } else { n };
            eprintln!("msqu {}: no slot {n}, there are {max}", self.tool);
            return Err(2);
        }
        self.slot = n.to_string();
        Ok(())
    }

    /// The alarm's systemd unit name, before its time: msqu-TOOL or
    /// msqu-TOOL-SLOT.
    fn unit(&self) -> String {
        if self.slot.is_empty() { format!("msqu-{}", self.tool) } else { format!("msqu-{}-{}", self.tool, self.slot) }
    }

    /// Stops this tool's (or slot's) alarm, if it has one. Only the timers:
    /// a "ring" that arms the next alarm (drink) runs inside the last
    /// alarm's service, and stopping that would kill it halfway.
    pub fn disarm(&self) {
        let u = self.unit();
        quiet("systemctl", &["--user", "stop", &format!("{u}.timer"), &format!("{u}-*.timer")]);
    }

    /// The alarm: runs this tool's "ring" (with the slot, if any) at unix
    /// time `at`, as a systemd user timer, so it survives reloading sway or
    /// waybar. A calendar time rather than "in N seconds": time suspended
    /// counts, and one that came due while suspended goes off on resume.
    ///
    /// Each alarm is its own unit, msqu-TOOL[-SLOT]-AT-NANOSECONDS (new
    /// even when re-armed for the same second, before systemd has dropped
    /// the old one), with MSQU_AT set to its time for `stale`. --collect
    /// and RemainAfterElapse=no: the units go once it has rung.
    ///
    /// Err(exit code) if systemd-run failed (its message is on stderr).
    pub fn arm(&self, at: i64) -> Result<(), i32> {
        self.disarm();
        let u = self.unit();
        quiet("systemctl", &["--user", "reset-failed", &format!("{u}-*")]);
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0);
        let mut cmd = Command::new("systemd-run");
        cmd.args(["--user", "--quiet", "--collect"])
            .arg(format!("--unit={u}-{at}-{nanos:09}"))
            .arg(format!("--setenv=MSQU_AT={at}"))
            // a unix time: systemd reads "@SECONDS" as that moment
            .arg(format!("--on-calendar=@{at}"))
            .args(["--timer-property=AccuracySec=1s", "--timer-property=RemainAfterElapse=no"]);
        // the launcher, so the ring gets the same environment; run
        // straight from msqu-tool (testing), msqu-tool TOOL
        match std::env::var_os("MSQU_SELF").filter(|v| !v.is_empty()) {
            Some(me) => cmd.arg(me),
            None => cmd.arg(std::env::current_exe().unwrap_or_else(|_| "msqu-tool".into())).arg(self.tool),
        };
        cmd.arg("ring");
        if !self.slot.is_empty() {
            cmd.arg(&self.slot);
        }
        match status(&mut cmd) {
            0 => Ok(()),
            code => Err(code),
        }
    }

    /// The bell (MSQU_BELL), unless do-not-disturb is on. Unlocks first:
    /// it takes a while.
    pub fn bell(&mut self) {
        self.unlock();
        if !dnd() {
            if let Some(bell) = std::env::var_os("MSQU_BELL") {
                status(Command::new("pw-play").arg(bell));
            }
        }
    }
}

/// How long a ringing notification's sound keeps going if it isn't
/// closed: 10 minutes (MSQU_RING_LIMIT, in seconds, for testing).
fn ring_limit() -> Duration {
    Duration::from_secs(std::env::var("MSQU_RING_LIMIT").ok().and_then(|s| s.parse().ok()).unwrap_or(600))
}

impl State {
    /// A notification that stays until it's closed, with the alarm sound
    /// (MSQU_ALARM) playing over and over until then. The sound gives up
    /// after 10 minutes, leaving the notification up. If the notification
    /// can't be shown (no notification daemon), the sound plays once.
    /// Unlocks first: this takes as long as you leave it.
    pub fn ring(&mut self, summary: &str, body: &str) {
        self.unlock();
        let sound = std::env::var_os("MSQU_ALARM").filter(|s| !s.is_empty());
        let stop = Arc::new(AtomicBool::new(false));
        // the pw-play playing now. The player thread waits for it to end
        // without reaping it (WNOWAIT), then reaps it with the lock held, so
        // the main thread can stop it (under the lock) without its pid ever
        // having been reused
        let playing: Arc<Mutex<Option<Child>>> = Arc::default();
        let player = sound.clone().map(|sound| {
            let (stop, playing) = (stop.clone(), playing.clone());
            thread::spawn(move || {
                while !stop.load(Ordering::SeqCst) {
                    let ok = match Command::new("pw-play").arg(&sound).stdin(Stdio::null()).spawn() {
                        Ok(c) => {
                            let pid = c.id();
                            {
                                // told to stop while it was starting (the main
                                // thread found nothing to stop): stop it now
                                let mut p = playing.lock().unwrap();
                                if stop.load(Ordering::SeqCst) {
                                    unsafe { libc::kill(pid as i32, libc::SIGTERM) };
                                }
                                *p = Some(c);
                            }
                            wait_exit(pid);
                            let child = playing.lock().unwrap().take();
                            child.and_then(|mut c| c.wait().ok()).is_some_and(|s| s.success())
                        }
                        Err(_) => false,
                    };
                    // it failed: try again after a second (unless told to stop)
                    if !ok {
                        for _ in 0..10 {
                            if stop.load(Ordering::SeqCst) {
                                break;
                            }
                            thread::sleep(Duration::from_millis(100));
                        }
                    }
                }
            })
        });

        // -w: until it's closed; -t 0: mako's default timeout doesn't close it
        let shown = Command::new("notify-send")
            .args(["-w", "-t", "0", "-u", "critical", "-c", "msqu-ring", summary, body])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn();
        let rc = match shown {
            Ok(mut c) => {
                // waited for on another thread, without reaping it, so it can
                // still be stopped (by its pid) once the time is up
                let (done, ended) = std::sync::mpsc::channel();
                let pid = c.id();
                thread::spawn(move || {
                    wait_exit(pid);
                    let _ = done.send(());
                });
                if ended.recv_timeout(ring_limit()).is_err() {
                    // the 10 minutes are up: stop waiting (the notification stays)
                    unsafe { libc::kill(pid as i32, libc::SIGTERM) };
                    let _ = c.wait();
                    124
                } else {
                    c.wait().map(|s| s.code().unwrap_or(1)).unwrap_or(1)
                }
            }
            Err(e) => {
                eprintln!("notify-send: {e}");
                127
            }
        };

        // stop the sound: TERM, as the shell's kill did (pw-play ends its
        // stream cleanly); the player thread reaps it. `stop` is set before
        // taking the lock, so one starting just now is stopped by the
        // player thread instead
        stop.store(true, Ordering::SeqCst);
        if let Some(c) = playing.lock().unwrap().as_ref() {
            unsafe { libc::kill(c.id() as i32, libc::SIGTERM) };
        }
        if let Some(p) = player {
            let _ = p.join();
        }
        // it failed rather than timing out (no notification daemon?), so
        // nothing to close: play the sound through once instead
        if rc != 0 && rc != 124 {
            if let Some(sound) = sound {
                status(Command::new("pw-play").arg(sound));
            }
        }
    }
}

/// Waits until child `pid` has ended, without reaping it: until it's
/// reaped (Child::wait) its pid can't be reused, so it's still safe to
/// signal.
fn wait_exit(pid: u32) {
    loop {
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        let r = unsafe { libc::waitid(libc::P_PID, pid, &mut info, libc::WEXITED | libc::WNOWAIT) };
        if r == 0 || std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted {
            return;
        }
    }
}

/// Unix time -> "07:30", local time.
pub fn hm(at: i64) -> String {
    let t = at as libc::time_t;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    unsafe { libc::localtime_r(&t, &mut tm) };
    format!("{:02}:{:02}", tm.tm_hour, tm.tm_min)
}

/// In "ring": true if this ring's alarm isn't the one in the state file
/// any more (it's not running, or it's armed for another time): it went
/// off just as a click replaced it. Rings without MSQU_AT (armed by the
/// first versions) are never stale.
pub fn stale(mode: &str, t: &str) -> bool {
    match std::env::var("MSQU_AT") {
        Ok(at) if !at.is_empty() => (mode != "run" && mode != "at") || t != at,
        _ => false,
    }
}

/// Do-not-disturb is on (mako's "dnd" mode, the DND icon in waybar).
pub fn dnd() -> bool {
    match Command::new("makoctl").arg("mode").stdin(Stdio::null()).stderr(Stdio::null()).output() {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).lines().any(|l| l == "dnd"),
        _ => false,
    }
}

/// The screen is locked: swaylock is running (or its Nix wrapper, should
/// it ever get one; process names are cut to 15 characters).
pub fn screen_locked() -> bool {
    let Ok(procs) = fs::read_dir("/proc") else { return false };
    procs.flatten().any(|p| {
        let pid = p.file_name();
        pid.to_str().is_some_and(|s| s.bytes().all(|b| b.is_ascii_digit()))
            && fs::read_to_string(p.path().join("comm"))
                .is_ok_and(|c| matches!(c.trim_end_matches('\n'), "swaylock" | ".swaylock-wrapp"))
    })
}

/// A state file's line split as the scripts' `read -r a b c ...` split it
/// into `n` variables: a word each, the last getting the rest of the line
/// as it is (spaces inside kept, the ends trimmed); "" for any missing.
pub fn fields(line: &str, n: usize) -> Vec<String> {
    let ws = |c: char| c == ' ' || c == '\t' || c == '\n';
    let mut rest = line.trim_matches(ws);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        if i + 1 == n {
            out.push(rest.to_string());
            break;
        }
        let end = rest.find(ws).unwrap_or(rest.len());
        out.push(rest[..end].to_string());
        rest = rest[end..].trim_start_matches(ws);
    }
    out
}

/// `fields` into three: the first word, the second, and the rest.
pub fn words(line: &str) -> (String, String, String) {
    let mut f = fields(line, 3).into_iter();
    let mut next = || f.next().unwrap_or_default();
    (next(), next(), next())
}

/// A number from a state file (0 if it isn't one).
pub fn num(s: &str) -> i64 {
    s.parse().unwrap_or(0)
}

/// Runs a command, waiting for it; its exit code (127 if it can't be run,
/// as in the shell).
pub fn status(cmd: &mut Command) -> i32 {
    match cmd.stdin(Stdio::null()).status() {
        Ok(s) => s.code().unwrap_or(1),
        Err(e) => {
            eprintln!("{}: {e}", cmd.get_program().to_string_lossy());
            127
        }
    }
}

/// Runs a command whose failure doesn't matter (the scripts'
/// `2>/dev/null || true`).
fn quiet(prog: &str, args: &[&str]) {
    let _ = Command::new(prog).args(args).stdin(Stdio::null()).stderr(Stdio::null()).status();
}

/// Runs notify-send with these arguments; its exit code.
pub fn notify(args: &[&str]) -> i32 {
    status(Command::new("notify-send").args(args))
}

/// Exits with a message, like a failing command in the scripts.
pub fn die(tool: &str, msg: &str) -> ! {
    eprintln!("msqu {tool}: {msg}");
    std::process::exit(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_words() {
        assert_eq!(words("run 100 0"), ("run".into(), "100".into(), "0".into()));
        assert_eq!(words("  pause\t42  a  b "), ("pause".into(), "42".into(), "a  b".into()));
        assert_eq!(words("at 5 tea\ttime"), ("at".into(), "5".into(), "tea\ttime".into()));
        assert_eq!(words("run 100"), ("run".into(), "100".into(), String::new()));
        assert_eq!(words(""), (String::new(), String::new(), String::new()));
        assert_eq!((num("42"), num("-3"), num(""), num("abc")), (42, -3, 0, 0));
        assert_eq!(fields("run 5 60 app_id=x  my  label ", 5), ["run", "5", "60", "app_id=x", "my  label"]);
        assert_eq!(fields("run 5 60", 5), ["run", "5", "60", "", ""]);
        assert_eq!(fields("", 2), ["", ""]);
    }
}
