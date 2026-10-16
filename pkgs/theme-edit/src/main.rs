//! theme-edit: pick the desktop's colours with a live preview.
//!
//! Usage: theme-edit [theme.nix] [colors.json]
//!   theme.nix    the palettes and their defaults (read only),
//!                default /etc/nixos/theme.nix
//!   colors.json  your colours, default ~/.config/theme/colors.json
//!
//! Saving writes colors.json: the variant (dark or light) and the colours
//! that differ from theme.nix's defaults. nxrebuild applies it.

mod preview;
mod theme;

use eframe::egui::{self, Color32, RichText};
use std::path::{Path, PathBuf};
use std::process::Command;
use theme::{Rgb, Theme, VARIANTS};

const THEME_NIX: &str = "/etc/nixos/theme.nix";

pub fn col(c: Rgb) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

fn colors_json() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config")
        });
    base.join("theme/colors.json")
}

struct App {
    theme_nix: PathBuf,
    json: PathBuf,
    /// colors.json as last read or saved ("" if there was none)
    json_text: String,
    theme: Theme,
    /// hex boxes, in entry order, for the variant shown
    hex: Vec<String>,
    status: (String, bool), // message, is error
    look: Look,
    /// closing with unsaved changes was asked once already
    close_warned: bool,
    /// colours picked from the contrast list: outlined for a moment, and
    /// the list scrolls to the first one
    find: Vec<String>,
    find_since: f64,
    find_scrolled: bool,
}

/// colours for the editor itself: the saved colours of the variant shown,
/// so a half-picked colour can't make the controls unreadable
#[derive(Clone, Copy, Default)]
struct Look {
    light: bool,
    bg: Rgb,
    bg_alt: Rgb,
    surface: Rgb,
    fg: Rgb,
    muted: Rgb,
    accent: Rgb,
    red: Rgb,
    green: Rgb,
}

/// theme.nix with colors.json applied; the json's text, and a warning
fn load(theme_nix: &Path, json: &Path) -> Result<(Theme, String, String), String> {
    let text = std::fs::read_to_string(theme_nix)
        .map_err(|e| format!("can't read {}: {e}", theme_nix.display()))?;
    let mut theme = theme::parse(&text).map_err(|e| format!("{}: {e}", theme_nix.display()))?;
    let json_text = match std::fs::read_to_string(json) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(format!("can't read {}: {e}", json.display())),
    };
    let mut warning = String::new();
    if !json_text.is_empty() {
        match theme::parse_local(&json_text) {
            Ok(local) => {
                let unknown = theme.load_local(&local);
                if !unknown.is_empty() {
                    warning = format!(
                        "not in theme.nix, dropped when saving: {}",
                        unknown.join(", ")
                    );
                }
            }
            // show the defaults; saving replaces the broken file
            Err(e) => warning = format!("colors.json not used ({e}); saving replaces it"),
        }
    }
    Ok((theme, json_text, warning))
}

impl App {
    fn refresh(&mut self) {
        let v = self.theme.variant;
        self.hex = self
            .theme
            .entries()
            .map(|e| theme::hex(e.color[v]))
            .collect();
        let saved = |n: &str| {
            self.theme
                .entries()
                .find(|e| e.name == n)
                .map(|e| e.saved[v])
                .unwrap_or([128; 3])
        };
        self.look = Look {
            light: v == 1,
            bg: saved("bg"),
            bg_alt: saved("bgAlt"),
            surface: saved("surface"),
            fg: saved("fg"),
            muted: saved("muted"),
            accent: saved("accent"),
            red: saved("red"),
            green: saved("green"),
        };
    }

    fn reload(&mut self) {
        match load(&self.theme_nix, &self.json) {
            Ok((theme, json_text, warning)) => {
                self.theme = theme;
                self.json_text = json_text;
                self.refresh();
                self.status = if warning.is_empty() {
                    ("reloaded".into(), false)
                } else {
                    (warning, true)
                };
            }
            Err(e) => self.status = (e, true),
        }
    }

    fn save(&mut self) -> bool {
        // don't overwrite a change made somewhere else since it was read
        let now = match std::fs::read_to_string(&self.json) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => {
                self.status = (format!("can't read {}: {e}", self.json.display()), true);
                return false;
            }
        };
        if now != self.json_text {
            self.status = (
                "colors.json changed since it was opened: Reload, then make your changes again"
                    .into(),
                true,
            );
            return false;
        }
        let new = self.theme.to_json();
        let res = (|| {
            let dir = self.json.parent().ok_or("no folder")?;
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            let tmp = dir.join(".colors.json.tmp");
            std::fs::write(&tmp, &new).map_err(|e| e.to_string())?;
            std::fs::rename(&tmp, &self.json).map_err(|e| e.to_string())
        })();
        match res {
            Ok(()) => {
                self.json_text = new;
                self.theme.mark_saved();
                self.refresh();
                self.status = ("saved; rebuild to use it".into(), false);
                true
            }
            Err(e) => {
                self.status = (format!("not saved: {e}"), true);
                false
            }
        }
    }
}

