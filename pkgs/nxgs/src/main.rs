//! nxgs: games and apps run in gamescope at a set size, so they don't
//! trip over the monitor layout. Saved entries, launched from here or with
//! `nxgs run`, plus Steam launch options.
//!
//! Usage: nxgs [--colors FILE]   the list
//!        nxgs run NAME           run an entry (its name or slug)
//!        nxgs list               the entries' names
//!        nxgs on OUTPUT CMD...   focus a monitor, then run CMD (for launch options)

mod apps;
mod entry;
mod fuzzy;
mod outputs;
mod run;
mod theme;

use entry::{Entry, Runs, Window};
use outputs::Output;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Paragraph, Wrap};
use std::path::PathBuf;
use theme::Palette;

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut colors = None;
    if let Some(i) = args.iter().position(|a| a == "--colors") {
        if i + 1 < args.len() {
            colors = Some(args.remove(i + 1));
        }
        args.remove(i);
    }
    let dir = entry::dir();
    match args.first().map(String::as_str) {
        Some("run") => {
            let name = args[1..].join(" ");
            let Some(e) = entry::find(&dir, &name) else {
                eprintln!("nxgs: no entry {name:?} (nxgs list shows them)");
                std::process::exit(1);
            };
            eprintln!("nxgs: {}", run::exec(&e));
            std::process::exit(1);
        }
        Some("on") if args.len() >= 3 => {
            use std::os::unix::process::CommandExt;
            outputs::focus(&args[1]);
            let err = std::process::Command::new(&args[2]).args(&args[3..]).exec();
            eprintln!("nxgs: couldn't start {}: {err}", args[2]);
            std::process::exit(1);
        }
        Some("list") => {
            for e in entry::load_all(&dir) {
                println!("{}\t{}", e.slug(), e.name);
            }
            return;
        }
        Some(_) => {
            eprintln!("usage: nxgs [run NAME | list | on OUTPUT CMD...]");
            std::process::exit(2);
        }
        None => {}
    }
    let pal = colors
        .and_then(|f| std::fs::read_to_string(f).ok())
        .map(|t| Palette::from_json(&t))
        .unwrap_or_default();
    let mut app = App::new(dir, pal, outputs::list());
    let mut term = ratatui::init();
    let result = app.run(&mut term);
    ratatui::restore();
    if let Err(e) = result {
        eprintln!("nxgs: {e}");
        std::process::exit(1);
    }
}

/// a setting in the form
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
    Name,
    Runs,
    Monitor,
    Width,
    Height,
    Refresh,
    Window,
    Grab,
    Vrr,
    Fps,
    UnfocusedFps,
    Fsr,
    Render,
    Sharpness,
    Gamemode,
}

const FIELDS: [Field; 15] = [
    Field::Name,
    Field::Runs,
    Field::Monitor,
    Field::Width,
    Field::Height,
    Field::Refresh,
    Field::Window,
    Field::Grab,
    Field::Vrr,
    Field::Fps,
    Field::UnfocusedFps,
    Field::Fsr,
    Field::Render,
    Field::Sharpness,
    Field::Gamemode,
];

struct Form {
    entry: Entry,
    /// the entry's slug before editing (None: a new one)
    was: Option<String>,
    sel: usize,
    /// a digit typed on this field already (the first one replaces the number)
    typed: bool,
    /// quick copy: settings only, nothing saved
    quick: bool,
}

impl Form {
    fn new(entry: Entry, was: Option<String>) -> Form {
        Form { entry, was, sel: 0, typed: false, quick: false }
    }

    /// the fields shown: no name or command for quick copy, and the FSR
    /// settings only with FSR on
    fn fields(&self) -> Vec<Field> {
        FIELDS
            .into_iter()
            .filter(|f| !(self.quick && matches!(f, Field::Name | Field::Runs)))
            .filter(|f| self.entry.fsr || !matches!(f, Field::Render | Field::Sharpness))
            .collect()
    }

    fn field(&self) -> Field {
        let fields = self.fields();
        fields[self.sel.min(fields.len() - 1)]
    }
}

enum Popup {
    Help,
    Copy,
    /// what to add: a desktop app or a command
    Add,
    Delete,
}

enum Screen {
    List,
    /// picking a desktop app to add
    Pick { apps: Vec<apps::App>, filter: String, sel: usize },
    Form(Form),
}

struct App {
    dir: PathBuf,
    /// quick copy's last settings
    quick_file: PathBuf,
    pal: Palette,
    outputs: Vec<Output>,
    entries: Vec<Entry>,
    sel: usize,
    screen: Screen,
    popup: Option<Popup>,
    message: Option<(String, bool)>,
    quit: bool,
}

impl App {
    fn new(dir: PathBuf, pal: Palette, outputs: Vec<Output>) -> App {
        let entries = entry::load_all(&dir);
        App {
            quick_file: dir.with_file_name("quick.conf"),
            dir,
            pal,
            outputs,
            entries,
            sel: 0,
            screen: Screen::List,
            popup: None,
            message: None,
            quit: false,
        }
    }

    fn reload(&mut self, keep: Option<&str>) {
        self.entries = entry::load_all(&self.dir);
        if let Some(slug) = keep
            && let Some(i) = self.entries.iter().position(|e| e.slug() == slug)
        {
            self.sel = i;
        }
        self.sel = self.sel.min(self.entries.len().saturating_sub(1));
    }

    fn selected(&self) -> Option<&Entry> {
        self.entries.get(self.sel)
    }

    fn say(&mut self, msg: impl Into<String>, err: bool) {
        self.message = Some((msg.into(), err));
    }

    /// a new entry's settings: the focused monitor's size and refresh rate
    fn fresh(&self, name: &str, runs: Runs) -> Entry {
        let mut e = Entry::new(name, runs);
        if let Some(o) = self.outputs.first() {
            e.monitor = o.name.clone();
            e.width = o.width;
            e.height = o.height;
            e.refresh = o.refresh;
        }
        e
    }

