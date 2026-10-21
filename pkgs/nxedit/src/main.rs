//! nxedit: the config's files as a tree (or a flat list), filtered as you
//! type, with git state and a preview. Enter edits in $EDITOR, Ctrl+R
//! rebuilds, Ctrl+A stages.
//!
//! Usage: nxedit [--root DIR] [--colors FILE]
//!   DIR   the repo, default /etc/nixos
//!   FILE  the theme's colours as JSON (theme.nix's names)

mod ansi;
mod fuzzy;
mod repo;
mod rows;
mod theme;

use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Paragraph};
use repo::{File, Git, Kind, Repo};
use rows::Row;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;
use theme::Palette;

const PREVIEW_LINES: usize = 400;

struct App {
    repo: Repo,
    pal: Palette,
    folders: HashMap<String, Color>,
    filter: String,
    flat: bool,
    closed: HashSet<String>,
    rows: Vec<Row>,
    sel: usize,
    offset: usize,
    diff: bool,
    scroll: u16,
    /// (row key, diff, width) and its lines
    preview: Option<((String, bool, u16), Vec<Line<'static>>)>,
    /// a message for the hint line, and whether it's an error
    message: Option<(String, bool)>,
    /// the ? popup is open
    help: bool,
    quit: bool,
}

fn main() {
    let mut root = PathBuf::from("/etc/nixos");
    let mut colors = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match (a.as_str(), args.next()) {
            ("--root", Some(v)) => root = PathBuf::from(v),
            ("--colors", Some(v)) => colors = Some(v),
            _ => {
                eprintln!("usage: nxedit [--root DIR] [--colors FILE]");
                std::process::exit(2);
            }
        }
    }
    if !root.is_dir() {
        eprintln!("nxedit: no folder {}", root.display());
        std::process::exit(1);
    }
    let pal = colors
        .and_then(|f| std::fs::read_to_string(f).ok())
        .map(|t| Palette::from_json(&t))
        .unwrap_or_default();
    let mut app = App::new(Repo::load(&root), pal);
    let mut term = ratatui::init();
    let result = app.run(&mut term);
    ratatui::restore();
    if let Err(e) = result {
        eprintln!("nxedit: {e}");
        std::process::exit(1);
    }
}

impl App {
    fn new(repo: Repo, pal: Palette) -> App {
        let mut app = App {
            repo,
            pal,
            folders: HashMap::new(),
            filter: String::new(),
            flat: false,
            closed: HashSet::new(),
            rows: Vec::new(),
            sel: 0,
            offset: 0,
            diff: false,
            scroll: 0,
            preview: None,
            message: None,
            help: false,
            quit: false,
        };
        app.colour_folders();
        app.rebuild(None);
        app
    }

    /// each top-level folder its own colour, in order
    fn colour_folders(&mut self) {
        let mut tops: Vec<&str> = self
            .repo
            .files
            .iter()
            .filter_map(|f| f.path.split_once('/').map(|(t, _)| t))
            .collect();
        tops.sort();
        tops.dedup();
        let cs = self.pal.folder_colours();
        self.folders = tops
            .iter()
            .enumerate()
            .map(|(i, t)| (t.to_string(), cs[i % cs.len()]))
            .collect();
    }

    fn folder_colour(&self, path: &str) -> Color {
        let top = path.split('/').next().unwrap_or(path);
        self.folders.get(top).copied().unwrap_or(self.pal.fg)
    }

    /// the rows again, keeping `keep` (or the current row) selected
    fn rebuild(&mut self, keep: Option<String>) {
        let keep = keep.or_else(|| self.rows.get(self.sel).map(|r| r.key(&self.repo.files)));
        self.rows = if self.flat {
            rows::flat(&self.repo.files, &self.filter)
        } else {
            rows::tree(&self.repo.files, &self.filter, &self.closed)
        };
        self.sel = keep
            .and_then(|k| self.rows.iter().position(|r| r.key(&self.repo.files) == k))
            .unwrap_or(0)
            .min(self.rows.len().saturating_sub(1));
    }