/// foot in the floating popup, running nxrebuild
fn rebuild() -> Result<(), String> {
    Command::new("foot")
        .args([
            "--app-id",
            "foot-float",
            "zsh",
            "-ic",
            "nxrebuild; echo; read -sk '?press a key to close'",
        ])
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("can't start foot: {e}"))
}

fn style(ctx: &egui::Context, l: Look) {
    let mut v = if l.light {
        egui::Visuals::light()
    } else {
        egui::Visuals::dark()
    };
    v.panel_fill = col(l.bg);
    v.window_fill = col(l.bg_alt);
    v.extreme_bg_color = col(l.bg_alt);
    v.faint_bg_color = col(l.bg_alt);
    v.override_text_color = Some(col(l.fg));
    v.hyperlink_color = col(l.accent);
    v.selection.bg_fill = col(theme::mix(l.accent, l.bg, 0.5));
    v.selection.stroke.color = col(l.fg);
    v.window_stroke.color = col(l.surface);
    for w in [&mut v.widgets.inactive, &mut v.widgets.noninteractive] {
        w.bg_stroke.color = col(l.surface);
        w.fg_stroke.color = col(l.fg);
    }
    v.widgets.inactive.weak_bg_fill = col(l.bg_alt);
    v.widgets.inactive.bg_fill = col(l.bg_alt);
    v.widgets.hovered.weak_bg_fill = col(l.surface);
    v.widgets.hovered.bg_fill = col(l.surface);
    v.widgets.hovered.bg_stroke.color = col(l.accent);
    v.widgets.active.weak_bg_fill = col(l.surface);
    v.widgets.active.bg_stroke.color = col(l.accent);
    v.widgets.noninteractive.bg_fill = col(l.bg);
    ctx.set_visuals(v);
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        style(&ctx, self.look);
        let l = self.look;
        let changed = self.theme.changed();

        if ctx.input(|i| i.viewport().close_requested()) && changed > 0 && !self.close_warned {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.close_warned = true;
            self.status = (
                "not saved: close again to throw the changes away".into(),
                true,
            );
        }

        egui::Panel::top("buttons").show(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                for (i, name) in VARIANTS.iter().enumerate() {
                    let r = ui
                        .selectable_label(self.theme.variant == i, *name)
                        .on_hover_text(format!(
                            "use the {name} colours, and the {name} GTK, icon and Qt themes"
                        ));
                    if r.clicked() && self.theme.variant != i {
                        self.theme.variant = i;
                        self.find.clear();
                        self.refresh();
                    }
                }
                ui.separator();
                let save = ui.add_enabled(changed > 0, egui::Button::new("Save"));
                let save_rebuild =
                    ui.add_enabled(changed > 0, egui::Button::new("Save and rebuild"));
                let revert = ui.add_enabled(changed > 0, egui::Button::new("Undo all"));
                let reload = ui
                    .button("Reload")
                    .on_hover_text("read the files again; unsaved changes are dropped");
                if save.clicked() {
                    self.save();
                }
                if save_rebuild.clicked()
                    && self.save()
                    && let Err(e) = rebuild()
                {
                    self.status = (e, true);
                }
                if revert.clicked() {
                    for e in self.theme.entries_mut() {
                        e.color = e.saved;
                    }
                    self.theme.variant = self.theme.saved_variant;
                    self.refresh();
                    self.status = ("back to the saved colours".into(), false);
                }
                if reload.clicked() {
                    self.reload();
                }
                ui.separator();
                if changed > 0 {
                    ui.label(RichText::new(format!("{changed} unsaved")).color(col(l.accent)));
                }
                let (msg, err) = &self.status;
                if !msg.is_empty() {
                    ui.label(RichText::new(msg).color(col(if *err { l.red } else { l.muted })));
                }
            });
            ui.add_space(4.0);
        });

        egui::Panel::left("colours")
            .default_size(340.0)
            .min_size(300.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(6.0);
                    self.colour_list(ui, l);
                    ui.add_space(10.0);
                });
            });

        egui::Panel::right("contrast")
            .default_size(300.0)
            .min_size(250.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(6.0);
                    if let Some(names) =
                        preview::contrast(ui, &self.theme, l.muted, l.red, l.green, l.accent)
                    {
                        self.find = names;
                        self.find_since = ui.input(|i| i.time);
                        self.find_scrolled = false;
                    }
                });
            });

        egui::CentralPanel::default().show(ui, |ui| {
            preview::desktop(ui, &self.theme);
        });
    }
}

