//! The mock desktop and the contrast table, drawn from the live colours.
//! Which colour goes where follows the config: sway/compositor.nix
//! (borders), waybar.nix, kitty.nix, fuzzel.nix, mako.nix, gtk.nix.

use crate::col;
use crate::theme::{self, Rgb, Theme};
use eframe::egui::{
    self, Align2, Color32, FontId, Painter, Pos2, Rect, Stroke, StrokeKind, pos2, vec2,
};

/// draws `parts` (text, colour) one after another; returns the end x
fn spans(p: &Painter, at: Pos2, size: f32, parts: &[(&str, Color32)]) -> f32 {
    let mut x = at.x;
    for (s, c) in parts {
        let r = p.text(
            pos2(x, at.y),
            Align2::LEFT_TOP,
            *s,
            FontId::monospace(size),
            *c,
        );
        x = r.right();
    }
    x
}

fn text(p: &Painter, at: Pos2, size: f32, s: &str, c: Color32) -> Rect {
    p.text(at, Align2::LEFT_TOP, s, FontId::proportional(size), c)
}

fn border(p: &Painter, r: Rect, w: f32, c: Color32) {
    p.rect_stroke(r, 0.0, Stroke::new(w, c), StrokeKind::Inside);
}

pub fn desktop(ui: &mut egui::Ui, t: &Theme) {
    let c = |n: &str| col(t.c(n));
    let w = ui.available_width().max(360.0);
    let h = ui.available_height().max(360.0);
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), egui::Sense::hover());
    let p = ui.painter_at(rect);
    let s = (w.min(h * 1.6) / 900.0).clamp(0.6, 1.3); // text scale
    let mono = 12.0 * s;

    // desktop (stands in for the wallpaper)
    p.rect_filled(rect, 4.0, c("bgDark"));

    // waybar
    let bar = Rect::from_min_size(rect.min, vec2(w, 26.0 * s));
    p.rect_filled(bar, 0.0, c("bg"));
    let mut x = bar.left() + 8.0 * s;
    let y = bar.top() + 5.0 * s;
    for (ws, kind) in [("1", 0), ("2", 1), ("3", 2), ("4", 1)] {
        let r = Rect::from_min_size(pos2(x, bar.top()), vec2(22.0 * s, bar.height()));
        match kind {
            0 => {
                p.text(
                    r.center(),
                    Align2::CENTER_CENTER,
                    ws,
                    FontId::proportional(13.0 * s),
                    c("accent"),
                );
                p.rect_filled(
                    Rect::from_min_max(pos2(r.left(), r.bottom() - 2.0), r.max),
                    0.0,
                    c("accent"),
                );
            }
            2 => {
                p.rect_filled(r, 0.0, c("red"));
                p.text(
                    r.center(),
                    Align2::CENTER_CENTER,
                    ws,
                    FontId::proportional(13.0 * s),
                    c("onAccent"),
                );
            }
            _ => {
                p.text(
                    r.center(),
                    Align2::CENTER_CENTER,
                    ws,
                    FontId::proportional(13.0 * s),
                    c("fg"),
                );
            }
        }
        x = r.right() + 2.0;
    }
    text(
        &p,
        pos2(x + 12.0 * s, y),
        13.0 * s,
        "kitty: ~/nixos",
        c("muted"),
    );
    let mut rx = bar.right() - 10.0 * s;
    let sliver = Rect::from_min_size(pos2(rx - 3.0 * s, bar.top()), vec2(3.0 * s, bar.height()));
    p.rect_filled(sliver, 0.0, c("accent"));
    rx = sliver.left() - 10.0 * s;
    for item in ["16:20", "84%", "12°C"] {
        let r = p.text(
            pos2(rx, y),
            Align2::RIGHT_TOP,
            item,
            FontId::proportional(13.0 * s),
            c("fg"),
        );
        rx = r.left() - 14.0 * s;
    }

    // windows: 2px gaps, 2px borders
    let area = Rect::from_min_max(
        pos2(rect.left() + 2.0, bar.bottom() + 2.0),
        rect.max - vec2(2.0, 2.0),
    );
    let split = area.left() + area.width() * 0.56;
    let left = Rect::from_min_max(area.min, pos2(split - 1.0, area.bottom()));
    let right = Rect::from_min_max(pos2(split + 1.0, area.top()), area.max);

    // focused: kitty
    p.rect_filled(left, 0.0, c("bg"));
    border(&p, left, 2.0, c("accent"));
    let inner = left.shrink(2.0);
    let tabs = Rect::from_min_size(inner.min, vec2(inner.width(), 20.0 * s));
    p.rect_filled(tabs, 0.0, c("bgDark"));
    let t1 = Rect::from_min_size(tabs.min, vec2(110.0 * s, tabs.height()));
    p.rect_filled(t1, 0.0, c("bgAlt"));
    p.text(
        t1.center(),
        Align2::CENTER_CENTER,
        "~/nixos",
        FontId::proportional(12.0 * s),
        c("fgBright"),
    );
    p.text(
        pos2(t1.right() + 55.0 * s, t1.center().y),
        Align2::CENTER_CENTER,
        "music",
        FontId::proportional(12.0 * s),
        c("fg"),
    );

    let lh = mono * 1.45;
    let mut ty = tabs.bottom() + 8.0 * s;
    let tx = inner.left() + 8.0 * s;
    let line = |ty: &mut f32, parts: &[(&str, Color32)]| {
        spans(&p, pos2(tx, *ty), mono, parts);
        *ty += lh;
    };
    let prompt = |cmd: &'static str| [("➜ ", c("green")), ("nixos ", c("yellow")), (cmd, c("fg"))];
    line(&mut ty, &prompt("ls"));
    line(
        &mut ty,
        &[
            ("hosts  ", c("blue")),
            ("sway  ", c("blue")),
            ("flake.nix  ", c("fg")),
            ("theme.nix", c("magenta")),
        ],
    );
    line(&mut ty, &prompt("nxrebuild"));
    line(
        &mut ty,
        &[("building the system configuration...", c("muted"))],
    );
    line(
        &mut ty,
        &[("warning: ", c("orange")), ("Git tree is dirty", c("fg"))],
    );
    line(
        &mut ty,
        &[("error: ", c("red")), ("attribute 'bgg' missing", c("fg"))],
    );
    line(
        &mut ty,
        &[
            ("+ ", c("green")),
            ("pc  ", c("fg")),
            ("+ ", c("green")),
            ("laptop", c("fg")),
        ],
    );
    // terminal selection: surface behind bg text
    let sel_end = spans(&p, pos2(tx, ty), mono, &[("copied ", c("fg"))]);
    let sel = p.layout_no_wrap("this bit".into(), FontId::monospace(mono), c("selectionFg"));
    p.rect_filled(
        Rect::from_min_size(pos2(sel_end, ty), sel.size()),
        0.0,
        c("selectionBg"),
    );
    p.galley(pos2(sel_end, ty), sel, c("selectionFg"));
    ty += lh;
    let cur = spans(&p, pos2(tx, ty), mono, &prompt(""));
    p.rect_filled(
        Rect::from_min_size(pos2(cur, ty), vec2(mono * 0.6, mono * 1.2)),
        0.0,
        c("fg"),
    );

    // unfocused: a GTK file manager
    p.rect_filled(right, 0.0, c("bg"));
    border(&p, right, 2.0, c("surface"));
    let gi = right.shrink(2.0);
    let side = Rect::from_min_size(gi.min, vec2(gi.width() * 0.32, gi.height()));
    p.rect_filled(side, 0.0, c("bgAlt"));
    let mut sy = side.top() + 10.0 * s;
    text(
        &p,
        pos2(side.left() + 10.0 * s, sy),
        11.0 * s,
        "PLACES",
        c("muted"),
    );
    sy += 20.0 * s;
    for (i, place) in ["Home", "Downloads", "Shared"].iter().enumerate() {
        let r = Rect::from_min_size(
            pos2(side.left(), sy - 3.0 * s),
            vec2(side.width(), 22.0 * s),
        );
        if i == 1 {
            p.rect_filled(r, 0.0, c("surface"));
        }
        text(
            &p,
            pos2(side.left() + 10.0 * s, sy),
            13.0 * s,
            place,
            c("fg"),
        );
        sy += 24.0 * s;
    }
    let mut fy = gi.top() + 10.0 * s;
    let fx = side.right() + 10.0 * s;
    for (i, (name, size)) in [
        ("Documents", "12 items"),
        ("theme.nix", "2.1 kB"),
        ("wallpaper.png", "4.8 MB"),
        ("notes.txt", "860 B"),
    ]
    .iter()
    .enumerate()
    {
        let r = Rect::from_min_max(
            pos2(side.right(), fy - 4.0 * s),
            pos2(gi.right(), fy + 20.0 * s),
        );
        let (fg, sub) = if i == 1 {
            p.rect_filled(r, 0.0, c("selectionBg"));
            (c("selectionFg"), c("selectionFg"))
        } else {
            (c("fg"), c("muted"))
        };
        text(&p, pos2(fx, fy), 13.0 * s, name, fg);
        p.text(
            pos2(gi.right() - 10.0 * s, fy),
            Align2::RIGHT_TOP,
            *size,
            FontId::proportional(12.0 * s),
            sub,
        );
        fy += 26.0 * s;
    }

    // a GTK button: accent with onAccent text
    let bw = 70.0 * s;
    let br = Rect::from_min_size(
        pos2(gi.right() - bw - 10.0 * s, gi.bottom() - 34.0 * s),
        vec2(bw, 24.0 * s),
    );
    p.rect_filled(br, 4.0, c("accent"));
    p.text(
        br.center(),
        Align2::CENTER_CENTER,
        "Open",
        FontId::proportional(13.0 * s),
        c("onAccent"),
    );

    // fuzzel
    let fw = 250.0 * s;
    let fz = Rect::from_min_size(
        pos2(
            right.center().x - fw / 2.0,
            right.top() + right.height() * 0.42,
        ),
        vec2(fw, 112.0 * s),
    );
    p.rect_filled(fz, 0.0, col(t.c("bg")).gamma_multiply(0.93));
    border(&p, fz, 2.0, c("accent"));
    let mut zy = fz.top() + 8.0 * s;
    let zx = fz.left() + 10.0 * s;
    spans(
        &p,
        pos2(zx, zy),
        mono,
        &[("❯ ", c("accent")), ("fir", c("fg"))],
    );
    zy += 24.0 * s;
    for (i, (m, rest)) in [
        ("Fir", "efox"),
        ("fir", "ewall-config"),
        ("Fir", "mware updates"),
    ]
    .iter()
    .enumerate()
    {
        let r = Rect::from_min_max(
            pos2(fz.left() + 4.0, zy - 3.0 * s),
            pos2(fz.right() - 4.0, zy + 19.0 * s),
        );
        let fg = if i == 0 {
            p.rect_filled(r, 0.0, c("surface"));
            c("fgBright")
        } else {
            c("fg")
        };
        spans(&p, pos2(zx, zy), mono, &[(m, c("accent")), (rest, fg)]);
        zy += 24.0 * s;
    }

    // mako, top right
    let nw = 270.0 * s;
    let mut ny = bar.bottom() + 10.0 * s;
    for critical in [false, true] {
        let r = Rect::from_min_size(
            pos2(rect.right() - nw - 10.0 * s, ny),
            vec2(nw, if critical { 50.0 } else { 62.0 } * s),
        );
        p.rect_filled(r, 0.0, c("bg"));
        let (edge, fg) = if critical {
            (c("red"), c("red"))
        } else {
            (c("accent"), c("fg"))
        };
        if !critical {
            // progress: accentDim over the left part
            let pr = Rect::from_min_size(r.min, vec2(r.width() * 0.6, r.height()));
            p.rect_filled(pr, 0.0, c("accentDim"));
        }
        border(&p, r, 2.0, edge);
        let (title, body) = if critical {
            ("battery low", "8% left")
        } else {
            ("volume", "60%")
        };
        let tr = p.text(
            r.min + vec2(10.0, 8.0) * s,
            Align2::LEFT_TOP,
            title,
            FontId::proportional(13.0 * s),
            fg,
        );
        p.text(
            pos2(tr.left(), tr.bottom() + 4.0 * s),
            Align2::LEFT_TOP,
            body,
            FontId::proportional(12.0 * s),
            fg,
        );
        ny = r.bottom() + 8.0 * s;
    }
}

