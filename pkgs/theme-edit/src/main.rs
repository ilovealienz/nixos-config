//! theme-edit: pick the desktop's colours with a live preview.
//!
//! Usage: theme-edit [theme.nix] [colors.json]
//!   theme.nix    the palettes and their defaults (read only),
//!                default /etc/nixos/theme.nix
//!   colors.json  your colours, default ~/.config/theme/colors.json
//!
//! Saves colors.json (variant, changed colours, icons, wallpaper) for
//! nxrebuild to apply.

mod icons;
mod preview;
mod theme;
mod wallpaper;

use eframe::egui::{self, Color32, RichText};
use std::collections::HashMap;
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
    tab: Tab,
    /// installed icon themes, and each one's preview icons (looked up
    /// once; Reload looks again)
    icon_themes: icons::Themes,
    previews: HashMap<String, Vec<Option<PathBuf>>>,
    /// the wallpaper folders and their images (None: not looked at yet),
    /// the thumbnail maker, and the thumbnails made (or why not)
    walls: Option<Vec<(String, Vec<PathBuf>)>>,
    thumbs: Option<wallpaper::Thumbs>,
    thumb_tex: HashMap<PathBuf, Result<egui::TextureHandle, String>>,
    /// the folder being typed in to add
    new_folder: String,
}

/// what the left panel shows
#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Colours,
    Icons,
    Wallpaper,
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
                self.icon_themes = icons::Themes::new(icons::roots());
                self.previews.clear();
                self.walls = None;
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
        // the wallpaper first, so colors.json never names a missing one
        let copy = wallpaper::copy_path(&self.json);
        let wall_changed = self.theme.wallpaper != self.theme.saved_wallpaper;
        let mut note = String::new();
        if let Some(w) = &self.theme.wallpaper {
            let new_image = self.theme.saved_wallpaper.as_ref().map(|s| &s.from) != Some(&w.from);
            if (new_image || !copy.exists())
                && let Err(e) = wallpaper::install(Path::new(&w.from), &copy)
            {
                if new_image {
                    self.status = (format!("not saved: wallpaper: {e}"), true);
                    return false;
                }
                note = format!("; the wallpaper's copy is missing: {e}");
            }
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
                self.status = (format!("saved; rebuild to use it{note}"), !note.is_empty());
                if wall_changed {
                    if self.theme.wallpaper.is_none() {
                        // so sway's startup command (until the rebuild)
                        // falls back to the default too
                        let _ = std::fs::remove_file(&copy);
                    }
                    let bg = theme::hex(self.theme.c("bg"));
                    let shown = match &self.theme.wallpaper {
                        Some(w) => wallpaper::show(&copy, &w.mode, &bg),
                        None => wallpaper::show(&self.default_wall(), "fill", &bg),
                    };
                    self.status = match shown {
                        Ok(()) => (
                            "saved; wallpaper shown now, rebuild so it stays after a sway reload"
                                .into(),
                            false,
                        ),
                        Err(e) => (format!("saved; {e}"), true),
                    };
                }
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
                    self.theme.undo_all();
                    self.refresh();
                    self.status = ("back to what was saved".into(), false);
                }
                if reload.clicked() {
                    self.reload();
                }
                ui.separator();
                if changed > 0 {
                    ui.label(RichText::new(format!("{changed} unsaved")).color(col(l.accent)));
                }
                // the presets on the right, the status in the space left
                let status = self.status.clone();
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    self.preset_menu(ui);
                    let (msg, err) = &status;
                    if !msg.is_empty() {
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.add(
                                egui::Label::new(RichText::new(msg).color(col(if *err {
                                    l.red
                                } else {
                                    l.muted
                                })))
                                .truncate(),
                            );
                        });
                    }
                });
            });
            ui.add_space(4.0);
        });

        egui::Panel::left("colours")
            .default_size(340.0)
            .min_size(300.0)
            .show(ui, |ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.tab, Tab::Colours, "colours");
                    ui.selectable_value(&mut self.tab, Tab::Icons, "icons");
                    ui.selectable_value(&mut self.tab, Tab::Wallpaper, "wallpaper");
                });
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| match self.tab {
                    Tab::Colours => {
                        self.colour_list(ui, l);
                        ui.add_space(10.0);
                    }
                    Tab::Icons => {
                        self.icon_list(ui, l);
                        ui.add_space(10.0);
                    }
                    Tab::Wallpaper => {
                        self.wallpaper_list(ui, l);
                        ui.add_space(10.0);
                    }
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
                        self.tab = Tab::Colours;
                    }
                });
            });

        egui::CentralPanel::default().show(ui, |ui| {
            preview::desktop(ui, &self.theme);
        });
    }
}

