//! msqu pick: the right-click menu on a block, in fuzzel under the bar.
//!
//!   msqu pick TOOL [SLOT]    what can be done to that one
//!   msqu pick all            everything open (the one block's menu); pick
//!                            one for its menu
//!
//! Runs the msqu-TOOL commands, so they must be on PATH.

use super::notify;
use crate::model::{self, Bar, Running};
use std::io::Write;
use std::process::{Command, Stdio};

enum Do {
    /// msqu-TOOL ARGS
    Run(&'static str, Vec<String>),
    /// pin or hide (kind) on or off
    Mark(&'static str, &'static str, String, bool),
    /// the bar: one block or a block each
    One(bool),
    /// that one's own menu
    Menu(Item),
}

#[derive(Clone)]
struct Item {
    tool: &'static str,
    /// its name in the bar file (the slot, or the tool's name)
    name: String,
    /// "" for a tool without slots
    slot: String,
    /// what it says on the bar
    text: String,
    paused: bool,
}

const ICONS: [(&str, &str); 8] = [
    ("timer", "󰔛"),
    ("stopwatch", "󰁫"),
    ("alarm", "󰀠"),
    ("repeat", "󰑖"),
    ("drink", "󰆫"),
    ("countdown", "󰃭"),
    ("clock", "󰥔"),
    ("notes", "󰂞"),
];

fn icon(tool: &str) -> &'static str {
    ICONS.iter().find(|(t, _)| *t == tool).map_or("", |(_, i)| i)
}

/// "tea 4:59", "Tokyo 21:54", "xmas 63d"
fn describe(r: &Running) -> String {
    let label = |s: String| if r.label.is_empty() { s } else { format!("{} {s}", r.label) };
    match r.tool {
        "alarm" => label(model::clock_text(r.at)),
        "countdown" => label(model::countdown_text(r.at, r.timed).unwrap_or_default()),
        "clock" => match model::zone_time(&r.zone, model::now()) {
            Some((h, m, ..)) => format!("{} {h:02}:{m:02}", if r.label.is_empty() { model::zone_city(&r.zone) } else { r.label.clone() }),
            None => r.zone.clone(),
        },
        "drink" => format!("drink {}m{}", (r.left + 59) / 60, if r.paused { " (paused)" } else { "" }),
        _ => label(format!("{}{}", model::hms(r.left), if r.paused { " (paused)" } else { "" })),
    }
}

/// what's open: running ones, and the notes if a reminder is set
fn items() -> Vec<Item> {
    let mut out: Vec<Item> = model::running()
        .iter()
        .filter(|r| r.tool != "countdown" || model::countdown_text(r.at, r.timed).is_some())
        .map(|r| Item { tool: r.tool, name: model::bar_name(r), slot: r.slot.clone(), text: describe(r), paused: r.paused })
        .collect();
    let mut due: Vec<model::Note> = model::load_notes().into_iter().filter(|n| n.at > 0).collect();
    due.sort_by_key(|n| n.at);
    if let Some(n) = due.first() {
        let more = if due.len() > 1 { format!(" +{}", due.len() - 1) } else { String::new() };
        out.push(Item { tool: "notes", name: "notes".into(), slot: String::new(), text: format!("{} {}{more}", super::hm(n.at), n.text), paused: false });
    }
    out
}