/// What a pair of colours is used for, which sets how much contrast it
/// needs (WCAG 2: 4.5 for text, 3 for large or bold text).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Role {
    /// read in sentences: needs 4.5
    Body,
    /// short labels (tabs, buttons, workspaces, selected rows): 4.5 is
    /// best, 3 is fine
    Label,
    /// dim on purpose (hints, labels in grey): 3 is the aim, more is fine
    Dim,
    /// terminal colours: coloured output, often short: like a label
    Term,
}

/// the groups the table shows, in order: role, heading, its bar
const GROUPS: &[(Role, &str, &str)] = &[
    (Role::Body, "text you read", "needs 4.5"),
    (Role::Label, "labels and highlights", "4.5 best, 3 fine"),
    (Role::Dim, "dim on purpose", "3 is enough"),
    (Role::Term, "terminal colours", "4.5 best, 3 fine"),
];

/// text colour, background, what it's used for, and its role
const PAIRS: &[(&str, &str, &str, Role)] = &[
    ("fg", "bg", "text", Role::Body),
    ("fgBright", "bg", "bright text", Role::Body),
    ("muted", "bg", "dim text, labels", Role::Dim),
    ("fg", "bgAlt", "text on raised views", Role::Body),
    (
        "selectionFg",
        "selectionBg",
        "selected rows, terminal selection",
        Role::Label,
    ),
    ("fgBright", "surface", "launcher selection", Role::Label),
    ("accent", "bg", "focused workspace", Role::Label),
    (
        "onAccent",
        "accent",
        "text on accent: tab titles, buttons",
        Role::Label,
    ),
    ("onAccent", "red", "urgent workspace", Role::Label),
    ("red", "bg", "errors", Role::Body),
    ("orange", "bg", "warnings", Role::Body),
    ("yellow", "bg", "terminal yellow", Role::Term),
    ("green", "bg", "terminal green", Role::Term),
    ("blue", "bg", "terminal blue", Role::Term),
    ("magenta", "bg", "terminal magenta", Role::Term),
];