    fn run(&mut self, term: &mut DefaultTerminal) -> std::io::Result<()> {
        while !self.quit {
            term.draw(|f| self.draw(f))?;
            if let Event::Key(k) = event::read()?
                && k.kind == KeyEventKind::Press
            {
                self.key(k);
            }
        }
        Ok(())
    }

    fn key(&mut self, k: KeyEvent) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        if ctrl && matches!(k.code, KeyCode::Char('c' | 'q')) {
            self.quit = true;
            return;
        }
        self.message = None;
        if let Some(p) = self.popup.take() {
            self.popup_key(p, k);
            return;
        }
        match self.screen {
            Screen::List => self.list_key(k),
            Screen::Pick { .. } => self.pick_key(k),
            Screen::Form(_) => self.form_key(k),
        }
    }

    // ── the list ──

    fn list_key(&mut self, k: KeyEvent) {
        let last = self.entries.len().saturating_sub(1);
        match k.code {
            KeyCode::Esc | KeyCode::Char('q') => self.quit = true,
            KeyCode::Up | KeyCode::Char('k') => self.sel = self.sel.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => self.sel = (self.sel + 1).min(last),
            KeyCode::Home => self.sel = 0,
            KeyCode::End => self.sel = last,
            KeyCode::Char('?') => self.popup = Some(Popup::Help),
            KeyCode::Char('a') => self.popup = Some(Popup::Add),
            KeyCode::Char('o') => self.open_quick(),
            KeyCode::Enter => {
                if let Some(e) = self.selected().cloned() {
                    match run::spawn(&e) {
                        Ok(()) => self.quit = true,
                        Err(msg) => self.say(msg, true),
                    }
                }
            }
            KeyCode::Char('e') => {
                if let Some(e) = self.selected().cloned() {
                    let was = Some(e.slug());
                    self.screen = Screen::Form(Form::new(e, was));
                }
            }
            KeyCode::Char('c') => {
                // one thing to copy: straight away, otherwise ask which
                if let Some(e) = self.selected().cloned() {
                    match options(&e, false).as_slice() {
                        [(c, _, _)] => self.copy_option(&e, *c, false),
                        _ => self.popup = Some(Popup::Copy),
                    }
                }
            }
            KeyCode::Char('d') if self.selected().is_some() => self.popup = Some(Popup::Delete),
            KeyCode::Char('p') => self.toggle_permanent(),
            _ => {}
        }
    }

    /// the app's launcher entry opens it in gamescope, or back to normal
    fn toggle_permanent(&mut self) {
        let Some(e) = self.selected().cloned() else { return };
        let Runs::App(id) = &e.runs else {
            self.say("only desktop apps can be made permanent", true);
            return;
        };
        let owner = apps::pinned_by(id);
        if owner.as_deref() == Some(e.slug().as_str()) {
            match apps::remove_copy(id) {
                Ok(_) => self.say(format!("{} opens normally again", e.name), false),
                Err(msg) => self.say(msg, true),
            }
            return;
        }
        let Some(app) = apps::find(&apps::dirs(), id) else {
            self.say(format!("{id} isn't installed"), true);
            return;
        };
        if app.is_steam_game() {
            self.say("a Steam game: use the launch option (c) instead", true);
            return;
        }
        // another entry for the same app had it: this one takes over
        if owner.is_some() {
            let _ = apps::remove_copy(id);
        }
        match apps::install_copy(&app, &format!("nxgs run {}", e.slug()), &e.slug()) {
            Ok(_) => self.say(format!("{} now always opens in gamescope with these settings", app.name), false),
            Err(msg) => self.say(msg, true),
        }
    }

    fn popup_key(&mut self, p: Popup, k: KeyEvent) {
        match p {
            Popup::Help => {}
            Popup::Add => match k.code {
                KeyCode::Char('a') => {
                    self.screen = Screen::Pick {
                        apps: apps::scan(&apps::dirs(), false),
                        filter: String::new(),
                        sel: 0,
                    };
                }
                KeyCode::Char('c') => {
                    let e = self.fresh("", Runs::Command(String::new()));
                    self.screen = Screen::Form(Form::new(e, None));
                }
                KeyCode::Char('s') => {
                    let e = self.fresh("", Runs::Steam);
                    self.screen = Screen::Form(Form::new(e, None));
                }
                _ => {}
            },
            Popup::Copy => {
                let Some(e) = self.selected().cloned() else { return };
                if let KeyCode::Char(c) = k.code {
                    self.copy_option(&e, c, false);
                }
            }
            Popup::Delete => {
                if k.code == KeyCode::Char('y')
                    && let Some(e) = self.selected().cloned()
                {
                    // its launcher copy goes with it (not another entry's)
                    if let Runs::App(id) = &e.runs
                        && apps::pinned_by(id).as_deref() == Some(e.slug().as_str())
                    {
                        let _ = apps::remove_copy(id);
                    }
                    match entry::remove(&self.dir, &e) {
                        Ok(()) => self.say(format!("deleted {}", e.name), false),
                        Err(err) => self.say(format!("couldn't delete it: {err}"), true),
                    }
                    self.reload(None);
                }
            }
        }
    }

    // ── picking an app ──

    fn shown_apps(apps: &[apps::App], filter: &str) -> Vec<usize> {
        let mut scored: Vec<(i32, usize)> = apps
            .iter()
            .enumerate()
            .filter_map(|(i, a)| fuzzy::score(filter, &a.name).map(|(s, _)| (s, i)))
            .collect();
        if !filter.trim().is_empty() {
            scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        }
        scored.into_iter().map(|(_, i)| i).collect()
    }

    fn pick_key(&mut self, k: KeyEvent) {
        let Screen::Pick { apps, filter, sel } = &mut self.screen else { return };
        let shown = Self::shown_apps(apps, filter);
        match k.code {
            KeyCode::Esc if !filter.is_empty() => {
                filter.clear();
                *sel = 0;
            }
            KeyCode::Esc => self.screen = Screen::List,
            KeyCode::Up => *sel = sel.saturating_sub(1),
            KeyCode::Down => *sel = (*sel + 1).min(shown.len().saturating_sub(1)),
            KeyCode::Backspace => {
                filter.pop();
                *sel = 0;
            }
            KeyCode::Char(c) => {
                filter.push(c);
                *sel = 0;
            }
            KeyCode::Enter => {
                let Some(app) = shown.get(*sel).map(|&i| apps[i].clone()) else { return };
                // Steam starts its own games: only the launch option helps
                let runs = if app.is_steam_game() { Runs::Steam } else { Runs::App(app.id.clone()) };
                let e = self.fresh(&app.name, runs);
                self.screen = Screen::Form(Form::new(e, None));
            }
            _ => {}
        }
    }

    // ── quick copy ──

    /// the settings form with nothing to save: copy a launch option and go
    fn open_quick(&mut self) {
        let e = std::fs::read_to_string(&self.quick_file)
            .ok()
            .and_then(|t| Entry::from_text(&t))
            .unwrap_or_else(|| self.fresh("quick", Runs::Steam));
        let mut f = Form::new(e, None);
        f.quick = true;
        self.screen = Screen::Form(f);
    }

    /// keeps quick copy's settings for next time
    fn keep_quick(&self, e: &Entry) {
        if let Some(d) = self.quick_file.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let _ = std::fs::write(&self.quick_file, e.to_text());
    }

    fn copy_option(&mut self, e: &Entry, c: char, quick: bool) {
        if e.width == 0 || e.height == 0 {
            self.say("the size can't be 0", true);
            return;
        }
        let mut e = e.clone();
        e.render = e.render.clamp(25, 100);
        e.sharpness = e.sharpness.min(20);
        let Some((_, what, text)) = options(&e, quick).into_iter().find(|o| o.0 == c) else { return };
        match run::copy(&text) {
            Ok(()) => self.say(format!("copied the {what}"), false),
            Err(msg) => self.say(msg, true),
        }
    }

    // ── the form ──

    fn form_key(&mut self, k: KeyEvent) {
        let outputs = self.outputs.clone();
        let Screen::Form(f) = &mut self.screen else { return };
        let field = f.field();
        let count = f.fields().len();
        if f.quick {
            match k.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    let e = f.entry.clone();
                    self.keep_quick(&e);
                    self.screen = Screen::List;
                    return;
                }
                KeyCode::Char(c @ ('s' | 'w')) => {
                    let e = f.entry.clone();
                    self.keep_quick(&e);
                    self.copy_option(&e, c, true);
                    return;
                }
                // nothing to save here
                KeyCode::Enter => {
                    self.say("s, w or m copies it", false);
                    return;
                }
                _ => {}
            }
        }
        let e = &mut f.entry;
        match k.code {
            KeyCode::Esc => self.screen = Screen::List,
            KeyCode::Up | KeyCode::BackTab => {
                f.sel = f.sel.saturating_sub(1);
                f.typed = false;
            }
            KeyCode::Down | KeyCode::Tab => {
                f.sel = (f.sel + 1).min(count - 1);
                f.typed = false;
            }
            KeyCode::Enter => self.save_form(),
            // space types in the text fields, changes the rest
            KeyCode::Left | KeyCode::Right => change(e, field, k.code == KeyCode::Left, &outputs),
            KeyCode::Char(' ') if !matches!(field, Field::Name | Field::Runs) => change(e, field, false, &outputs),
            KeyCode::Backspace => match field {
                Field::Name => {
                    e.name.pop();
                }
                Field::Runs => {
                    if let Runs::Command(c) = &mut e.runs {
                        c.pop();
                    }
                }
                _ => {
                    if let Some(n) = number(e, field) {
                        *n /= 10;
                    }
                }
            },
            KeyCode::Char(c) => match field {
                Field::Name => e.name.push(c),
                Field::Runs => {
                    if let Runs::Command(cmd) = &mut e.runs {
                        cmd.push(c);
                    }
                }
                _ => {
                    if let (Some(d), Some(n)) = (c.to_digit(10), number(e, field)) {
                        *n = if f.typed { (*n * 10 + d).min(100_000) } else { d };
                        f.typed = true;
                    }
                }
            },
            _ => {}
        }
    }

    fn save_form(&mut self) {
        let Screen::Form(f) = &self.screen else { return };
        let mut e = f.entry.clone();
        e.name = e.name.trim().to_string();
        e.render = e.render.clamp(25, 100);
        e.sharpness = e.sharpness.min(20);
        let was = f.was.clone();
        if e.name.trim().is_empty() {
            self.say("give it a name", true);
            return;
        }
        if matches!(&e.runs, Runs::Command(c) if c.trim().is_empty()) {
            self.say("what should it run? type a command", true);
            return;
        }
        if e.width == 0 || e.height == 0 {
            self.say("the size can't be 0", true);
            return;
        }
        let taken = self.entries.iter().any(|x| x.slug() == e.slug());
        if taken && was.as_deref() != Some(&e.slug()) {
            self.say("there's already an entry with that name", true);
            return;
        }
        if let Err(err) = entry::save(&self.dir, &e) {
            self.say(format!("couldn't save: {err}"), true);
            return;
        }
        // renamed: the old file goes, and a permanent copy follows the new name
        if let Some(old) = &was
            && *old != e.slug()
        {
            let _ = std::fs::remove_file(self.dir.join(format!("{old}.conf")));
            if let Runs::App(id) = &e.runs
                && apps::pinned_by(id).as_deref() == Some(old.as_str())
                && let Some(app) = apps::find(&apps::dirs(), id)
            {
                let _ = apps::remove_copy(id);
                let _ = apps::install_copy(&app, &format!("nxgs run {}", e.slug()), &e.slug());
            }
        }
        self.screen = Screen::List;
        self.reload(Some(&e.slug()));
        self.say(format!("saved {}", e.name), false);
    }

    // ── drawing ──

    fn draw(&mut self, f: &mut ratatui::Frame) {
        let [main, hints] = Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(f.area());
        match &self.screen {
            Screen::List => self.draw_list(f, main),
            Screen::Pick { .. } => self.draw_pick(f, main),
            Screen::Form(_) => self.draw_form(f, main),
        }
        self.draw_hints(f, hints);
        match self.popup {
            Some(Popup::Help) => self.draw_help(f),
            Some(Popup::Copy) => self.draw_copy(f),
            Some(Popup::Add) => self.draw_menu(
                f,
                " add ",
                &[
                    ("s", "a Steam game"),
                    ("a", "a desktop app"),
                    ("c", "a command"),
                ],
            ),
            Some(Popup::Delete) => {
                let name = self.selected().map(|e| e.name.clone()).unwrap_or_default();
                self.draw_menu(f, " delete ", &[("y", &format!("delete {name}")), ("any", "keep it")]);
            }
            None => {}
        }
    }

    fn block(&self, title: &str) -> Block<'static> {
        Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(self.pal.surface))
            .title_top(Line::from(Span::styled(
                format!(" {title} "),
                Style::new().fg(self.pal.accent).add_modifier(Modifier::BOLD),
            )))
    }

    fn draw_list(&self, f: &mut ratatui::Frame, area: Rect) {
        let p = &self.pal;
        let [left, right] =
            Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)]).areas(area);
        let mut lines = Vec::new();
        if self.entries.is_empty() {
            lines.push(Line::from(Span::styled("  nothing saved yet", Style::new().fg(p.muted))));
            lines.push(Line::from(vec![
                Span::styled("  o", Style::new().fg(p.accent)),
                Span::styled(" quick copy: pick settings, copy a launch option", Style::new().fg(p.muted)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  a", Style::new().fg(p.accent)),
                Span::styled(" saves a Steam game, a desktop app or a command", Style::new().fg(p.muted)),
            ]));
        }
        let rows = left.height.saturating_sub(2) as usize / 2;
        let start = (self.sel + 1).saturating_sub(rows.max(1));
        for (i, e) in self.entries.iter().enumerate().skip(start) {
            let on = i == self.sel;
            let bg = if on { Style::new().bg(p.surface) } else { Style::new() };
            let mark = if on { "› " } else { "  " };
            let permanent = matches!(&e.runs, Runs::App(id) if apps::pinned_by(id).as_deref() == Some(e.slug().as_str()));
            let mut head = vec![
                Span::styled(mark, bg.fg(p.accent)),
                Span::styled(e.name.clone(), bg.fg(if on { p.bright } else { p.fg }).add_modifier(Modifier::BOLD)),
            ];
            if permanent {
                head.push(Span::styled("  pinned", bg.fg(p.green)));
            }
            lines.push(Line::from(head).style(bg));
            lines.push(Line::from(Span::styled(format!("    {}", e.summary()), bg.fg(p.muted))).style(bg));
        }
        f.render_widget(Paragraph::new(lines).block(self.block("nxgs")), left);
        f.render_widget(Paragraph::new(self.details()).wrap(Wrap { trim: false }).block(self.block("details")), right);
    }

    /// the selected entry: what it runs and the commands it gives
    fn details(&self) -> Vec<Line<'static>> {
        let p = &self.pal;
        let Some(e) = self.selected() else {
            return vec![
                Line::from(Span::styled(" gamescope runs a game at a set size in its own window,", Style::new().fg(p.fg))),
                Line::from(Span::styled(" so the monitor layout can't trip it up.", Style::new().fg(p.fg))),
                Line::from(""),
                Line::from(Span::styled(" ? explains when to use what", Style::new().fg(p.muted))),
            ];
        };
        let label = |s: &str| Span::styled(format!(" {s:<10}"), Style::new().fg(p.accent));
        let val = |s: String| Span::styled(s, Style::new().fg(p.fg));
        let runs = match &e.runs {
            Runs::App(id) => match apps::find(&apps::dirs(), id) {
                Some(a) if a.is_steam_game() => format!("{} (Steam: use the launch option)", a.name),
                Some(a) => a.command,
                None => format!("{id} (not installed)"),
            },
            Runs::Command(c) => c.clone(),
            Runs::Steam => "Steam: its Properties → Launch Options".into(),
        };
        let code = |s: String| Line::from(Span::styled(format!("   {s}"), Style::new().fg(p.bright)));
        let mut l = vec![
            Line::from(vec![label("runs"), val(runs)]),
            Line::from(vec![label("settings"), val(e.summary())]),
        ];
        let opts = options(e, false);
        let single = opts.len() == 1;
        for (c, what, text) in opts {
            let keys = if single { "c".to_string() } else { format!("c {c}") };
            l.push(Line::from(""));
            l.push(Line::from(Span::styled(format!(" {what} ({keys})"), Style::new().fg(p.muted))));
            l.push(code(text));
        }
        if matches!(e.runs, Runs::App(_)) {
            l.push(Line::from(""));
            l.push(Line::from(Span::styled(
                " p: always open it in gamescope from your launcher (p again undoes it)",
                Style::new().fg(p.muted),
            )));
        }
        l
    }

    fn draw_pick(&self, f: &mut ratatui::Frame, area: Rect) {
        let p = &self.pal;
        let Screen::Pick { apps, filter, sel } = &self.screen else { return };
        let shown = Self::shown_apps(apps, filter);
        let [list, input] = Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(area);
        let rows = list.height.saturating_sub(2) as usize;
        let start = (*sel + 1).saturating_sub(rows.max(1));
        let lines: Vec<Line> = shown
            .iter()
            .enumerate()
            .skip(start)
            .take(rows)
            .map(|(n, &i)| {
                let a = &apps[i];
                let on = n == *sel;
                let bg = if on { Style::new().bg(p.surface) } else { Style::new() };
                let mut spans = vec![
                    Span::styled(if on { "› " } else { "  " }, bg.fg(p.accent)),
                    Span::styled(a.name.clone(), bg.fg(if on { p.bright } else { p.fg })),
                ];
                if a.is_steam_game() {
                    spans.push(Span::styled("  steam", bg.fg(p.blue)));
                }
                spans.push(Span::styled(format!("  {}", a.id), bg.fg(p.muted)));
                Line::from(spans).style(bg)
            })
            .collect();
        f.render_widget(Paragraph::new(lines).block(self.block("add a desktop app")), list);
        let prompt = Line::from(vec![
            Span::styled(" ➜ ", Style::new().fg(p.accent).add_modifier(Modifier::BOLD)),
            Span::styled(filter.clone(), Style::new().fg(p.bright)),
        ]);
        let x = input.x + prompt.width() as u16;
        f.render_widget(Paragraph::new(prompt), input);
        f.render_widget(
            Paragraph::new(Span::styled(format!("{}/{} ", shown.len(), apps.len()), Style::new().fg(p.muted)))
                .right_aligned(),
            input,
        );
        f.set_cursor_position(Position::new(x.min(input.right().saturating_sub(1)), input.y));
    }

    fn draw_form(&self, f: &mut ratatui::Frame, area: Rect) {
        let p = &self.pal;
        let Screen::Form(form) = &self.screen else { return };
        let e = &form.entry;
        let yn = |b: bool| if b { "on".to_string() } else { "off".to_string() };
        let limit = |n: u32| if n == 0 { "none".to_string() } else { format!("{n}") };
        let mut lines = Vec::new();
        for (i, field) in form.fields().iter().enumerate() {
            let on = i == form.sel;
            let (name, value, help) = match field {
                Field::Name => ("name", e.name.clone(), "type"),
                Field::Runs => match &e.runs {
                    Runs::App(id) => ("runs", id.clone(), "the desktop app"),
                    Runs::Command(c) => ("command", c.clone(), "type the command"),
                    Runs::Steam => ("for", "Steam".into(), "c copies its launch option once saved"),
                },
                Field::Monitor => (
                    "monitor",
                    if e.monitor.is_empty() {
                        "wherever the focus is".into()
                    } else {
                        self.outputs
                            .iter()
                            .find(|o| o.name == e.monitor)
                            .map_or(format!("{} (not connected)", e.monitor), Output::label)
                    },
                    "←→ picks; its size and refresh rate fill in",
                ),
                Field::Width => ("width", e.width.to_string(), "type"),
                Field::Height => ("height", e.height.to_string(), "type"),
                Field::Refresh => ("refresh", format!("{} Hz", e.refresh), "type or ←→ (0: gamescope's default)"),
                Field::Window => ("window", e.window.word().into(), "←→"),
                Field::Grab => ("mouse grab", yn(e.grab), "keeps the mouse in the window"),
                Field::Vrr => ("vrr", yn(e.vrr), "variable refresh, if the monitor has it"),
                Field::Fps => ("fps limit", limit(e.fps), "type or ←→"),
                Field::UnfocusedFps => ("unfocused", limit(e.unfocused_fps), "fps limit when not focused"),
                Field::Fsr => ("fsr", yn(e.fsr), "render smaller and upscale (laptops)"),
                Field::Render => ("render at", format!("{}%", e.render), "with fsr: of the size"),
                Field::Sharpness => ("sharpness", e.sharpness.to_string(), "with fsr: 0 sharpest, 20 softest"),
                Field::Gamemode => ("gamemode", yn(e.gamemode), "performance profile while it runs"),
            };
            let bg = if on { Style::new().bg(p.surface) } else { Style::new() };
            let vc = if on { p.bright } else { p.fg };
            lines.push(
                Line::from(vec![
                    Span::styled(if on { " › " } else { "   " }, bg.fg(p.accent)),
                    Span::styled(format!("{name:<11}"), bg.fg(p.accent)),
                    Span::styled(format!("{value:<34}"), bg.fg(vc)),
                    Span::styled(format!(" {help}"), bg.fg(p.muted)),
                ])
                .style(bg),
            );
        }
        lines.push(Line::from(""));
        if form.quick {
            for (k, what, text) in options(e, true) {
                lines.push(Line::from(vec![
                    Span::styled(format!("   {k} "), Style::new().fg(p.accent)),
                    Span::styled(what, Style::new().fg(p.muted)),
                ]));
                lines.push(Line::from(Span::styled(format!("     {text}"), Style::new().fg(p.bright))));
            }
        } else {
            lines.push(Line::from(Span::styled(
                format!("   gamescope {}", e.gamescope_args().join(" ")),
                Style::new().fg(p.muted),
            )));
        }
        let title = if form.quick {
            "quick copy (not saved)"
        } else if form.was.is_some() {
            "edit"
        } else {
            "new entry"
        };
        f.render_widget(Paragraph::new(lines).block(self.block(title)), area);
    }

    fn draw_hints(&self, f: &mut ratatui::Frame, area: Rect) {
        let p = &self.pal;
        if let Some((msg, err)) = &self.message {
            let c = if *err { p.red } else { p.green };
            f.render_widget(Paragraph::new(Span::styled(format!(" {msg}"), Style::new().fg(c))), area);
            return;
        }
        let keys: Vec<(&str, &str)> = match self.screen {
            Screen::List => match self.selected() {
                None => vec![("o", "quick copy"), ("a", "add"), ("?", "help"), ("esc", "quit")],
                Some(e) => {
                    // enter and p only where nxgs starts it itself
                    let starts = matches!(e.runs, Runs::App(_) | Runs::Command(_));
                    let mut k = Vec::new();
                    if starts {
                        k.push(("enter", "launch"));
                    }
                    k.extend([("o", "quick copy"), ("a", "add"), ("e", "edit"), ("c", "copy")]);
                    if matches!(e.runs, Runs::App(_)) {
                        k.push(("p", "pin"));
                    }
                    k.extend([("d", "delete"), ("?", "help"), ("esc", "quit")]);
                    k
                }
            },
            Screen::Pick { .. } => vec![("type", "filter"), ("enter", "pick"), ("esc", "back")],
            Screen::Form(Form { quick: true, .. }) => {
                vec![("↑↓", "move"), ("←→", "change"), ("s w", "copy"), ("esc", "back")]
            }
            Screen::Form(_) => vec![("↑↓", "move"), ("←→ space", "change"), ("enter", "save"), ("esc", "cancel")],
        };
        let mut spans = vec![Span::raw(" ")];
        for (i, (k, what)) in keys.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(" · ", Style::new().fg(p.surface)));
            }
            spans.push(Span::styled(*k, Style::new().fg(p.accent)));
            spans.push(Span::styled(format!(" {what}"), Style::new().fg(p.muted)));
        }
        f.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    fn popup_area(f: &ratatui::Frame, w: u16, h: u16) -> Rect {
        let a = f.area();
        let w = w.min(a.width.saturating_sub(2));
        let h = h.min(a.height.saturating_sub(2));
        Rect::new(a.x + (a.width - w) / 2, a.y + (a.height - h) / 2, w, h)
    }

    fn draw_popup(&self, f: &mut ratatui::Frame, title: &str, lines: Vec<Line<'static>>, w: u16) {
        let area = Self::popup_area(f, w, lines.len() as u16 + 2);
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(self.pal.accent))
            .title_top(Line::from(Span::styled(
                title.to_string(),
                Style::new().fg(self.pal.accent).add_modifier(Modifier::BOLD),
            )));
        f.render_widget(Clear, area);
        f.render_widget(Paragraph::new(lines).block(block), area);
    }

    fn draw_menu(&self, f: &mut ratatui::Frame, title: &str, items: &[(&str, &str)]) {
        let p = &self.pal;
        let lines = items
            .iter()
            .map(|(k, what)| {
                Line::from(vec![
                    Span::styled(format!("  {k:<5}"), Style::new().fg(p.accent)),
                    Span::styled(what.to_string(), Style::new().fg(p.fg)),
                ])
            })
            .collect();
        self.draw_popup(f, title, lines, 44);
    }

    fn draw_copy(&self, f: &mut ratatui::Frame) {
        let Some(e) = self.selected() else { return };
        let opts = options(e, false);
        let keys: Vec<String> = opts.iter().map(|o| o.0.to_string()).collect();
        let items: Vec<(&str, &str)> = keys.iter().zip(&opts).map(|(k, o)| (k.as_str(), o.1)).collect();
        self.draw_menu(f, " copy ", &items);
    }

    fn draw_help(&self, f: &mut ratatui::Frame) {
        let p = &self.pal;
        let head = |s: &str| Line::from(Span::styled(s.to_string(), Style::new().fg(p.accent).add_modifier(Modifier::BOLD)));
        let text = |s: &str| Line::from(Span::styled(format!("  {s}"), Style::new().fg(p.fg)));
        let key = |k: &str, s: &str| {
            Line::from(vec![
                Span::styled(format!("  {k:<9}"), Style::new().fg(p.accent)),
                Span::styled(s.to_string(), Style::new().fg(p.fg)),
            ])
        };
        let lines = vec![
            head("a game opens at the wrong size? try, in order:"),
            text("1. GE-Proton: set it as the game's compatibility tool in Steam"),
            text("   (install it with protonup-qt). Often fixes it on its own."),
            text("2. Proton Wayland (c w): with GE-Proton, the game as a Wayland"),
            text("   window on the chosen monitor. Experimental; windowed mode"),
            text("   gets a Wine border, so play fullscreen."),
            text("3. gamescope (c s, or launch from here): works for most games,"),
            text("   at a small cost."),
            text("Minecraft (Prism) runs badly in gamescope: set its size in the"),
            text("   instance's Settings → Minecraft → Window size instead."),
            Line::from(""),
            head("keys"),
            key("enter", "launch the entry (nxgs closes, it keeps running)"),
            key("o", "quick copy: settings to a launch option, nothing saved"),
            key("a", "add a Steam game, a desktop app or a command"),
            key("e", "edit its settings"),
            key("c", "copy its launch option or run command"),
            key("p", "desktop apps: always open in gamescope from your launcher"),
            key("d", "delete it (and undo p)"),
            key("esc", "quit"),
            Line::from(""),
            Line::from(Span::styled("  any key closes this", Style::new().fg(p.muted))),
        ];
        self.draw_popup(f, " help ", lines, 72);
    }
}

