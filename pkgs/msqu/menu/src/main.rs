//! msqu-ui: the Super+Shift+A menu for msqu's tools, in the floating
//! terminal. A home screen (what's running, starting something new,
//! presets, recent), and a short form per tool. The tools themselves
//! (alarms, sounds) are the msqu-TOOL scripts and the bar blocks are
//! msqu-bar; this only asks and then runs the scripts.
//!
//! Usage: msqu-ui --config FILE [timer|repeat|alarm|drink]
//!   FILE: "color NAME HEX", "preset LABEL|EVERY|WINDOW" and
//!   "slots TOOL N" lines
//!   timer / repeat / alarm / drink: open straight on that form

use msqu_ui::{model, term};

use model::{Preset, Recent, Running, calc, calc_text, clock, duration, hms, rough, split_clock, split_when};
use term::{BOLD, Key, Pal, RESET, cut, width};

/// what Enter does on a home row
#[derive(Clone, Debug)]
enum Act {
    /// a tool, and which one if it has slots
    Toggle(&'static str, String),
    StartTimer { secs: i64, label: String },
    StartRepeat { secs: i64, window: String, label: String },
    /// replace: the slot of an alarm this changes
    StartAlarm { h: u32, m: u32, label: String, replace: Option<String> },
    StartDrink { secs: i64 },
    StartStopwatch,
    /// a tool's form, filled in with this ("25 tea", "7:30 wake up")
    Open(&'static str, String),
    /// empties RECENT, on a second Enter
    ClearRecent,
    /// a sum's answer, to the clipboard
    Copy(String),
    /// the same, on its first field: changing a running alarm (its slot)
    Change(&'static str, String, String),
}

impl Act {
    /// the tool this starts, if it starts one
    fn starts(&self) -> Option<&'static str> {
        Some(match self {
            Act::StartTimer { .. } => "timer",
            Act::StartRepeat { .. } => "repeat",
            Act::StartAlarm { .. } => "alarm",
            Act::StartDrink { .. } => "drink",
            Act::StartStopwatch => "stopwatch",
            _ => return None,
        })
    }
}

struct Row {
    section: &'static str,
    /// whose colour the icon is in
    tool: &'static str,
    icon: &'static str,
    title: String,
    note: String,
    act: Act,
    /// a running tool (and its slot): Ctrl+X / Delete stops it
    cancel: Option<(&'static str, String)>,
    /// a recent entry: Ctrl+X / Delete takes it out of the list
    forget: Option<Recent>,
}

#[derive(PartialEq, Clone, Copy)]
enum Field {
    When,
    Label,
    List,
}

struct Form {
    tool: &'static str,
    when: String,
    label: String,
    filter: String,
    focus: Field,
    sel: usize,
    /// repeat: "nothing" plus the open windows
    windows: Vec<(String, String, String)>,
    /// changing a running alarm: its slot
    replace: Option<String>,
}

enum Screen {
    Home,
    Form(Form),
}

struct App {
    pal: Pal,
    presets: Vec<Preset>,
    /// tools that run several at once: how many
    slots: std::collections::HashMap<String, usize>,
    names: std::collections::HashMap<String, String>,
    recent: Vec<Recent>,
    screen: Screen,
    query: String,
    sel: usize,
    error: String,
    /// Enter was pressed once on "clear all": the next Enter clears
    confirm_clear: bool,
}

const TIMER: &str = "󰔛";
const REPEAT: &str = "󰑖";
const STOPWATCH: &str = "󰁫";
const ALARM: &str = "󰀠";
const DRINK: &str = "󰆫";
const PAUSED: &str = "󰏤";
const CLEAR: &str = "󰗩";
const CALC: &str = "󰃬";

fn icon(tool: &str) -> &'static str {
    match tool {
        "timer" => TIMER,
        "repeat" => REPEAT,
        "stopwatch" => STOPWATCH,
        "alarm" => ALARM,
        _ => DRINK,
    }
}

/// a form's time field, read the way that tool reads it: seconds, or for
/// the alarm the next time the clock says it
fn when_secs(tool: &str, when: &str) -> Option<i64> {
    if tool == "alarm" {
        clock(when).map(|(h, m)| model::next_at(h, m))
    } else {
        duration(when)
    }
}

/// what Ctrl+X does to a running one
fn stop_word(tool: &str) -> &'static str {
    if matches!(tool, "timer" | "alarm") { "cancel" } else { "stop" }
}

impl App {
    /// "app_id=org.vinegarhq.Sober" -> "Sober"
    fn window_name(&self, window: &str) -> String {
        match window.split_once('=') {
            Some((_, id)) => self.names.get(&id.to_lowercase()).cloned().unwrap_or_else(|| id.to_string()),
            None => "nothing".into(),
        }
    }