/// How a verdict looks: fine, a caution, or a problem.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mark {
    Good,
    Okay,
    Bad,
}

/// The ratio as shown: cut to two decimals, never rounded up, so a
/// number shown as passing always passes (WCAG: 4.499 isn't 4.5).
pub fn shown(r: f32) -> String {
    format!("{:.2}", (r as f64 * 100.0).floor() / 100.0)
}

/// What a ratio means for this role, judged on the exact ratio.
pub fn verdict(r: f32, role: Role) -> (&'static str, Mark) {
    match role {
        Role::Body if r >= 4.5 => ("good", Mark::Good),
        Role::Body if r >= 3.0 => ("too faint for reading", Mark::Okay),
        Role::Body => ("hard to read", Mark::Bad),
        Role::Label | Role::Term if r >= 4.5 => ("good", Mark::Good),
        Role::Label if r >= 3.0 => ("fine for labels", Mark::Good),
        Role::Term if r >= 3.0 => ("fine for short output", Mark::Good),
        Role::Dim if r >= 4.5 => ("good, not very dim", Mark::Good),
        Role::Dim if r >= 3.0 => ("dim, as meant", Mark::Good),
        _ => ("hard to read", Mark::Bad),
    }
}

/// returns the colours of a row that was clicked: its two colours, or for
/// a worked-out one, the colours it's mixed from
pub fn contrast(
    ui: &mut egui::Ui,
    t: &Theme,
    muted: Rgb,
    red: Rgb,
    green: Rgb,
    accent: Rgb,
) -> Option<Vec<String>> {
    let mut clicked = None;
    ui.label(egui::RichText::new("contrast").strong());
    ui.label(
        egui::RichText::new(
            "Each group is judged for its use (WCAG contrast). Click a row to find its colours.",
        )
        .size(11.5)
        .color(col(muted)),
    );
    ui.add_space(4.0);
    // the panel's width, for the lines between groups
    let across = ui.max_rect().x_range();
    egui::Grid::new("contrast")
        .num_columns(3)
        .spacing([14.0, 4.0])
        .striped(false)
        .show(ui, |ui| {
            for (gi, (group, title, bar)) in GROUPS.iter().enumerate() {
                // the group's heading, with a line above all but the first
                let tall = if gi > 0 { 30.0 } else { 18.0 };
                let (head, _) = ui.allocate_exact_size(vec2(44.0, tall), egui::Sense::hover());
                if gi > 0 {
                    ui.painter().hline(
                        across,
                        head.top() + 5.0,
                        Stroke::new(1.0, col(muted).gamma_multiply(0.5)),
                    );
                }
                ui.label(
                    egui::RichText::new(*title)
                        .size(12.0)
                        .strong()
                        .color(col(muted)),
                );
                ui.label(egui::RichText::new(*bar).size(11.5).color(col(muted)));
                ui.end_row();
                for (fg, bg, what, role) in PAIRS.iter().filter(|p| p.3 == *group) {
                    let (Some(f), Some(b)) = (t.get(fg), t.get(bg)) else {
                        continue;
                    };
                    let r = theme::contrast(f, b);
                    let (sample, sr) =
                        ui.allocate_exact_size(vec2(44.0, 20.0), egui::Sense::click());
                    ui.painter().rect_filled(sample, 2.0, col(b));
                    ui.painter().text(
                        sample.center(),
                        Align2::CENTER_CENTER,
                        "Aa",
                        FontId::proportional(14.0),
                        col(f),
                    );
                    let made = |n: &str| match t.derived.iter().find(|d| d.name == n) {
                        Some(d) => format!("\n{n} is worked out from {} and {}", d.a, d.b),
                        None => String::new(),
                    };
                    let need = match role {
                        Role::Body => "read in sentences: needs 4.5",
                        Role::Label => "short labels: 4.5 is best, 3 is fine",
                        Role::Dim => "dim on purpose: 3 is enough",
                        Role::Term => "terminal colour: 4.5 is best, 3 is fine",
                    };
                    let hover = format!(
                        "{fg} on {bg}{}{}\n{need}\nclick to find them",
                        made(fg),
                        made(bg)
                    );
                    let lr = ui.add(egui::Label::new(*what).sense(egui::Sense::click()));
                    if sr.union(lr.clone()).on_hover_text(hover).clicked() {
                        let mut names = Vec::new();
                        for n in [fg, bg] {
                            names.push(n.to_string());
                            if let Some(d) = t.derived.iter().find(|d| d.name == *n) {
                                names.extend([d.a.clone(), d.b.clone()]);
                            }
                        }
                        clicked = Some(names);
                    }
                    let (v, mark) = verdict(r, *role);
                    let c = match mark {
                        Mark::Good => green,
                        Mark::Okay => accent,
                        Mark::Bad => red,
                    };
                    ui.label(egui::RichText::new(format!("{}  {v}", shown(r))).color(col(c)));
                    ui.end_row();
                }
            }
        });
    clicked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_never_round_up() {
        assert_eq!(shown(4.499), "4.49");
        assert_eq!(shown(4.4999), "4.49");
        assert_eq!(shown(4.5), "4.50");
        assert_eq!(shown(21.0), "21.00");
        assert_eq!(shown(1.0), "1.00");
    }

    #[test]
    fn verdicts_by_role() {
        // just under 4.5: shown as 4.49 and not good for sentences
        assert_eq!(verdict(4.499, Role::Body).1, Mark::Okay);
        assert_eq!(verdict(4.5, Role::Body).1, Mark::Good);
        // text on accent at 4.40: fine as a label
        assert_eq!(verdict(4.40, Role::Label), ("fine for labels", Mark::Good));
        // dim text at 3.6: as meant
        assert_eq!(verdict(3.6, Role::Dim), ("dim, as meant", Mark::Good));
        assert_eq!(verdict(2.9, Role::Dim).1, Mark::Bad);
        assert_eq!(verdict(2.9, Role::Label).1, Mark::Bad);
        assert_eq!(verdict(2.9, Role::Term).1, Mark::Bad);
        assert_eq!(verdict(2.9, Role::Body).1, Mark::Bad);
        assert_eq!(verdict(3.0, Role::Term).1, Mark::Good);
        assert_eq!(verdict(7.8, Role::Dim).0, "good, not very dim");
    }

    #[test]
    fn every_pair_is_in_a_group() {
        for (_, _, what, role) in PAIRS {
            assert!(GROUPS.iter().any(|g| g.0 == *role), "{what}");
        }
    }

    #[test]
    fn every_pair_has_colours_in_the_test_theme() {
        let t = theme::parse(include_str!("../testdata/theme.nix")).unwrap();
        for (fg, bg, what, _) in PAIRS {
            assert!(t.get(fg).is_some() && t.get(bg).is_some(), "{what}");
        }
    }
}