    /// after typing: the best match is selected (ties: the shorter path);
    /// with the filter cleared, the selection stays
    fn refilter(&mut self) {
        self.rebuild(None);
        if self.filter.trim().is_empty() {
            return;
        }
        let score = |r: &Row| match r {
            Row::File { idx, .. } => {
                let path = &self.repo.files[*idx].path;
                fuzzy::score(&self.filter, path)
                    .map_or((i32::MIN, 0), |(s, _)| (s, -(path.len() as i32)))
            }
            Row::Dir { .. } => (i32::MIN, 0),
        };
        let mut best = 0;
        for (i, r) in self.rows.iter().enumerate() {
            if score(r) > score(&self.rows[best]) {
                best = i;
            }
        }
        self.sel = best;
    }

    fn reload(&mut self) {
        let keep = self.rows.get(self.sel).map(|r| r.key(&self.repo.files));
        self.repo = Repo::load(&self.repo.root.clone());
        self.colour_folders();
        self.preview = None;
        self.rebuild(keep);
    }

    fn run(&mut self, term: &mut DefaultTerminal) -> std::io::Result<()> {
        while !self.quit {
            term.draw(|f| self.draw(f))?;
            if let Event::Key(k) = event::read()?
                && k.kind == KeyEventKind::Press
            {
                self.key(k, term)?;
            }
        }
        Ok(())
    }

    fn key(&mut self, k: KeyEvent, term: &mut DefaultTerminal) -> std::io::Result<()> {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let shift = k.modifiers.contains(KeyModifiers::SHIFT);
        let before = self.sel;
        self.message = None;
        // any key closes the help
        if self.help {
            self.help = false;
            return Ok(());
        }
        let page = 10;
        match k.code {
            KeyCode::Char('c' | 'q') if ctrl => self.quit = true,
            KeyCode::Char('r') if ctrl => self.rebuild_system(term)?,
            KeyCode::Char('a') if ctrl => self.stage(term)?,
            KeyCode::Char('d') if ctrl => {
                self.diff = !self.diff;
                self.scroll = 0;
            }
            KeyCode::Char('u') if ctrl => {
                self.filter.clear();
                self.refilter();
            }
            // ? opens the help, unless it's part of a filter
            KeyCode::Char('?') if self.filter.is_empty() => self.help = true,
            KeyCode::Char(c) if !ctrl && !k.modifiers.contains(KeyModifiers::ALT) => {
                self.filter.push(c);
                self.refilter();
            }
            KeyCode::Backspace => {
                self.filter.pop();
                self.refilter();
            }
            KeyCode::Esc => {
                if self.filter.is_empty() {
                    self.quit = true;
                } else {
                    self.filter.clear();
                    self.refilter();
                }
            }
            KeyCode::Up if shift => self.scroll = self.scroll.saturating_sub(3),
            KeyCode::Down if shift => self.scroll = self.scroll.saturating_add(3),
            KeyCode::Up => self.sel = self.sel.saturating_sub(1),
            KeyCode::Down => self.sel = (self.sel + 1).min(self.rows.len().saturating_sub(1)),
            KeyCode::PageUp => self.sel = self.sel.saturating_sub(page),
            KeyCode::PageDown => {
                self.sel = (self.sel + page).min(self.rows.len().saturating_sub(1))
            }
            KeyCode::Home => self.sel = 0,
            KeyCode::End => self.sel = self.rows.len().saturating_sub(1),
            KeyCode::Tab | KeyCode::BackTab => {
                self.flat = !self.flat;
                self.rebuild(None);
            }
            KeyCode::Left => self.fold(),
            KeyCode::Right => {
                self.unfold();
            }
            KeyCode::Enter => match self.rows.get(self.sel) {
                // (folders all stay open while filtering)
                Some(Row::Dir { .. }) if self.filter.trim().is_empty() => {
                    if !self.unfold() {
                        self.fold();
                    }
                }
                Some(Row::Dir { .. }) => {}
                Some(Row::File { .. }) => self.edit(term)?,
                None => {}
            },
            _ => {}
        }
        if self.sel != before {
            self.scroll = 0;
        }
        Ok(())
    }