    fn home_rows(&self) -> Vec<Row> {
        let mut rows = Vec::new();
        let running = model::running();
        // say when starting would replace a running one, or when there's
        // no room for another
        let replaces = |tool: &str, note: String| {
            let extra = self.blocked(&running, tool);
            match (note.is_empty(), extra) {
                (_, None) => note,
                (true, Some(e)) => e,
                (false, Some(e)) => format!("{note} · {e}"),
            }
        };
        // a sum: its answer ("1024x8" = 8192), enter copies it
        let q = self.query.trim();
        if let Some(v) = calc(q) {
            let answer = calc_text(v);
            rows.push(Row {
                section: "CALC",
                tool: "calc",
                icon: CALC,
                title: format!("= {answer}"),
                note: q.to_string(),
                act: Act::Copy(answer),
                cancel: None,
                forget: None,
            });
            return rows;
        }
        // typing a time starts it straight away: "25 tea", "7pm", or with
        // the tool first: "t 25 tea", "r 15", "a 7:30 wake up", "d 45"
        let (tool, rest) = match q.split_once(' ') {
            Some(("t" | "timer", r)) => ("timer", r),
            Some(("r" | "repeat", r)) => ("repeat", r),
            Some(("a" | "at" | "alarm", r)) => ("alarm", r),
            Some(("d" | "drink", r)) => ("drink", r),
            // "7pm", "7:30 pm": an alarm ("7:30" alone is a timer)
            _ if split_clock(q).is_some()
                && (duration(&split_when(q).0).is_none() || q.split_whitespace().nth(1).is_some_and(|w| matches!(w.to_lowercase().as_str(), "am" | "pm"))) =>
            {
                ("alarm", q)
            }
            _ => ("timer", q),
        };
        let start = |icon: &'static str, title: String, note: String, act: Act| Row { section: "START", tool, icon, title, note, act, cancel: None, forget: None };
        if tool == "alarm" {
            if let Some(((h, m), label)) = split_clock(rest) {
                let at = model::next_at(h, m);
                let note = [label.as_str(), &format!("in {}", rough(at - model::now()))].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(" · ");
                rows.push(start(ALARM, format!("alarm {}", model::clock_text(at)), replaces("alarm", note), Act::StartAlarm { h, m, label, replace: None }));
                return rows;
            }
        } else {
            let (when, label) = split_when(rest);
            if let Some(secs) = duration(&when) {
                rows.push(match tool {
                    "timer" => start(TIMER, format!("timer {}", hms(secs)), replaces("timer", label.clone()), Act::StartTimer { secs, label }),
                    "drink" => start(DRINK, format!("drink every {}", hms(secs)), replaces("drink", String::new()), Act::StartDrink { secs }),
                    _ => start(REPEAT, format!("repeat every {}", hms(secs)), replaces("repeat", "pick a window next".into()), Act::Open("repeat", rest.to_string())),
                });
                return rows;
            }
        }

        for r in running.clone() {
            let (title, act) = match r.tool {
                // enter on an alarm: change it
                "alarm" => (
                    format!("alarm {}", model::clock_text(r.at)),
                    Act::Change("alarm", format!("{} {}", model::clock_text(r.at).rsplit(' ').next().unwrap_or(""), r.label), r.slot.clone()),
                ),
                t => (format!("{t} {}", hms(r.left)), Act::Toggle(r.tool, r.slot.clone())),
            };
            rows.push(Row {
                section: "RUNNING",
                tool: r.tool,
                icon: if r.paused { PAUSED } else { icon(r.tool) },
                title,
                note: running_note(self, &r),
                act,
                cancel: Some((r.tool, r.slot.clone())),
                forget: None,
            });
        }
        for (tool, note, act) in [
            ("timer", replaces("timer", "count down, then ring".into()), Act::Open("timer", String::new())),
            ("stopwatch", if running.iter().any(|r| r.tool == "stopwatch") { "starts again from 0".into() } else { "count up, starts now".into() }, Act::StartStopwatch),
            ("alarm", replaces("alarm", "ring at a time of day".into()), Act::Open("alarm", String::new())),
            ("repeat", "every so often, can bring up a window".into(), Act::Open("repeat", String::new())),
            ("drink", "a reminder to drink water, every so often".into(), Act::Open("drink", hms(self.last_drink()))),
        ] {
            rows.push(Row { section: "NEW", tool, icon: icon(tool), title: tool.into(), note, act, cancel: None, forget: None });
        }
        for p in &self.presets {
            if let Some(secs) = duration(&p.every) {
                let to = if p.window == "-" { String::new() } else { format!(" · brings up {}", self.window_name(&p.window)) };
                rows.push(Row {
                    section: "PRESETS",
                    tool: "repeat",
                    icon: REPEAT,
                    title: p.label.clone(),
                    note: format!("every {}{to}", hms(secs)),
                    act: Act::StartRepeat { secs, window: p.window.clone(), label: p.label.clone() },
                    cancel: None,
                    forget: None,
                });
            }
        }
        for r in &self.recent {
            let row = match r {
                Recent::Timer { secs, label } => Row {
                    section: "RECENT",
                    tool: "timer",
                    icon: TIMER,
                    title: format!("timer {}", hms(*secs)),
                    note: label.clone(),
                    act: Act::StartTimer { secs: *secs, label: label.clone() },
                    cancel: None,
                    forget: Some(r.clone()),
                },
                Recent::Repeat { secs, window, label } => Row {
                    section: "RECENT",
                    tool: "repeat",
                    icon: REPEAT,
                    title: format!("repeat every {}", hms(*secs)),
                    note: {
                        let name = self.window_name(window);
                        if window == "-" || name == *label { label.clone() } else { format!("{label} · brings up {name}") }
                    },
                    act: Act::StartRepeat { secs: *secs, window: window.clone(), label: label.clone() },
                    cancel: None,
                    forget: Some(r.clone()),
                },
                Recent::Alarm { h, m, label } => Row {
                    section: "RECENT",
                    tool: "alarm",
                    icon: ALARM,
                    title: format!("alarm {h:02}:{m:02}"),
                    note: label.clone(),
                    act: Act::StartAlarm { h: *h, m: *m, label: label.clone(), replace: None },
                    cancel: None,
                    forget: Some(r.clone()),
                },
                Recent::Drink { secs } => Row {
                    section: "RECENT",
                    tool: "drink",
                    icon: DRINK,
                    title: format!("drink every {}", hms(*secs)),
                    note: String::new(),
                    act: Act::StartDrink { secs: *secs },
                    cancel: None,
                    forget: Some(r.clone()),
                },
            };
            rows.push(row);
        }
        if !self.recent.is_empty() {
            rows.push(Row {
                section: "RECENT",
                tool: "",
                icon: CLEAR,
                title: "clear all".into(),
                note: if self.confirm_clear { "enter again to clear the list".into() } else { "empty the recent list".into() },
                act: Act::ClearRecent,
                cancel: None,
                forget: None,
            });
        }
        // every word typed must appear somewhere in the row
        let terms: Vec<String> = q.to_lowercase().split_whitespace().map(String::from).collect();
        rows.retain(|r| {
            let text = format!("{} {} {}", r.section, r.title, r.note).to_lowercase();
            terms.iter().all(|t| text.contains(t))
        });
        rows
    }