/// ←→ (or space) on a field: the next choice, on/off, or a step
fn change(e: &mut Entry, field: Field, back: bool, outputs: &[Output]) {
    let by: i64 = if back { -1 } else { 1 };
    let step = |x: &mut u32, by: i64, lo: u32, hi: u32| {
        *x = (*x as i64 + by).clamp(lo as i64, hi as i64) as u32;
    };
    match field {
        Field::Monitor => {
            // "" (wherever the focus is), then each monitor
            let names: Vec<&str> = std::iter::once("").chain(outputs.iter().map(|o| o.name.as_str())).collect();
            let at = names.iter().position(|n| *n == e.monitor).unwrap_or(0) as i64;
            let next = (at + by).rem_euclid(names.len() as i64) as usize;
            e.monitor = names[next].to_string();
            if let Some(o) = outputs.iter().find(|o| o.name == e.monitor) {
                e.width = o.width;
                e.height = o.height;
                e.refresh = o.refresh;
            }
        }
        Field::Window => {
            let at = Window::ALL.iter().position(|w| *w == e.window).unwrap_or(0) as i64;
            e.window = Window::ALL[(at + by).rem_euclid(3) as usize];
        }
        Field::Grab => e.grab = !e.grab,
        Field::Vrr => e.vrr = !e.vrr,
        Field::Fsr => e.fsr = !e.fsr,
        Field::Gamemode => e.gamemode = !e.gamemode,
        Field::Render => step(&mut e.render, by * 5, 25, 100),
        Field::Sharpness => step(&mut e.sharpness, by, 0, 20),
        Field::Refresh => step(&mut e.refresh, by, 0, 1000),
        Field::Fps => step(&mut e.fps, by * 5, 0, 1000),
        Field::UnfocusedFps => step(&mut e.unfocused_fps, by * 5, 0, 1000),
        _ => {}
    }
}