impl App {
    /// the config's own wallpaper, used when none is picked
    fn default_wall(&self) -> PathBuf {
        self.theme_nix.with_file_name("walls").join("1.png")
    }

    /// the folders added to the picker: remove one, or type one in
    fn folder_list(&mut self, ui: &mut egui::Ui, l: Look) {
        ui.add_space(8.0);
        ui.label(RichText::new("folders").size(11.5).color(col(l.muted)));
        let mut remove = None;
        for (i, f) in self.theme.wall_folders.iter().enumerate() {
            ui.horizontal(|ui| {
                if ui
                    .small_button("×")
                    .on_hover_text("stop looking here")
                    .clicked()
                {
                    remove = Some(i);
                }
                let there = Path::new(f).is_dir();
                let label = RichText::new(if there {
                    f.clone()
                } else {
                    format!("{f} (not there now)")
                })
                .size(11.5);
                // cut short rather than widen the panel
                ui.add(
                    egui::Label::new(if there {
                        label
                    } else {
                        label.color(col(l.muted))
                    })
                    .truncate(),
                )
                .on_hover_text(f);
            });
        }
        if let Some(i) = remove {
            self.theme.wall_folders.remove(i);
            self.walls = None;
        }
        let mut add = false;
        ui.horizontal(|ui| {
            let r = ui.add(
                egui::TextEdit::singleline(&mut self.new_folder)
                    .hint_text("add a folder: /path or ~/path")
                    .desired_width((ui.available_width() - 60.0).max(80.0)),
            );
            add = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            add |= ui
                .add_enabled(!self.new_folder.trim().is_empty(), egui::Button::new("add"))
                .clicked();
        });
        if add {
            match wallpaper::check_folder(&self.new_folder) {
                Ok(f) if self.theme.wall_folders.contains(&f) => {
                    self.status = ("that folder is there already".into(), true);
                }
                Ok(f) => {
                    self.theme.wall_folders.push(f);
                    self.new_folder.clear();
                    self.walls = None;
                }
                Err(e) => self.status = (e, true),
            }
        }
    }

    fn wallpaper_list(&mut self, ui: &mut egui::Ui, l: Look) {
        ui.label(RichText::new("wallpaper").strong().color(col(l.fg)));
        ui.label(
            RichText::new(
                "Saving copies the picked image to ~/.config/theme/wallpaper and shows it at once.",
            )
            .size(11.5)
            .color(col(l.muted)),
        );
        ui.add_space(6.0);
        let changed = self.theme.wallpaper != self.theme.saved_wallpaper;
        ui.horizontal(|ui| {
            let what = match &self.theme.wallpaper {
                Some(w) => Path::new(&w.from)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| w.from.clone()),
                None => "the config's default (walls/1.png)".into(),
            };
            let label = RichText::new(what);
            ui.label(if changed {
                label.color(col(l.accent))
            } else {
                label
            });
            if self.theme.wallpaper.is_some() && ui.small_button("default").clicked() {
                self.theme.wallpaper = None;
            }
        });
        if let Some(w) = &mut self.theme.wallpaper {
            ui.horizontal(|ui| {
                for m in theme::MODES {
                    if ui.selectable_label(w.mode == m, m).clicked() {
                        w.mode = m.to_string();
                    }
                }
            });
            if !Path::new(&w.from).exists() {
                let copied = wallpaper::copy_path(&self.json).exists();
                ui.label(
                    RichText::new(if copied {
                        "the picked image is gone; its copy is still used"
                    } else {
                        "the picked image and its copy are gone: sway shows walls/1.png"
                    })
                    .size(11.5)
                    .color(col(l.muted)),
                );
            }
        }

        self.folder_list(ui, l);