    /// the drink reminder's last interval, or 45 minutes
    fn last_drink(&self) -> i64 {
        self.recent
            .iter()
            .find_map(|r| match r {
                Recent::Drink { secs } => Some(*secs),
                _ => None,
            })
            .unwrap_or(45 * 60)
    }

    /// starting another `tool` now: replaces the running one (most tools),
    /// or can't, all its slots being in use (timer, alarm). None if it
    /// just starts.
    fn blocked(&self, running: &[Running], tool: &str) -> Option<String> {
        let n = running.iter().filter(|r| r.tool == tool).count();
        match self.slots.get(tool) {
            Some(&max) if n >= max => Some(format!("{max} {tool}s {} already: cancel one first", if tool == "alarm" { "set" } else { "running" })),
            Some(_) => None,
            None if n > 0 => Some(format!("replaces the running {tool}")),
            None => None,
        }
    }

    fn open_form(&mut self, tool: &'static str, prefill: &str) {
        let (when, label) = if tool == "alarm" {
            match split_clock(prefill) {
                // as typed, so "7:30 pm" stays as it was
                Some((_, label)) => (prefill.trim().strip_suffix(label.as_str()).unwrap_or(prefill).trim().to_string(), label),
                None => (prefill.trim().to_string(), String::new()),
            }
        } else {
            split_when(prefill)
        };
        let windows = if tool == "repeat" {
            let mut w = vec![("-".to_string(), String::new(), String::new())];
            w.extend(model::windows());
            w
        } else {
            Vec::new()
        };
        self.error.clear();
        self.screen = Screen::Form(Form {
            tool,
            focus: match tool {
                _ if when_secs(tool, &when).is_none() => Field::When,
                "repeat" => Field::List,
                "drink" => Field::When,
                _ => Field::Label,
            },
            when,
            label,
            filter: String::new(),
            sel: 0,
            windows,
            replace: None,
        });
    }