impl App {
    fn colour_list(&mut self, ui: &mut egui::Ui, l: Look) {
        // outline for the colours picked in the contrast list, fading out
        let age = ui.input(|i| i.time) - self.find_since;
        let fade = (1.0 - age / 3.0).clamp(0.0, 1.0) as f32;
        if fade > 0.0 && !self.find.is_empty() {
            ui.ctx().request_repaint();
        }
        let find = &self.find;
        let scrolled = &mut self.find_scrolled;
        let mut mark = |ui: &mut egui::Ui, name: &str, rect: egui::Rect| {
            if fade <= 0.0 || !find.iter().any(|n| n == name) {
                return;
            }
            let c = col(l.accent).gamma_multiply(fade);
            ui.painter().rect_stroke(
                rect.expand(3.0),
                3.0,
                egui::Stroke::new(2.0, c),
                egui::StrokeKind::Outside,
            );
            if !*scrolled {
                ui.scroll_to_rect(rect, Some(egui::Align::Center));
                *scrolled = true;
            }
        };

        let v = self.theme.variant;
        let mut i = 0;
        for s in &mut self.theme.sections {
            if !s.title.is_empty() {
                ui.add_space(8.0);
                ui.label(RichText::new(&s.title).strong().color(col(l.fg)));
            }
            for n in &s.notes {
                ui.label(RichText::new(n).size(11.5).color(col(l.muted)));
            }
            ui.add_space(2.0);
            for e in &mut s.entries {
                let buf = &mut self.hex[i];
                i += 1;
                let row = ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        let before = e.color[v];
                        egui::color_picker::color_edit_button_srgb(ui, &mut e.color[v]);
                        let r = ui.add(
                            egui::TextEdit::singleline(buf)
                                .desired_width(62.0)
                                .font(egui::TextStyle::Monospace)
                                .char_limit(7),
                        );
                        if r.changed()
                            && let Some(c) = theme::parse_hex(buf)
                        {
                            e.color[v] = c;
                        }
                        if (!r.has_focus() && e.color[v] != before) || r.lost_focus() {
                            *buf = theme::hex(e.color[v]);
                        }
                        let label = RichText::new(&e.name).monospace();
                        ui.label(if e.color[v] != e.saved[v] {
                            label.color(col(l.accent))
                        } else {
                            label
                        });
                        if e.color[v] != e.saved[v]
                            && ui
                                .small_button("↺")
                                .on_hover_text(format!("undo: back to {}", theme::hex(e.saved[v])))
                                .clicked()
                        {
                            e.color[v] = e.saved[v];
                            *buf = theme::hex(e.color[v]);
                        }
                        if e.color[v] != e.default[v]
                            && ui
                                .small_button("default")
                                .on_hover_text(format!(
                                    "theme.nix's {}: {}",
                                    VARIANTS[v],
                                    theme::hex(e.default[v])
                                ))
                                .clicked()
                        {
                            e.color[v] = e.default[v];
                            *buf = theme::hex(e.color[v]);
                        }
                    });
                    if !e.comment.is_empty() {
                        ui.label(RichText::new(&e.comment).size(11.5).color(col(l.muted)));
                    }
                });
                mark(ui, &e.name, row.response.rect);
                ui.add_space(3.0);
            }
        }

        if !self.theme.derived.is_empty() {
            ui.add_space(10.0);
            ui.label(RichText::new("worked out from the others").strong());
            ui.label(
                RichText::new("follow automatically")
                    .size(11.5)
                    .color(col(l.muted)),
            );
            ui.add_space(2.0);
            for d in &self.theme.derived {
                let c = self.theme.c(&d.name);
                let row = ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        egui::color_picker::show_color(ui, col(c), egui::vec2(40.0, 18.0));
                        ui.monospace(theme::hex(c));
                        ui.monospace(&d.name);
                    });
                    let how = format!("{} mixed {}% with {}", d.a, (d.t * 100.0).round(), d.b);
                    let note = if d.comment.is_empty() {
                        how
                    } else {
                        format!("{how}: {}", d.comment)
                    };
                    ui.label(RichText::new(note).size(11.5).color(col(l.muted)));
                });
                mark(ui, &d.name, row.response.rect);
                ui.add_space(3.0);
            }
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let first = args.next();
    if matches!(first.as_deref(), Some("-h" | "--help")) {
        println!("usage: theme-edit [theme.nix] [colors.json]");
        println!("  defaults: {THEME_NIX} and {}", colors_json().display());
        return;
    }
    let theme_nix = PathBuf::from(first.unwrap_or_else(|| THEME_NIX.into()));
    let json = args.next().map(PathBuf::from).unwrap_or_else(colors_json);
    let (theme, json_text, warning) = load(&theme_nix, &json).unwrap_or_else(|e| {
        eprintln!("theme-edit: {e}");
        std::process::exit(1)
    });
    let mut app = App {
        theme_nix,
        json,
        json_text,
        theme,
        hex: Vec::new(),
        status: (warning.clone(), !warning.is_empty()),
        look: Look::default(),
        close_warned: false,
        find: Vec::new(),
        find_since: 0.0,
        find_scrolled: false,
    };
    app.refresh();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("theme-edit")
            .with_app_id("theme-edit")
            .with_inner_size([1400.0, 800.0]),
        ..Default::default()
    };
    if let Err(e) = eframe::run_native("theme-edit", options, Box::new(move |_| Ok(Box::new(app))))
    {
        eprintln!("theme-edit: {e}");
        std::process::exit(1);
    }
}