/// what an entry can copy: (key, what it is, the text). Quick copy has the
/// Steam options; a saved entry only the ones that fit it.
fn options(e: &Entry, quick: bool) -> Vec<(char, &'static str, String)> {
    let steam = || {
        vec![
            ('s', "Steam launch option (gamescope)", e.steam_option()),
            ('w', "Steam launch option, GE-Proton Wayland (no gamescope)", e.proton_wayland_option()),
        ]
    };
    if quick {
        return steam();
    }
    match &e.runs {
        Runs::Steam => steam(),
        // saved before Steam games got their own kind
        Runs::App(id) if apps::find(&apps::dirs(), id).is_some_and(|a| a.is_steam_game()) => steam(),
        _ => vec![('r', "nxgs run command", format!("nxgs run {}", e.slug()))],
    }
}

/// the number a field edits, if it's a number
fn number(e: &mut Entry, field: Field) -> Option<&mut u32> {
    Some(match field {
        Field::Width => &mut e.width,
        Field::Height => &mut e.height,
        Field::Refresh => &mut e.refresh,
        Field::Fps => &mut e.fps,
        Field::UnfocusedFps => &mut e.unfocused_fps,
        Field::Render => &mut e.render,
        Field::Sharpness => &mut e.sharpness,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    fn screen(app: &mut App, w: u16, h: u16) -> String {
        let mut t = ratatui::Terminal::new(TestBackend::new(w, h)).unwrap();
        t.draw(|f| app.draw(f)).unwrap();
        let buf = t.backend().buffer().clone();
        (0..h)
            .map(|y| (0..w).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>().trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn key(app: &mut App, c: KeyCode) {
        app.key(KeyEvent::new(c, KeyModifiers::NONE));
    }

    fn typed(app: &mut App, s: &str) {
        for c in s.chars() {
            key(app, KeyCode::Char(c));
        }
    }

    fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("nxgs-ui-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn dp1() -> Vec<Output> {
        vec![
            Output { name: "DP-1".into(), width: 1920, height: 1080, refresh: 144, focused: true },
            Output { name: "HDMI-A-2".into(), width: 2560, height: 1440, refresh: 60, focused: false },
        ]
    }

    #[test]
    fn empty_list_says_how_to_add() {
        let mut app = App::new(temp("empty"), Palette::default(), dp1());
        let s = screen(&mut app, 110, 20);
        assert!(s.contains("nothing saved yet"), "{s}");
        assert!(s.contains("a add"), "{s}");
    }

    #[test]
    fn adds_a_command_entry_with_the_focused_monitor() {
        let dir = temp("add");
        let mut app = App::new(dir.clone(), Palette::default(), dp1());
        key(&mut app, KeyCode::Char('a'));
        key(&mut app, KeyCode::Char('c'));
        typed(&mut app, "Glx Gears");
        key(&mut app, KeyCode::Down);
        typed(&mut app, "glxgears");
        // monitor: on to HDMI-A-2, which fills its size
        key(&mut app, KeyCode::Down);
        key(&mut app, KeyCode::Right);
        let s = screen(&mut app, 120, 24);
        assert!(s.contains("HDMI-A-2 · 2560×1440 · 60Hz"), "{s}");
        assert!(s.contains("gamescope -W 2560 -H 1440 -r 60 -f"), "{s}");
        key(&mut app, KeyCode::Enter);
        let e = entry::find(&dir, "glx gears").unwrap();
        assert_eq!(e.runs, Runs::Command("glxgears".into()));
        assert_eq!((e.width, e.monitor.as_str()), (2560, "HDMI-A-2"));
        let s = screen(&mut app, 120, 24);
        assert!(s.contains("Glx Gears"), "{s}");
        assert!(s.contains("nxgs run glx-gears"), "{s}");
        assert!(!s.contains("PROTON_ENABLE_WAYLAND"), "{s}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn adds_a_steam_game_without_a_command() {
        let dir = temp("steam");
        let mut app = App::new(dir.clone(), Palette::default(), dp1());
        key(&mut app, KeyCode::Char('a'));
        key(&mut app, KeyCode::Char('s'));
        typed(&mut app, "The Sims 4");
        key(&mut app, KeyCode::Enter);
        let e = entry::find(&dir, "the sims 4").unwrap();
        assert_eq!(e.runs, Runs::Steam);
        assert_eq!(Entry::from_text(&e.to_text()).unwrap(), e);
        let s = screen(&mut app, 120, 24);
        assert!(s.contains("gamescope -W 1920 -H 1080 -r 144 -f -- %command%"), "{s}");
        assert!(!s.contains("nxgs run the-sims-4"), "{s}");
        // enter can't start it: says to use the launch option
        key(&mut app, KeyCode::Enter);
        assert!(!app.quit);
        assert!(app.message.as_ref().unwrap().0.contains("launch option"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn quick_copy_saves_nothing_and_remembers() {
        let dir = temp("quick");
        let mut app = App::new(dir.clone(), Palette::default(), dp1());
        app.quick_file = dir.with_extension("quick");
        key(&mut app, KeyCode::Char('o'));
        let s = screen(&mut app, 120, 30);
        assert!(s.contains("quick copy (not saved)"), "{s}");
        assert!(!s.contains("type the command"), "{s}");
        assert!(!s.contains("sharpness"), "{s}");
        assert!(s.contains("gamescope -W 1920 -H 1080 -r 144 -f --"), "{s}");
        // width is the second field: 1280
        key(&mut app, KeyCode::Down);
        typed(&mut app, "1280");
        let s = screen(&mut app, 120, 30);
        assert!(s.contains("gamescope -W 1280 -H 1080"), "{s}");
        key(&mut app, KeyCode::Enter);
        assert!(matches!(app.screen, Screen::Form(_)));
        key(&mut app, KeyCode::Esc);
        assert!(entry::load_all(&dir).is_empty());
        key(&mut app, KeyCode::Char('o'));
        let Screen::Form(f) = &app.screen else { panic!() };
        assert_eq!(f.entry.width, 1280);
        let _ = std::fs::remove_file(&app.quick_file);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fsr_settings_show_with_fsr() {
        let dir = temp("fsr");
        let mut app = App::new(dir.clone(), Palette::default(), dp1());
        app.quick_file = dir.with_extension("quick");
        key(&mut app, KeyCode::Char('o'));
        let Screen::Form(f) = &mut app.screen else { panic!() };
        let n = f.fields().len();
        f.entry.fsr = true;
        assert_eq!(f.fields().len(), n + 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn form_checks_before_saving() {
        let dir = temp("check");
        let mut app = App::new(dir.clone(), Palette::default(), dp1());
        key(&mut app, KeyCode::Char('a'));
        key(&mut app, KeyCode::Char('c'));
        key(&mut app, KeyCode::Enter);
        assert_eq!(app.message.as_ref().unwrap().0, "give it a name");
        typed(&mut app, "x");
        key(&mut app, KeyCode::Enter);
        assert!(app.message.as_ref().unwrap().0.contains("type a command"));
        assert!(entry::load_all(&dir).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn edits_numbers_and_toggles() {
        let dir = temp("edit");
        let mut e = Entry::new("Game", Runs::Command("game".into()));
        e.refresh = 60;
        entry::save(&dir, &e).unwrap();
        let mut app = App::new(dir.clone(), Palette::default(), dp1());
        key(&mut app, KeyCode::Char('e'));
        // to width, retype it
        for _ in 0..3 {
            key(&mut app, KeyCode::Down);
        }
        // the first digit replaces the number
        typed(&mut app, "1280");
        // down to fsr (field 11), on; render up 5
        for _ in 0..8 {
            key(&mut app, KeyCode::Down);
        }
        key(&mut app, KeyCode::Char(' '));
        key(&mut app, KeyCode::Down);
        key(&mut app, KeyCode::Right);
        key(&mut app, KeyCode::Enter);
        let got = entry::find(&dir, "game").unwrap();
        assert_eq!((got.width, got.fsr, got.render), (1280, true, 80));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn help_and_copy_popups() {
        let dir = temp("pop");
        entry::save(&dir, &Entry::new("Game", Runs::Command("game".into()))).unwrap();
        let mut app = App::new(dir.clone(), Palette::default(), dp1());
        key(&mut app, KeyCode::Char('?'));
        let s = screen(&mut app, 110, 30);
        assert!(s.contains("GE-Proton"), "{s}");
        key(&mut app, KeyCode::Char('x'));
        assert!(app.popup.is_none());
        // a command entry has only its run command: c copies it, no popup
        key(&mut app, KeyCode::Char('c'));
        assert!(app.popup.is_none());
        let s = screen(&mut app, 110, 30);
        assert!(!s.contains("Prism"), "{s}");
        assert!(s.contains("nxgs run game"), "{s}");
        entry::save(&dir, &Entry::new("Sims", Runs::Steam)).unwrap();
        app.reload(Some("sims"));
        let s = screen(&mut app, 110, 30);
        assert!(!s.contains("enter launch") && !s.contains("p pin"), "{s}");
        key(&mut app, KeyCode::Char('c'));
        let s = screen(&mut app, 110, 30);
        assert!(s.contains("GE-Proton Wayland") && !s.contains("nxgs run"), "{s}");
        key(&mut app, KeyCode::Esc);
        assert!(app.popup.is_none() && !app.quit);
        // delete asks first
        key(&mut app, KeyCode::Char('d'));
        key(&mut app, KeyCode::Char('n'));
        assert_eq!(entry::load_all(&dir).len(), 2);
        key(&mut app, KeyCode::Char('d'));
        key(&mut app, KeyCode::Char('y'));
        assert_eq!(entry::load_all(&dir).len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn picker_filters_apps() {
        let mut app = App::new(temp("pick"), Palette::default(), dp1());
        let mk = |id: &str, name: &str, exec: &str| apps::App {
            id: id.into(),
            name: name.into(),
            command: exec.into(),
            path: PathBuf::from("/x"),
            ours: false,
        };
        app.screen = Screen::Pick {
            apps: vec![
                mk("brave.desktop", "Brave", "brave"),
                mk("sims.desktop", "The Sims 4", "steam steam://rungameid/1222670"),
                mk("sober.desktop", "Sober", "flatpak run org.vinegarhq.Sober"),
            ],
            filter: String::new(),
            sel: 0,
        };
        typed(&mut app, "sob");
        let s = screen(&mut app, 100, 12);
        assert!(s.contains("Sober") && !s.contains("Brave"), "{s}");
        key(&mut app, KeyCode::Enter);
        let Screen::Form(f) = &app.screen else { panic!("no form") };
        assert_eq!(f.entry.runs, Runs::App("sober.desktop".into()));
        assert_eq!(f.entry.monitor, "DP-1");
        // a Steam game shows as such
        app.screen = Screen::Pick { apps: vec![mk("sims.desktop", "The Sims 4", "steam steam://rungameid/1")], filter: String::new(), sel: 0 };
        let s = screen(&mut app, 100, 12);
        assert!(s.contains("The Sims 4  steam"), "{s}");
    }
}