    /// starts something; Ok(true) closes the menu
    fn act(&mut self, a: Act) -> Result<bool, String> {
        match a {
            Act::Toggle(tool, slot) => model::run(tool, &["toggle".into(), slot]).map(|_| true),
            Act::StartTimer { secs, label } => {
                model::run("timer", &["start".into(), secs.to_string(), label.clone()])?;
                model::remember(Recent::Timer { secs, label });
                Ok(true)
            }
            Act::StartRepeat { secs, window, label } => {
                model::run("repeat", &["start".into(), secs.to_string(), window.clone(), label.clone()])?;
                model::remember(Recent::Repeat { secs, window, label });
                Ok(true)
            }
            Act::StartAlarm { h, m, label, replace } => {
                let at = model::next_at(h, m);
                // changing one: in its place
                let mut args = vec!["start".into(), at.to_string(), label.clone()];
                args.extend(replace);
                model::run("alarm", &args)?;
                model::remember(Recent::Alarm { h, m, label });
                Ok(true)
            }
            Act::StartDrink { secs } => {
                model::run("drink", &["start".into(), secs.to_string()])?;
                model::remember(Recent::Drink { secs });
                Ok(true)
            }
            Act::StartStopwatch => model::run("stopwatch", &["start".into()]).map(|_| true),
            Act::Copy(text) => model::copy(&text).map(|_| true),
            Act::ClearRecent => {
                if self.confirm_clear {
                    model::clear_recent();
                    self.recent.clear();
                    self.confirm_clear = false;
                } else {
                    self.confirm_clear = true;
                }
                Ok(false)
            }
            Act::Open(tool, p) => {
                self.open_form(tool, &p);
                Ok(false)
            }
            Act::Change(tool, p, slot) => {
                self.open_form(tool, &p);
                if let Screen::Form(f) = &mut self.screen {
                    f.focus = Field::When;
                    f.replace = Some(slot);
                }
                Ok(false)
            }
        }
    }

    /// the form's windows matching its filter (indexes into windows)
    fn shown_windows(&self, f: &Form) -> Vec<usize> {
        let terms: Vec<String> = f.filter.to_lowercase().split_whitespace().map(String::from).collect();
        (0..f.windows.len())
            .filter(|&i| {
                let (win, id, title) = &f.windows[i];
                let text = if win == "-" { "nothing just remind me".to_string() } else { format!("{} {id} {title}", self.window_name(win)) };
                terms.iter().all(|t| text.to_lowercase().contains(t))
            })
            .collect()
    }

    /// handles a key; true quits
    fn key(&mut self, k: Key) -> bool {
        if k == Key::Quit {
            return true;
        }
        match &mut self.screen {
            Screen::Home => {
                let rows = self.home_rows();
                self.sel = self.sel.min(rows.len().saturating_sub(1));
                // "clear all" asks for a second Enter, straight after
                if k != Key::Enter {
                    self.confirm_clear = false;
                }
                match k {
                    Key::Esc if !self.query.is_empty() => {
                        self.query.clear();
                        self.sel = 0;
                    }
                    Key::Esc => return true,
                    Key::Up => self.sel = self.sel.saturating_sub(1),
                    Key::Down | Key::Tab => self.sel = (self.sel + 1).min(rows.len().saturating_sub(1)),
                    Key::BackTab => self.sel = self.sel.saturating_sub(1),
                    Key::Home | Key::PageUp => self.sel = 0,
                    Key::End | Key::PageDown => self.sel = rows.len().saturating_sub(1),
                    Key::Enter => {
                        if let Some(r) = rows.into_iter().nth(self.sel) {
                            match self.act(r.act) {
                                Ok(quit) => return quit,
                                Err(e) => self.error = e,
                            }
                        }
                    }
                    Key::CtrlX | Key::Delete => {
                        let row = rows.get(self.sel);
                        if let Some((tool, slot)) = row.and_then(|r| r.cancel.clone()) {
                            match model::run(tool, &["cancel".into(), slot]) {
                                Ok(()) => self.sel = 0,
                                Err(e) => self.error = e,
                            }
                        } else if let Some(r) = row.and_then(|r| r.forget.clone()) {
                            // stays where it is, on the next row
                            model::forget(&r);
                            self.recent = model::load_recent();
                        }
                    }
                    Key::Backspace => {
                        self.query.pop();
                        self.sel = 0;
                    }
                    Key::CtrlU => {
                        self.query.clear();
                        self.sel = 0;
                    }
                    Key::CtrlW => {
                        let t = self.query.trim_end().len();
                        self.query.truncate(self.query[..t].rfind(' ').map_or(0, |i| i + 1));
                        self.sel = 0;
                    }
                    Key::Char(c) => {
                        self.query.push(c);
                        self.sel = 0;
                    }
                    _ => {}
                }
            }
            Screen::Form(_) => return self.form_key(k),
        }
        false
    }