    /// on an open folder: close it; anywhere else: up to the folder
    fn fold(&mut self) {
        if self.flat {
            return;
        }
        let filtering = !self.filter.trim().is_empty();
        match self.rows.get(self.sel).cloned() {
            Some(Row::Dir {
                path, open: true, ..
            }) if !filtering => {
                self.closed.insert(path.clone());
                self.rebuild(Some(format!("{path}/")));
            }
            Some(row) => {
                let key = row.key(&self.repo.files);
                let parent = key
                    .trim_end_matches('/')
                    .rsplit_once('/')
                    .map(|(p, _)| format!("{p}/"));
                if let Some(i) =
                    parent.and_then(|p| self.rows.iter().position(|r| r.key(&self.repo.files) == p))
                {
                    self.sel = i;
                }
            }
            None => {}
        }
    }

    /// on a closed folder: open it (true if it did)
    fn unfold(&mut self) -> bool {
        if let Some(Row::Dir {
            path, open: false, ..
        }) = self.rows.get(self.sel).cloned()
        {
            self.closed.remove(&path);
            self.rebuild(Some(format!("{path}/")));
            return true;
        }
        false
    }

    fn selected_file(&self) -> Option<&File> {
        match self.rows.get(self.sel) {
            Some(Row::File { idx, .. }) => self.repo.files.get(*idx),
            _ => None,
        }
    }

    // ── actions that hand the terminal over ──

    fn suspended<T>(term: &mut DefaultTerminal, f: impl FnOnce() -> T) -> T {
        ratatui::restore();
        let r = f();
        *term = ratatui::init();
        r
    }

    fn edit(&mut self, term: &mut DefaultTerminal) -> std::io::Result<()> {
        let Some(file) = self.selected_file().cloned() else {
            return Ok(());
        };
        let path = self.repo.root.join(&file.path);
        let editor = std::env::var("EDITOR")
            .ok()
            .filter(|e| !e.trim().is_empty())
            .unwrap_or_else(|| "nvim".into());
        let ids = repo::own_ids();
        let result = Self::suspended(term, || {
            // written with sudo (a pull, a patch): yours again first, so it saves
            if file.foreign
                && let Some((u, g)) = ids
            {
                let _ = Command::new("sudo")
                    .arg("chown")
                    .arg(format!("{u}:{g}"))
                    .arg(&path)
                    .status();
            }
            let mut words = editor.split_whitespace();
            let prog = words.next().unwrap_or("nvim");
            Command::new(prog).args(words).arg(&path).status()
        });
        if let Err(e) = result {
            self.message = Some((format!("couldn't start {editor}: {e}"), true));
        }
        self.reload();
        Ok(())
    }

    fn rebuild_system(&mut self, term: &mut DefaultTerminal) -> std::io::Result<()> {
        let ok = Self::suspended(term, || {
            let ok = Command::new("nxrebuild")
                .status()
                .is_ok_and(|s| s.success());
            println!(
                "\n{} — press enter to go back",
                if ok { "rebuilt" } else { "rebuild failed" }
            );
            let _ = std::io::stdin().read_line(&mut String::new());
            ok
        });
        self.message = Some((
            if ok {
                "rebuilt".into()
            } else {
                "rebuild failed".into()
            },
            !ok,
        ));
        self.reload();
        Ok(())
    }

    fn stage(&mut self, term: &mut DefaultTerminal) -> std::io::Result<()> {
        let target = match self.rows.get(self.sel) {
            Some(r) => r.key(&self.repo.files),
            None => return Ok(()),
        };
        let root = self.repo.root.clone();
        let ok = Self::suspended(term, || {
            Command::new("sudo")
                .arg("git")
                .arg("-C")
                .arg(&root)
                .args(["add", "-f", "--", target.trim_end_matches('/')])
                .status()
                .is_ok_and(|s| s.success())
        });
        self.message = Some(if ok {
            (format!("staged {target}"), false)
        } else {
            (format!("couldn't stage {target}"), true)
        });
        self.reload();
        Ok(())
    }

    // ── drawing ──