        let theme_nix = self.theme_nix.clone();
        let added = self.theme.wall_folders.clone();
        let default_wall = self.default_wall();
        let default_wall_again = default_wall.clone();
        let walls = self
            .walls
            .get_or_insert_with(|| wallpaper::groups(&wallpaper::folders(&theme_nix, &added)));
        if walls.is_empty() {
            ui.add_space(8.0);
            ui.label(
                RichText::new(
                    "no wallpaper folders: make a Wallpaper folder in Pictures, or add one",
                )
                .color(col(l.muted)),
            );
            return;
        }
        let thumbs = self
            .thumbs
            .get_or_insert_with(|| wallpaper::Thumbs::new(ui.ctx()));
        for (path, r) in thumbs.ready() {
            let tex = r.map(|img| {
                ui.ctx().load_texture(
                    path.display().to_string(),
                    img,
                    egui::TextureOptions::LINEAR,
                )
            });
            self.thumb_tex.insert(path, tex);
        }
        // the one in use: the picked image, or the config's default
        let picked = match &self.theme.wallpaper {
            Some(w) => PathBuf::from(&w.from),
            None => default_wall,
        };
        let mut pick = None;
        let width = ((ui.available_width() - 8.0) / 2.0).max(60.0);
        for (gi, (label, files)) in walls.iter().enumerate() {
            // a line between folders, subfolders included
            ui.add_space(if gi == 0 { 10.0 } else { 6.0 });
            if gi > 0 {
                ui.separator();
            }
            ui.label(RichText::new(label).size(11.5).color(col(l.muted)));
            if files.is_empty() {
                ui.label(RichText::new("no images").size(11.5).color(col(l.muted)));
                continue;
            }
            for row in files.chunks(2) {
                ui.horizontal(|ui| {
                    for f in row {
                        let size = egui::vec2(width, width * 9.0 / 16.0);
                        let (rect, r) = ui.allocate_exact_size(size, egui::Sense::click());
                        // only once it scrolls into view, so a big folder
                        // doesn't make (and keep) hundreds of thumbnails
                        if !self.thumb_tex.contains_key(f) && ui.is_rect_visible(rect) {
                            thumbs.ask(f.clone());
                            self.thumb_tex.insert(f.clone(), Err(String::new()));
                        }
                        let name = f.file_name().map(|n| n.to_string_lossy().to_string());
                        match self.thumb_tex.get(f) {
                            Some(Ok(tex)) => {
                                // fill the box, cut to its shape
                                let [w, h] = tex.size().map(|v| v as f32);
                                let (bw, bh) = (rect.width(), rect.height());
                                let scale = (bw / w).max(bh / h);
                                let (uw, uh) = (bw / (w * scale), bh / (h * scale));
                                let uv = egui::Rect::from_min_max(
                                    egui::pos2((1.0 - uw) / 2.0, (1.0 - uh) / 2.0),
                                    egui::pos2((1.0 + uw) / 2.0, (1.0 + uh) / 2.0),
                                );
                                ui.painter().image(tex.id(), rect, uv, Color32::WHITE);
                            }
                            Some(Err(e)) if !e.is_empty() => {
                                ui.painter().rect_filled(rect, 2.0, col(l.bg_alt));
                                ui.painter().text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "can't read",
                                    egui::FontId::proportional(11.0),
                                    col(l.red),
                                );
                            }
                            _ => {
                                ui.painter().rect_filled(rect, 2.0, col(l.bg_alt));
                            }
                        }
                        let selected = picked == *f;
                        if selected || r.hovered() {
                            ui.painter().rect_stroke(
                                rect,
                                2.0,
                                egui::Stroke::new(
                                    if selected { 3.0 } else { 1.0 },
                                    col(if selected { l.accent } else { l.fg }),
                                ),
                                egui::StrokeKind::Inside,
                            );
                        }
                        let r = r.on_hover_text(name.unwrap_or_default());
                        if r.clicked() && !selected {
                            pick = Some(f.clone());
                        }
                    }
                });
            }
        }
        if let Some(f) = pick {
            let s = f.to_str().unwrap_or("").to_string();
            if f == default_wall_again {
                // the config's own: no copy needed
                self.theme.wallpaper = None;
            } else if s.is_empty() || s.contains(['"', '\\']) || s.chars().any(char::is_control) {
                self.status = (
                    "that file's name or folder has a \" or \\ or odd characters in it: rename it to use it"
                        .into(),
                    true,
                );
            } else {
                let mode = self.theme.wallpaper.as_ref().map(|w| w.mode.clone());
                self.theme.wallpaper = Some(theme::Wallpaper {
                    from: s,
                    mode: mode.unwrap_or_else(|| "fill".into()),
                });
            }
        }
    }

    fn icon_list(&mut self, ui: &mut egui::Ui, l: Look) {
        let v = self.theme.variant;
        ui.label(RichText::new("icon theme").strong().color(col(l.fg)));
        ui.label(
            RichText::new(format!(
                "Only the set picked is installed: rebuild to use it. Shown for {}; \
                 previews are of sets installed now.",
                VARIANTS[v]
            ))
            .size(11.5)
            .color(col(l.muted)),
        );
        if self.theme.icon_sets.is_empty() {
            ui.add_space(6.0);
            ui.label(
                RichText::new("theme.nix has no iconSets: update the config first")
                    .color(col(l.red)),
            );
            return;
        }
        let mut pick = None;
        let mut open = None;
        for s in &self.theme.icon_sets {
            let name = &s.names[v];
            let picked = self.theme.icons == s.key;
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let r = ui
                    .selectable_label(picked, &s.label)
                    .on_hover_text(if s.also.is_empty() {
                        format!("{name} (package {})", s.package)
                    } else {
                        format!("{name} (packages {} {})", s.package, s.also.join(" "))
                    });
                if r.clicked() && !picked {
                    pick = Some(s.key.clone());
                }
                // its page, right after the name
                if let Some(p) = &s.page {
                    let r = ui
                        .add(
                            egui::Label::new(RichText::new("↗").color(col(l.muted)))
                                .sense(egui::Sense::click()),
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text(format!("see what it looks like: {p}"));
                    if r.clicked() {
                        open = Some(p.clone());
                    }
                }
                if picked && self.theme.icons != self.theme.saved_icons {
                    ui.label(RichText::new("unsaved").size(11.5).color(col(l.accent)));
                }
                if s.key == self.theme.icon_default {
                    ui.label(RichText::new("default").size(11.5).color(col(l.muted)));
                }
                if s.key == self.theme.saved_icons {
                    ui.label(RichText::new("saved").size(11.5).color(col(l.muted)));
                }
            });
            let icons = self.previews.entry(name.clone()).or_insert_with(|| {
                if !self.icon_themes.installed(name) {
                    return Vec::new();
                }
                icons::SAMPLES
                    .iter()
                    // the size a file manager shows: some sets draw
                    // small sizes differently (Gruvbox Plus's folders)
                    .map(|names| self.icon_themes.lookup(name, names, 64))
                    .collect()
            });
            if icons.iter().all(Option::is_none) {
                ui.label(
                    RichText::new(if !icons.is_empty() {
                        format!("{name}: installed, no icons to preview")
                    } else if picked {
                        format!("{name}: installs on rebuild")
                    } else {
                        format!("{name}: not installed (pick it and rebuild)")
                    })
                    .size(11.5)
                    .color(col(l.muted)),
                );
                continue;
            }
            ui.horizontal(|ui| {
                for p in icons.iter().flatten() {
                    ui.add(
                        egui::Image::new(format!("file://{}", p.display()))
                            .fit_to_exact_size(egui::vec2(32.0, 32.0)),
                    );
                }
            });
            ui.label(RichText::new(name).size(11.5).color(col(l.muted)));
        }
        if let Some(k) = pick {
            self.theme.icons = k;
        }
        if let Some(url) = open {
            match Command::new("xdg-open").arg(&url).spawn() {
                // waited for on the side, so it doesn't linger
                Ok(mut child) => drop(std::thread::spawn(move || child.wait())),
                Err(e) => self.status = (format!("can't open {url}: {e}"), true),
            }
        }
    }

    /// top right: the presets dropdown; shows the matching preset or "custom"
    fn preset_menu(&mut self, ui: &mut egui::Ui) {
        if self.theme.presets.len() < 2 {
            return;
        }
        let now = self.theme.preset_now();
        let name = |i: usize, p: &theme::Preset| {
            if i == 0 {
                format!("{} (default)", p.label)
            } else {
                p.label.clone()
            }
        };
        let shown = now.map_or("custom".into(), |i| name(i, &self.theme.presets[i]));
        let mut pick = None;
        egui::ComboBox::from_id_salt("preset")
            .selected_text(shown)
            .show_ui(ui, |ui| {
                for (i, p) in self.theme.presets.iter().enumerate() {
                    let r = ui
                        .selectable_label(now == Some(i), name(i, p))
                        .on_hover_text(format!(
                            "sets every colour, dark and light, to {}'s; icons and wallpaper stay",
                            p.label
                        ));
                    if r.clicked() && now != Some(i) {
                        pick = Some(i);
                    }
                }
            })
            .response
            .on_hover_text("colour presets from theme.nix: sets every colour, dark and light");
        ui.label("preset");
        ui.separator();
        if let Some(i) = pick {
            self.theme.apply_preset(i);
            self.find.clear();
            self.refresh();
            let name = &self.theme.presets[i].label;
            self.status = (
                format!("{name}: Save to keep it, Undo all to go back"),
                false,
            );
            // the unsaved count and the status above are drawn already
            ui.ctx().request_repaint();
        }
    }

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
        tab: Tab::Colours,
        icon_themes: icons::Themes::new(icons::roots()),
        previews: HashMap::new(),
        walls: None,
        thumbs: None,
        thumb_tex: HashMap::new(),
        new_folder: String::new(),
    };
    app.refresh();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("theme-edit")
            .with_app_id("theme-edit")
            .with_inner_size([1400.0, 800.0]),
        ..Default::default()
    };
    if let Err(e) = eframe::run_native(
        "theme-edit",
        options,
        Box::new(move |cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    ) {
        eprintln!("theme-edit: {e}");
        std::process::exit(1);
    }
}