    fn form_key(&mut self, k: Key) -> bool {
        let Screen::Form(f) = &self.screen else { return false };
        let shown = self.shown_windows(f);
        let Screen::Form(f) = &mut self.screen else { return false };
        let order: &[Field] = match f.tool {
            "repeat" => &[Field::When, Field::Label, Field::List],
            "drink" => &[Field::When],
            _ => &[Field::When, Field::Label],
        };
        let at = order.iter().position(|x| *x == f.focus).unwrap_or(0);
        let text = match f.focus {
            Field::When => Some(&mut f.when),
            Field::Label => Some(&mut f.label),
            Field::List => Some(&mut f.filter),
        };
        match k {
            Key::Esc => {
                self.screen = Screen::Home;
                self.error.clear();
            }
            Key::Tab => f.focus = order[(at + 1) % order.len()],
            Key::BackTab => f.focus = order[(at + order.len() - 1) % order.len()],
            Key::Up if f.focus == Field::List => f.sel = f.sel.saturating_sub(1),
            Key::Down if f.focus == Field::List => f.sel = (f.sel + 1).min(shown.len().saturating_sub(1)),
            Key::Up => f.focus = order[at.saturating_sub(1)],
            Key::Down => f.focus = order[(at + 1).min(order.len() - 1)],
            Key::Enter => {
                let Some(secs) = when_secs(f.tool, &f.when) else {
                    f.focus = Field::When;
                    self.error = if f.tool == "alarm" { "what time? e.g. 7:30, 19:45, 7pm".into() } else { "how long? e.g. 5 (minutes), 1:30, 90s, 1h 30m".into() };
                    return false;
                };
                // Enter on the time, with more fields to go: next field
                if f.focus != Field::List && f.tool == "repeat" {
                    f.focus = order[at + 1];
                    return false;
                }
                let act = if f.tool == "timer" {
                    Act::StartTimer { secs, label: f.label.trim().to_string() }
                } else if f.tool == "drink" {
                    Act::StartDrink { secs }
                } else if f.tool == "alarm" {
                    let (h, m) = clock(&f.when).unwrap_or((0, 0));
                    Act::StartAlarm { h, m, label: f.label.trim().to_string(), replace: f.replace.clone() }
                } else {
                    let window = shown.get(f.sel).map(|&i| f.windows[i].0.clone()).unwrap_or_else(|| "-".into());
                    let label = f.label.trim().to_string();
                    let label = if !label.is_empty() {
                        label
                    } else if window == "-" {
                        "reminder".into()
                    } else {
                        self.window_name(&window)
                    };
                    Act::StartRepeat { secs, window, label }
                };
                match self.act(act) {
                    Ok(quit) => return quit,
                    Err(e) => self.error = e,
                }
            }
            Key::Backspace => {
                if let Some(t) = text {
                    t.pop();
                }
            }
            Key::CtrlU => {
                if let Some(t) = text {
                    t.clear();
                }
            }
            Key::CtrlW => {
                if let Some(t) = text {
                    let e = t.trim_end().len();
                    t.truncate(t[..e].rfind(' ').map_or(0, |i| i + 1));
                }
            }
            Key::Char(c) => {
                if let Some(t) = text {
                    t.push(c);
                }
                if f.focus == Field::List {
                    f.sel = 0;
                }
                self.error.clear();
            }
            _ => {}
        }
        false
    }

    // ── drawing ──