    fn draw(&mut self, f: &mut ratatui::Frame) {
        let [main, input, hints] = Layout::vertical([
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(f.area());
        let [left, right] =
            Layout::horizontal([Constraint::Percentage(42), Constraint::Percentage(58)])
                .areas(main);
        self.draw_list(f, left);
        self.draw_preview(f, right);
        self.draw_input(f, input);
        self.draw_hints(f, hints);
        if self.help {
            self.draw_help(f);
        }
    }

    fn block(&self, title: Line<'static>, right: Line<'static>) -> Block<'static> {
        Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(self.pal.surface))
            .title_top(title)
            .title_top(right.right_aligned())
    }

    fn draw_list(&mut self, f: &mut ratatui::Frame, area: Rect) {
        let p = &self.pal;
        let b = &self.repo.branch;
        let mut info = vec![Span::raw(" ")];
        if !b.name.is_empty() {
            info.push(Span::styled(
                format!(" {}", b.name),
                Style::new().fg(p.magenta),
            ));
        }
        if b.ahead > 0 {
            info.push(Span::styled(
                format!(" ↑{}", b.ahead),
                Style::new().fg(p.green),
            ));
        }
        if b.behind > 0 {
            info.push(Span::styled(
                format!(" ↓{}", b.behind),
                Style::new().fg(p.red),
            ));
        }
        let (changed, staged, new) = self.repo.counts();
        for (n, mark, c) in [
            (changed, "●", p.yellow),
            (staged, "+", p.green),
            (new, "?", p.red),
        ] {
            if n > 0 {
                info.push(Span::styled(format!(" {mark}{n}"), Style::new().fg(c)));
            }
        }
        info.push(Span::raw(" "));
        let title = Line::from(vec![
            Span::raw(" "),
            Span::styled(
                self.repo.root.display().to_string(),
                Style::new().fg(p.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                if self.flat { " list " } else { " tree " },
                Style::new().fg(p.muted),
            ),
        ]);
        let block = self.block(title, Line::from(info));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let h = inner.height as usize;
        if self.sel < self.offset {
            self.offset = self.sel;
        } else if h > 0 && self.sel >= self.offset + h {
            self.offset = self.sel + 1 - h;
        }
        self.offset = self.offset.min(self.rows.len().saturating_sub(h.max(1)));
        let lines: Vec<Line> = (self.offset..self.rows.len().min(self.offset + h))
            .map(|i| self.row_line(&self.rows[i], i == self.sel, inner.width as usize))
            .collect();
        if lines.is_empty() {
            let msg = if self.filter.is_empty() {
                "no files"
            } else {
                "nothing matches"
            };
            f.render_widget(
                Paragraph::new(Span::styled(msg, Style::new().fg(p.muted))),
                inner,
            );
        } else {
            f.render_widget(Paragraph::new(lines), inner);
        }
    }

    fn kind_style(&self, k: Kind) -> (&'static str, Color) {
        let p = &self.pal;
        match k {
            Kind::Nix => ("\u{f313}", p.blue),
            Kind::Shell => ("\u{f489}", p.green),
            Kind::Rust => ("\u{e7a8}", p.orange),
            Kind::Go => ("\u{e627}", p.blue),
            Kind::Python => ("\u{e73c}", p.yellow),
            Kind::Text => ("\u{f48a}", p.magenta),
            Kind::Data => ("\u{e615}", p.muted),
            Kind::Image => ("\u{f1c5}", p.magenta),
            Kind::Audio => ("\u{f1c7}", p.magenta),
            Kind::Secret => ("\u{f084}", p.red),
            Kind::Other => ("\u{f15b}", p.fg),
        }
    }

    fn row_line(&self, row: &Row, selected: bool, width: usize) -> Line<'static> {
        let p = &self.pal;
        let mut spans: Vec<Span> = Vec::new();
        let (depth, top) = match row {
            Row::Dir { path, depth, .. } => (*depth, path.as_str()),
            Row::File { idx, depth, .. } => (*depth, self.repo.files[*idx].path.as_str()),
        };
        let fc = self.folder_colour(top);
        // the folder's guide line, in its colour
        for _ in 0..depth {
            spans.push(Span::styled(
                "│ ",
                Style::new().fg(fc).add_modifier(Modifier::DIM),
            ));
        }
        match row {
            Row::Dir { path, open, .. } => {
                let name = path.rsplit('/').next().unwrap_or(path);
                let icon = if *open { "\u{f07c}" } else { "\u{f07b}" };
                let count = self
                    .repo
                    .files
                    .iter()
                    .filter(|f| f.path.starts_with(&format!("{path}/")))
                    .count();
                spans.push(Span::styled(
                    format!("  {icon} {name}/"),
                    Style::new().fg(fc).add_modifier(Modifier::BOLD),
                ));
                if !open {
                    spans.push(Span::styled(format!(" {count}"), Style::new().fg(p.muted)));
                }
            }
            Row::File { idx, hits, .. } => {
                let file = &self.repo.files[*idx];
                let (mark, mc) = match file.git {
                    Git::Clean => (" ", p.fg),
                    Git::Changed => ("●", p.yellow),
                    Git::Staged => ("+", p.green),
                    Git::New => ("?", p.red),
                };
                spans.push(Span::styled(
                    format!("{mark} "),
                    Style::new().fg(mc).add_modifier(Modifier::BOLD),
                ));
                let (icon, kc) = self.kind_style(file.kind);
                spans.push(Span::styled(format!("{icon} "), Style::new().fg(kc)));
                // the tree shows the name, the list the whole path
                let shown_from = if self.flat {
                    0
                } else {
                    file.path.rfind('/').map_or(0, |i| i + 1)
                };
                let base = file.path.rfind('/').map_or(0, |i| i + 1);
                for (ci, ch) in file
                    .path
                    .char_indices()
                    .skip_while(|(i, _)| *i < shown_from)
                {
                    let colour = if ci < base {
                        fc
                    } else if file.kind == Kind::Nix {
                        p.fg
                    } else {
                        kc
                    };
                    let mut st = Style::new().fg(colour);
                    if hits.contains(&file.path[..ci].chars().count()) {
                        st = st
                            .fg(p.accent)
                            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
                    }
                    spans.push(Span::styled(ch.to_string(), st));
                }
                if file.foreign {
                    spans.push(Span::styled(" \u{f023}", Style::new().fg(p.red)));
                }
            }
        }
        let mut line = Line::from(spans);
        if selected {
            let pad = width.saturating_sub(line.width());
            line.spans.push(Span::raw(" ".repeat(pad)));
            for s in &mut line.spans {
                s.style = s
                    .style
                    .fg(p.on_accent)
                    .bg(p.accent)
                    .remove_modifier(Modifier::DIM);
            }
        }
        line
    }

