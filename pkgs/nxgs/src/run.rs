//! Running an entry in gamescope, and starting it detached from the TUI.

use crate::apps;
use crate::entry::{Entry, Runs};
use crate::outputs;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

/// the shell command an entry runs, or why it can't run here
pub fn command(e: &Entry) -> Result<String, String> {
    match &e.runs {
        Runs::Command(c) => Ok(c.clone()),
        Runs::Steam => Err(format!("{} starts from Steam: c copies its launch option", e.name)),
        Runs::App(id) => {
            let app = apps::find(&apps::dirs(), id).ok_or_else(|| format!("{id} isn't installed"))?;
            if app.is_steam_game() {
                return Err(format!(
                    "{} starts through Steam: put the launch option in its Properties instead",
                    app.name
                ));
            }
            Ok(app.command)
        }
    }
}

/// gamescope's full command line
pub fn argv(e: &Entry, cmd: &str) -> Vec<String> {
    let mut a = Vec::new();
    if e.gamemode {
        a.push("gamemoderun".to_string());
    }
    a.push("gamescope".into());
    a.extend(e.gamescope_args());
    // through sh, so quotes and VAR=value prefixes work as typed
    a.extend(["--".into(), "sh".into(), "-c".into(), cmd.to_string()]);
    a
}

/// `nxgs run`: focus its monitor, then become gamescope
pub fn exec(e: &Entry) -> String {
    let cmd = match command(e) {
        Ok(c) => c,
        Err(msg) => return msg,
    };
    outputs::focus(&e.monitor);
    let a = argv(e, &cmd);
    let err = Command::new(&a[0]).args(&a[1..]).exec();
    format!("couldn't start {}: {err}", a[0])
}

/// from the TUI: `nxgs run <slug>` started by sway, so it outlives the TUI
/// and its terminal
pub fn spawn(e: &Entry) -> Result<(), String> {
    command(e)?;
    let line = format!("nxgs run {}", e.slug());
    let via_sway = std::env::var_os("SWAYSOCK").is_some()
        && Command::new("swaymsg")
            .args(["-q", "exec", &line])
            .status()
            .is_ok_and(|s| s.success());
    if via_sway {
        return Ok(());
    }
    Command::new("setsid")
        .args(["-f", "nxgs", "run", &e.slug()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|err| format!("couldn't start it: {err}"))
        .and_then(|s| if s.success() { Ok(()) } else { Err("couldn't start it".into()) })
}

/// to the clipboard (wl-copy)
pub fn copy(text: &str) -> Result<(), String> {
    use std::io::Write;
    let mut child = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("wl-copy: {e}"))?;
    child
        .stdin
        .take()
        .ok_or("wl-copy: no stdin")?
        .write_all(text.as_bytes())
        .map_err(|e| format!("wl-copy: {e}"))?;
    child.wait().map_err(|e| format!("wl-copy: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_argv() {
        let mut e = Entry::new("Sober", Runs::Command("flatpak run org.vinegarhq.Sober".into()));
        e.gamemode = true;
        let a = argv(&e, "flatpak run org.vinegarhq.Sober");
        assert_eq!(a[..2], ["gamemoderun", "gamescope"]);
        assert_eq!(a[a.len() - 4..], ["--", "sh", "-c", "flatpak run org.vinegarhq.Sober"]);
        assert!(a.contains(&"-f".to_string()));
    }

    #[test]
    fn missing_app_says_so() {
        let e = Entry::new("Gone", Runs::App("not-installed-nxgs-test.desktop".into()));
        assert!(command(&e).unwrap_err().contains("isn't installed"));
    }
}