    fn draw(&self) {
        let (rows, cols) = term::size();
        let p = &self.pal;
        let mut b = String::from("\x1b[H");
        let mut lines: Vec<String> = Vec::new();
        // on a running row: what Enter and Ctrl+X do to it
        let running_hint = match &self.screen {
            Screen::Home => self.home_rows().into_iter().nth(self.sel).and_then(|r| r.cancel.map(|(t, _)| (t, r.icon == PAUSED))),
            _ => None,
        }
        .map(|(tool, paused)| {
            let enter = if tool == "alarm" { "change" } else if paused { "resume" } else { "pause" };
            format!("enter {enter} · ctrl+x {} · ↑↓ move · esc quit", stop_word(tool))
        });
        // starting the selected row would replace a running one, or there's
        // no room; on a recent row, ctrl+x takes it out of the list
        let replace_hint = match &self.screen {
            Screen::Home => self.home_rows().into_iter().nth(self.sel).and_then(|r| {
                let tool = r.act.starts()?;
                let blocked = self.blocked(&model::running(), tool);
                let enter = blocked.map_or("enter start".to_string(), |e| format!("enter start ({e})"));
                match (r.forget.is_some(), enter.len() > "enter start".len()) {
                    (true, _) => Some(format!("{enter} · ctrl+x remove · ↑↓ move · esc quit")),
                    (false, true) => Some(format!("{enter} · ↑↓ move · esc quit")),
                    (false, false) => None,
                }
            }),
            _ => None,
        };
        let (crumb, input, hints) = match &self.screen {
            Screen::Home => {
                let act = self.home_rows().into_iter().nth(self.sel).map(|r| r.act);
                let clear_hint = match (&act, self.confirm_clear) {
                    (Some(Act::ClearRecent), true) => Some("enter again to clear · any other key keeps them"),
                    (Some(Act::ClearRecent), false) => Some("enter clear all recent · ↑↓ move · esc quit"),
                    (Some(Act::Copy(_)), _) => Some("enter copy the answer · esc quit"),
                    _ => None,
                };
                ("", self.query.clone(), running_hint.as_deref().or(replace_hint.as_deref()).or(clear_hint).unwrap_or("↑↓ move · enter start · esc quit"))
            }
            Screen::Form(f) => (
                f.tool,
                String::new(),
                match f.tool {
                    "repeat" => "tab next · ↑↓ pick · type to filter · enter start · esc back",
                    "drink" => "enter start · esc back",
                    _ if f.replace.is_some() => "tab next · enter save · esc back",
                    _ => "tab next · enter start · esc back",
                },
            ),
        };
        // top line: " msqu › query▏" or " msqu › timer"
        let mut top = format!(" {}{BOLD}msqu{RESET} {}›{RESET} ", p.accent, p.muted);
        if !crumb.is_empty() {
            top += &format!("{}{crumb}{RESET}", p.bright);
        } else {
            top += &format!("{}{}{RESET}{}▏{RESET}", p.bright, cut(&input, cols.saturating_sub(12)), p.accent);
            if input.is_empty() {
                top += &format!("{}25 starts a timer, 7pm an alarm, 4+4 sums; or search{RESET}", p.muted);
            }
        }
        lines.push(top);
        lines.push(format!("{}{}{RESET}", p.line, "─".repeat(cols)));

        match &self.screen {
            Screen::Home => self.draw_home(&mut lines, cols, rows.saturating_sub(4)),
            Screen::Form(f) => self.draw_form(f, &mut lines, cols, rows.saturating_sub(4)),
        }

        let body = rows.saturating_sub(2);
        lines.truncate(body);
        while lines.len() < body {
            lines.push(String::new());
        }
        lines.push(format!("{}{}{RESET}", p.line, "─".repeat(cols)));
        if self.error.is_empty() {
            lines.push(format!(" {}{}{RESET}", p.muted, cut(hints, cols.saturating_sub(2))));
        } else {
            lines.push(format!(" {}{}{RESET}", p.red, cut(&self.error, cols.saturating_sub(2))));
        }
        for (i, l) in lines.iter().enumerate() {
            b += l;
            b += "\x1b[K";
            if i + 1 < lines.len() {
                b += "\r\n";
            }
        }
        term::out(&b);
    }