    fn draw_preview(&mut self, f: &mut ratatui::Frame, area: Rect) {
        let p = self.pal.clone();
        let row = self.rows.get(self.sel).cloned();
        let key = row
            .as_ref()
            .map(|r| r.key(&self.repo.files))
            .unwrap_or_default();
        let file = self.selected_file().cloned();
        let showing_diff = self.diff
            && file
                .as_ref()
                .is_some_and(|f| matches!(f.git, Git::Changed | Git::Staged));
        let title = Line::from(vec![
            Span::raw(" "),
            Span::styled(
                key.clone(),
                Style::new().fg(p.fg).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]);
        let right = if showing_diff {
            Line::from(Span::styled(" diff ", Style::new().fg(p.yellow)))
        } else {
            Line::from("")
        };
        let block = self.block(title, right);
        let inner = block.inner(area);
        f.render_widget(block, area);

        let want = (key.clone(), showing_diff, inner.width);
        if self.preview.as_ref().is_none_or(|(k, _)| *k != want) {
            let lines = match (&row, &file) {
                (Some(Row::Dir { path, .. }), _) => self.folder_preview(path),
                (_, Some(file)) => self.file_preview(file, showing_diff, inner.width),
                _ => Vec::new(),
            };
            self.preview = Some((want, lines));
        }
        let lines = self
            .preview
            .as_ref()
            .map(|(_, l)| l.clone())
            .unwrap_or_default();
        let max = (lines.len() as u16).saturating_sub(1);
        self.scroll = self.scroll.min(max);
        f.render_widget(Paragraph::new(lines).scroll((self.scroll, 0)), inner);
    }

    fn folder_preview(&self, path: &str) -> Vec<Line<'static>> {
        let prefix = format!("{path}/");
        let files: Vec<&File> = self
            .repo
            .files
            .iter()
            .filter(|f| f.path.starts_with(&prefix))
            .collect();
        let mut out = vec![Line::from(Span::styled(
            format!("{} files", files.len()),
            Style::new().fg(self.pal.muted),
        ))];
        out.push(Line::from(""));
        for file in files {
            let (icon, kc) = self.kind_style(file.kind);
            out.push(Line::from(vec![
                Span::styled(format!("{icon} "), Style::new().fg(kc)),
                Span::styled(
                    file.path[prefix.len()..].to_string(),
                    Style::new().fg(self.pal.fg),
                ),
            ]));
        }
        out
    }