/// what can be done to one
fn options(it: &Item, bar: &Bar) -> Vec<(String, Do)> {
    let slot = || if it.slot.is_empty() { Vec::new() } else { vec![it.slot.clone()] };
    let cmd = |c: &str| {
        let mut a = vec![c.to_string()];
        a.extend(slot());
        a
    };
    let toggle = || (if it.paused { "Resume" } else { "Pause" }.to_string(), Do::Run(it.tool, cmd("toggle")));
    let mut o: Vec<(String, Do)> = match it.tool {
        "timer" => vec![
            toggle(),
            ("Start over".into(), Do::Run("timer", cmd("reset"))),
            ("More or less time".into(), Do::Run("timer", cmd("more"))),
            ("Cancel".into(), Do::Run("timer", cmd("cancel"))),
        ],
        "stopwatch" => vec![toggle(), ("Back to 0".into(), Do::Run("stopwatch", cmd("reset"))), ("Stop".into(), Do::Run("stopwatch", cmd("stop")))],
        "alarm" => vec![("Cancel".into(), Do::Run("alarm", cmd("cancel")))],
        "repeat" => vec![toggle(), ("Start the wait over".into(), Do::Run("repeat", cmd("reset"))), ("Stop".into(), Do::Run("repeat", cmd("cancel")))],
        "drink" => vec![toggle(), ("Start the wait over".into(), Do::Run("drink", cmd("reset"))), ("Stop".into(), Do::Run("drink", cmd("stop")))],
        "countdown" => vec![("Remove".into(), Do::Run("countdown", cmd("cancel")))],
        "clock" => vec![("Remove".into(), Do::Run("clock", cmd("remove")))],
        _ => vec![("Cancel the next reminder".into(), Do::Run("notes", vec!["right-click".into()]))],
    };
    let hidden = bar.has("hide", it.tool, &it.name);
    if matches!(it.tool, "countdown" | "clock" | "notes") {
        o.push((if hidden { "Show on the bar" } else { "Hide from the bar" }.into(), Do::Mark("hide", it.tool, it.name.clone(), !hidden)));
    }
    if bar.one {
        let pinned = bar.has("pin", it.tool, &it.name);
        o.push((if pinned { "Unpin" } else { "Pin to the bar" }.into(), Do::Mark("pin", it.tool, it.name.clone(), !pinned)));
    } else {
        o.push(("Fold everything into one block".into(), Do::One(true)));
    }
    o
}

/// fuzzel in the top-right corner, under the bar: the index picked
fn fuzzel(prompt: &str, lines: &[String]) -> Option<usize> {
    let child = Command::new("fuzzel")
        .args(["--dmenu", "--index", "--anchor", "top-right", "--width", "32"])
        .args(["--lines", &lines.len().clamp(1, 12).to_string(), "--prompt", prompt])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            eprintln!("fuzzel: {e}");
            return None;
        }
    };
    if let Some(mut input) = child.stdin.take() {
        let _ = input.write_all(lines.join("\n").as_bytes());
    }
    let out = child.wait_with_output().ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout).trim().parse().ok()
}

fn run(d: Do) -> i32 {
    match d {
        Do::Run(tool, args) => {
            if let Err(e) = model::run(tool, &args) {
                notify(&["-c", "osd", "-t", "3000", tool, &e]);
                return 1;
            }
        }
        Do::Mark(kind, tool, name, on) => model::set_mark(kind, tool, &name, on),
        Do::One(on) => model::set_bar_one(on),
        Do::Menu(it) => return item_menu(&it),
    }
    0
}

fn item_menu(it: &Item) -> i32 {
    let mut o = options(it, &model::load_bar());
    let labels: Vec<String> = o.iter().map(|(l, _)| l.clone()).collect();
    let Some(i) = fuzzel(&format!("{} {}  ", icon(it.tool), it.text), &labels) else { return 0 };
    if i >= o.len() {
        return 0;
    }
    run(o.swap_remove(i).1)
}

pub fn main(args: &[String]) -> i32 {
    let arg = |i: usize| args.get(i).map(String::as_str).unwrap_or("");
    let all = items();
    if arg(0) == "all" {
        let bar = model::load_bar();
        let mut o: Vec<(String, Do)> = all
            .into_iter()
            .map(|it| {
                let mut l = format!("{} {}", icon(it.tool), it.text);
                if bar.has("pin", it.tool, &it.name) {
                    l += "  (pinned)";
                }
                if bar.has("hide", it.tool, &it.name) {
                    l += "  (hidden)";
                }
                (l, Do::Menu(it))
            })
            .collect();
        o.push(("A block each".into(), Do::One(false)));
        let labels: Vec<String> = o.iter().map(|(l, _)| l.clone()).collect();
        let Some(i) = fuzzel("msqu  ", &labels) else { return 0 };
        if i >= o.len() {
            return 0;
        }
        return run(o.swap_remove(i).1);
    }
    let (tool, slot) = (arg(0), arg(1));
    let found = all.into_iter().find(|it| it.tool == tool && (tool == "notes" || it.slot == slot));
    match found {
        Some(it) => item_menu(&it),
        // gone since the click
        None if model::TOOLS.contains(&tool) || tool == "notes" => 0,
        None => {
            eprintln!("usage: msqu pick TOOL [SLOT] | all");
            2
        }
    }
}