    fn draw_home(&self, lines: &mut Vec<String>, cols: usize, room: usize) {
        let p = &self.pal;
        let rows = self.home_rows();
        if rows.is_empty() {
            lines.push(format!("  {}nothing matches{RESET}", p.muted));
            return;
        }
        // keep the selected row in view
        let mut out: Vec<(usize, String)> = Vec::new(); // (row index or MAX for headers, text)
        let mut last = "";
        for (i, r) in rows.iter().enumerate() {
            if r.section != last {
                if !out.is_empty() {
                    out.push((usize::MAX, String::new()));
                }
                out.push((usize::MAX, format!(" {}{BOLD}{}{RESET}", p.accent, r.section)));
                last = r.section;
            }
            let selected = i == self.sel;
            let title_w = 22.min(cols / 3);
            let title = cut(&r.title, title_w);
            let pad = " ".repeat(title_w.saturating_sub(width(&title)) + 2);
            let note_room = cols.saturating_sub(title_w + 9);
            let (lead, tc, nc, bg) = if selected {
                (format!("{}›{RESET}", p.accent), p.bright.clone(), p.fg.clone(), p.surface_bg.clone())
            } else {
                (" ".into(), p.fg.clone(), p.muted.clone(), String::new())
            };
            // the same colours as the bar
            let icon_c = match (r.icon, r.tool) {
                (PAUSED | CLEAR, _) => p.muted.clone(),
                (CALC, _) => p.accent.clone(),
                (_, "timer") => p.yellow.clone(),
                (_, "repeat") => p.blue.clone(),
                (_, "stopwatch") => p.green.clone(),
                (_, "alarm") => p.orange.clone(),
                _ => p.magenta.clone(),
            };
            let line = format!(
                "{bg} {lead}{bg} {icon_c}{}{RESET}{bg} {tc}{title}{pad}{nc}{}{RESET}",
                r.icon,
                cut(&r.note, note_room)
            );
            // fill the selected row's background to the edge
            let fill = if selected { format!("{bg}{}{RESET}", " ".repeat(cols.saturating_sub(5 + title_w + 2 + width(&cut(&r.note, note_room))))) } else { String::new() };
            out.push((i, line + &fill));
        }
        let at = out.iter().position(|(i, _)| *i == self.sel).unwrap_or(0);
        let start = (at + 1).saturating_sub(room);
        lines.extend(out.into_iter().skip(start).take(room).map(|(_, l)| l));
    }