    fn file_preview(&self, file: &File, diff: bool, width: u16) -> Vec<Line<'static>> {
        let path = self.repo.root.join(&file.path);
        let muted = |s: String| vec![Line::from(Span::styled(s, Style::new().fg(self.pal.muted)))];
        if diff {
            let out = Command::new("git")
                .arg("-C")
                .arg(&self.repo.root)
                .args([
                    "--no-optional-locks",
                    "diff",
                    "--color=always",
                    "HEAD",
                    "--",
                ])
                .arg(&file.path)
                .output();
            if let Ok(o) = out
                && o.status.success()
                && !o.stdout.is_empty()
            {
                return ansi::lines(&String::from_utf8_lossy(&o.stdout));
            }
        }
        if repo::is_binary(&path) {
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            return muted(format!("binary file, {}", human(size)));
        }
        let bat = Command::new("bat")
            .args([
                "--color=always",
                "--style=numbers",
                "--theme=ansi",
                "--paging=never",
                "--wrap=never",
            ])
            .arg(format!("--line-range=:{PREVIEW_LINES}"))
            .arg(format!("--terminal-width={width}"))
            .arg(&path)
            .output();
        match bat {
            Ok(o) if o.status.success() => ansi::lines(&String::from_utf8_lossy(&o.stdout)),
            _ => match std::fs::read_to_string(&path) {
                Ok(t) => {
                    let t: String = t.lines().take(PREVIEW_LINES).collect::<Vec<_>>().join("\n");
                    ansi::lines(&t)
                }
                Err(e) => muted(format!("can't read it: {e}")),
            },
        }
    }

    fn draw_input(&self, f: &mut ratatui::Frame, area: Rect) {
        let p = &self.pal;
        let total = self.repo.files.len();
        let count = if self.filter.trim().is_empty() {
            format!("{total} files ")
        } else {
            let found = self
                .rows
                .iter()
                .filter(|r| matches!(r, Row::File { .. }))
                .count();
            format!("{found}/{total} ")
        };
        let line = Line::from(vec![
            Span::styled(
                " ➜ ",
                Style::new().fg(p.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(self.filter.clone(), Style::new().fg(p.bright)),
        ]);
        let cursor_x = area.x + line.width() as u16;
        f.render_widget(Paragraph::new(line), area);
        f.render_widget(
            Paragraph::new(Span::styled(count, Style::new().fg(p.muted))).right_aligned(),
            area,
        );
        f.set_cursor_position(Position::new(
            cursor_x.min(area.right().saturating_sub(1)),
            area.y,
        ));
    }

    fn draw_hints(&self, f: &mut ratatui::Frame, area: Rect) {
        let p = &self.pal;
        if let Some((msg, err)) = &self.message {
            let c = if *err { p.red } else { p.green };
            f.render_widget(
                Paragraph::new(Span::styled(format!(" {msg}"), Style::new().fg(c))),
                area,
            );
            return;
        }
        let mut spans = vec![Span::raw(" ")];
        let keys = [
            ("enter", "edit"),
            ("^r", "rebuild"),
            ("^a", "stage"),
            ("^d", "diff"),
            ("tab", "tree/list"),
            ("←→", "fold"),
            ("?", "help"),
            ("esc", "quit"),
        ];
        for (i, (k, what)) in keys.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(" · ", Style::new().fg(p.surface)));
            }
            spans.push(Span::styled(*k, Style::new().fg(p.accent)));
            spans.push(Span::styled(format!(" {what}"), Style::new().fg(p.muted)));
        }
        f.render_widget(Paragraph::new(Line::from(spans)), area);
    }
}

impl App {
    /// what the marks, colours and keys mean, over the middle of the screen
    fn draw_help(&self, f: &mut ratatui::Frame) {
        let p = &self.pal;
        let head = |s: &str| {
            Line::from(Span::styled(
                s.to_string(),
                Style::new().fg(p.accent).add_modifier(Modifier::BOLD),
            ))
        };
        let row = |k: Span<'static>, what: &str| {
            Line::from(vec![
                Span::raw("  "),
                k,
                Span::styled(format!("  {what}"), Style::new().fg(p.fg)),
            ])
        };
        let key = |s: &str| Span::styled(format!("{s:<11}"), Style::new().fg(p.accent));
        let mark = |s: &str, c: Color| {
            Span::styled(
                format!("{s:<11}"),
                Style::new().fg(c).add_modifier(Modifier::BOLD),
            )
        };
        let mut lines = vec![
            head("marks"),
            row(mark("●", p.yellow), "changed, not staged yet"),
            row(mark("+", p.green), "staged (Ctrl+A)"),
            row(
                mark("?", p.red),
                "not in git: the flake can't see it until staged",
            ),
            row(
                mark("\u{f023}", p.red),
                "not yours (root's): handed back when you edit it",
            ),
            row(
                mark("↑2 ↓1", p.green),
                "commits not pushed / not pulled yet",
            ),
            Line::from(""),
            head("colours"),
            row(key("folders"), "each top-level folder has its own colour"),
            row(key("files"), "coloured by type, each with its own icon"),
            row(key("underlined"), "the letters your filter matched"),
            Line::from(""),
            head("keys"),
            row(key("type"), "filter (Backspace, Ctrl+U clears)"),
            row(key("↑ ↓"), "move (PgUp/PgDn, Home/End)"),
            row(
                key("← →"),
                "close / open a folder; ← on a file goes to its folder",
            ),
            row(
                key("enter"),
                "edit the file in $EDITOR, or open/close the folder",
            ),
            row(key("tab"), "tree or flat list"),
            row(key("ctrl+d"), "show the diff of a changed file"),
            row(key("shift+↑ ↓"), "scroll the preview"),
            row(key("ctrl+a"), "stage the file or folder (git add)"),
            row(key("ctrl+r"), "rebuild (nxrebuild)"),
            row(key("esc"), "clear the filter, or quit"),
        ];
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  any key closes this",
            Style::new().fg(p.muted),
        )));
        let area = f.area();
        let w = 76.min(area.width.saturating_sub(2));
        let h = (lines.len() as u16 + 2).min(area.height.saturating_sub(2));
        let popup = Rect::new(
            area.x + (area.width - w) / 2,
            area.y + (area.height - h) / 2,
            w,
            h,
        );
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(p.accent))
            .title_top(Line::from(Span::styled(
                " help ",
                Style::new().fg(p.accent).add_modifier(Modifier::BOLD),
            )));
        f.render_widget(Clear, popup);
        f.render_widget(Paragraph::new(lines).block(block), popup);
    }
}