    fn draw_form(&self, f: &Form, lines: &mut Vec<String>, cols: usize, room: usize) {
        let p = &self.pal;
        let field = |lines: &mut Vec<String>, name: &str, value: &str, focused: bool, help: String| {
            let mark = if focused { format!("{}›{RESET}", p.accent) } else { " ".into() };
            lines.push(format!(" {mark} {}{name}{RESET}", if focused { p.bright.clone() } else { p.fg.clone() }));
            let cursor = if focused { format!("{}▏{RESET}", p.accent) } else { String::new() };
            lines.push(format!("     {}{}{RESET}{cursor}   {help}", p.bright, cut(value, cols.saturating_sub(30))));
        };
        let what = match f.tool {
            "timer" => "How long",
            "alarm" => "At",
            _ => "Every",
        };
        let help = match when_secs(f.tool, &f.when) {
            Some(at) if f.tool == "alarm" => format!("{}→ {} (in {}){RESET}", p.green, model::clock_text(at), rough(at - model::now())),
            Some(s) => format!("{}→ {}{RESET}", p.green, hms(s)),
            None if f.tool == "alarm" && f.when.trim().is_empty() => format!("{}7:30 · 19:45 · 7pm · 6:45am · 7 (7:00){RESET}", p.muted),
            None if f.tool == "alarm" => format!("{}can't read that: try 7:30, 19:45 or 7pm{RESET}", p.red),
            None if f.when.trim().is_empty() => format!("{}5 = minutes · 1:30 · 90s · 1h 30m{RESET}", p.muted),
            None => format!("{}can't read that: try 5, 1:30, 90s or 1h 30m{RESET}", p.red),
        };
        let running = model::running();
        let old = running.iter().find(|r| r.tool == f.tool && f.replace.as_ref().is_none_or(|s| *s == r.slot));
        match old {
            Some(r) if f.replace.is_some() => lines.push(format!("   {}changes the alarm at {}{RESET}", p.yellow, model::clock_text(r.at))),
            _ if f.replace.is_some() => lines.push(String::new()),
            Some(r) if !self.slots.contains_key(f.tool) => {
                lines.push(format!("   {}starting this replaces the running {} ({} left){RESET}", p.yellow, f.tool, hms(r.left)))
            }
            _ => match self.blocked(&running, f.tool) {
                Some(e) => lines.push(format!("   {}{e}{RESET}", p.red)),
                None => lines.push(String::new()),
            },
        }
        field(lines, what, &f.when, f.focus == Field::When, help);
        if f.tool == "drink" {
            lines.push(String::new());
            lines.push(format!("     {}a notification every so often (not while locked); click{RESET}", p.muted));
            lines.push(format!("     {}its block on the bar to pause, right-click to stop{RESET}", p.muted));
            return;
        }
        lines.push(String::new());
        let label_help = match f.tool {
            "timer" => "optional: shown on the bar and when it rings",
            "alarm" => "optional: shown when it rings",
            _ => "optional: defaults to the window's name",
        };
        field(lines, "Label", &f.label, f.focus == Field::Label, format!("{}{label_help}{RESET}", p.muted));
        if f.tool != "repeat" {
            return;
        }
        lines.push(String::new());
        let focused = f.focus == Field::List;
        let mark = if focused { format!("{}›{RESET}", p.accent) } else { " ".into() };
        let filter = if focused || !f.filter.is_empty() {
            format!("  {}{}{RESET}{}", p.bright, f.filter, if focused { format!("{}▏{RESET}", p.accent) } else { String::new() })
        } else {
            String::new()
        };
        lines.push(format!(" {mark} {}Bring up{RESET}{filter}", if focused { p.bright.clone() } else { p.fg.clone() }));
        let shown = self.shown_windows(f);
        let left = room.saturating_sub(lines.len() + 1).max(1);
        let start = (f.sel + 1).saturating_sub(left);
        for (n, &i) in shown.iter().enumerate().skip(start).take(left) {
            let (win, id, title) = &f.windows[i];
            let selected = n == f.sel;
            let (lead, bg, nc) = if selected {
                (format!("{}›{RESET}", p.accent), p.surface_bg.clone(), p.bright.clone())
            } else {
                (" ".into(), String::new(), p.fg.clone())
            };
            let text = if win == "-" {
                format!("{nc}nothing, just remind me{RESET}")
            } else {
                let full = self.window_name(win);
                let name = cut(&full, 18);
                let pad = " ".repeat(18usize.saturating_sub(width(&name)) + 2);
                // the app ID too, unless it's just the name again
                let id = if id.eq_ignore_ascii_case(&full) { String::new() } else { cut(id, 30) };
                let room = cols.saturating_sub(30 + width(&id));
                format!("{nc}{name}{pad}{bg}{}{}{RESET}{bg}  {}{id}{RESET}", p.fg, cut(title, room), p.muted)
            };
            lines.push(format!("{bg}   {lead}{bg} {text}"));
        }
        if shown.is_empty() {
            lines.push(format!("     {}no window matches{RESET}", p.muted));
        }
    }
}

fn running_note(app: &App, r: &Running) -> String {
    let mut parts = Vec::new();
    if !r.label.is_empty() {
        parts.push(r.label.clone());
    }
    if r.tool == "timer" {
        parts.push(format!("of {}", hms(r.total)));
    } else if r.tool == "alarm" {
        parts.push(format!("in {}", rough(r.left)));
    } else if r.tool == "drink" {
        parts.push(format!("every {}", hms(r.total)));
    } else if r.tool == "repeat" {
        parts.push(format!("every {}", hms(r.total)));
        let name = app.window_name(&r.window);
        if r.window != "-" && name != r.label {
            parts.push(format!("brings up {name}"));
        }
    }
    if r.paused {
        parts.push("paused".into());
    }
    parts.join(" · ")
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut config = String::new();
    let mut open = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--config" => config = args.next().unwrap_or_default(),
            "timer" | "repeat" | "alarm" | "drink" => open = Some(a),
            // no form: starts straight from the home screen
            "stopwatch" => {}
            _ => {
                eprintln!("usage: msqu-ui --config FILE [timer|repeat|alarm|drink]");
                std::process::exit(2);
            }
        }
    }
    let cfg = model::parse_config(&std::fs::read_to_string(&config).unwrap_or_default());
    let mut app = App {
        pal: Pal::new(&cfg.colors),
        presets: cfg.presets,
        slots: cfg.slots,
        names: model::app_names(),
        recent: model::load_recent(),
        screen: Screen::Home,
        query: String::new(),
        sel: 0,
        error: String::new(),
        confirm_clear: false,
    };
    if let Some(tool) = open.as_deref() {
        let tool = model::TOOLS.into_iter().find(|t| *t == tool).unwrap_or("timer");
        let prefill = if tool == "drink" { hms(app.last_drink()) } else { String::new() };
        app.open_form(tool, &prefill);
    }
    let raw = match term::Raw::enter() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("msqu-ui: not a terminal: {e}");
            std::process::exit(1);
        }
    };
    term::out(&format!("{}\x1b[2J", app.pal.bg));
    loop {
        if matches!(app.screen, Screen::Home) {
            app.sel = app.sel.min(app.home_rows().len().saturating_sub(1));
        }
        // resized: start from a clear screen, so nothing of the old size is left
        if term::resized() {
            term::out(&format!("{}\x1b[2J", app.pal.bg));
        }
        app.draw();
        // wake on the next whole second, so countdowns keep moving
        let ms = 1000 - (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_millis()).unwrap_or(0) as i32) + 5;
        let mut quit = false;
        for k in term::keys(ms) {
            if app.key(k) {
                quit = true;
                break;
            }
        }
        if quit {
            break;
        }
    }
    drop(raw);
}