fn human(n: u64) -> String {
    match n {
        n if n >= 1 << 20 => format!("{:.1} MiB", n as f64 / (1 << 20) as f64),
        n if n >= 1 << 10 => format!("{:.1} KiB", n as f64 / 1024.0),
        n => format!("{n} bytes"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    fn repo(paths: &[(&str, Git)]) -> Repo {
        Repo {
            root: PathBuf::from("/nonexistent"),
            files: paths
                .iter()
                .map(|(p, g)| File {
                    path: p.to_string(),
                    git: *g,
                    kind: repo::kind(p),
                    foreign: false,
                })
                .collect(),
            branch: repo::Branch {
                name: "main".into(),
                ahead: 2,
                behind: 0,
            },
        }
    }

    fn screen(app: &mut App, w: u16, h: u16) -> String {
        let mut t = ratatui::Terminal::new(TestBackend::new(w, h)).unwrap();
        t.draw(|f| app.draw(f)).unwrap();
        let buf = t.backend().buffer().clone();
        (0..h)
            .map(|y| {
                (0..w)
                    .map(|x| buf[(x, y)].symbol().to_string())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn draws_tree_and_marks() {
        let mut app = App::new(
            repo(&[
                ("flake.nix", Git::Changed),
                ("desktop/sway.nix", Git::Staged),
                ("scripts/x.sh", Git::New),
            ]),
            Palette::default(),
        );
        let s = screen(&mut app, 100, 12);
        assert!(s.contains("/nonexistent"), "{s}");
        assert!(s.contains("main ↑2 ●1 +1 ?1"), "{s}");
        assert!(s.contains("● \u{f313} flake.nix"), "{s}");
        assert!(s.contains("\u{f07c} desktop/"), "{s}");
        assert!(s.contains("│ + \u{f313} sway.nix"), "{s}");
        assert!(s.contains("enter edit"), "{s}");
    }

    #[test]
    fn typing_filters_and_folding_hides() {
        let mut app = App::new(
            repo(&[
                ("flake.nix", Git::Clean),
                ("desktop/sway.nix", Git::Clean),
                ("desktop/awake.nix", Git::Clean),
            ]),
            Palette::default(),
        );
        for c in "sway".chars() {
            app.filter.push(c);
        }
        app.refilter();
        assert_eq!(app.selected_file().unwrap().path, "desktop/sway.nix");
        // the best match, not the first in the tree
        let mut app2 = App::new(
            repo(&[
                ("desktop/sway-helpers.nix", Git::Clean),
                ("desktop/sway.nix", Git::Clean),
            ]),
            Palette::default(),
        );
        app2.filter = "sway".into();
        app2.refilter();
        assert_eq!(app2.selected_file().unwrap().path, "desktop/sway.nix");
        // clearing the filter keeps the selection
        app.filter.clear();
        app.refilter();
        assert_eq!(app.selected_file().unwrap().path, "desktop/sway.nix");
        // onto the folder and close it
        app.sel = app
            .rows
            .iter()
            .position(|r| matches!(r, Row::Dir { .. }))
            .unwrap();
        app.fold();
        assert_eq!(app.rows.len(), 2);
        assert!(app.unfold());
        assert_eq!(app.rows.len(), 4);
        // from a file, left goes up to its folder
        app.sel = 3;
        app.fold();
        assert_eq!(app.rows[app.sel].key(&app.repo.files), "desktop/");
    }

    #[test]
    fn flat_list_shows_paths() {
        let mut app = App::new(
            repo(&[("flake.nix", Git::Clean), ("desktop/sway.nix", Git::Clean)]),
            Palette::default(),
        );
        app.flat = true;
        app.rebuild(None);
        let s = screen(&mut app, 80, 8);
        assert!(s.contains("desktop/sway.nix"), "{s}");
        assert!(s.contains(" list "), "{s}");
    }

    #[test]
    fn help_opens_and_closes() {
        let mut app = App::new(repo(&[("flake.nix", Git::Clean)]), Palette::default());
        app.help = true;
        let s = screen(&mut app, 100, 40);
        assert!(s.contains("not in git: the flake can't see it"), "{s}");
        assert!(s.contains("any key closes this"), "{s}");
        // ? while filtering is part of the filter, not the help
        app.help = false;
        app.filter = "a".into();
        let s = screen(&mut app, 100, 40);
        assert!(!s.contains("any key closes this"));
        assert!(s.contains("? help"), "{s}");
    }

    #[test]
    fn sizes() {
        assert_eq!(human(10), "10 bytes");
        assert_eq!(human(2048), "2.0 KiB");
    }
}
