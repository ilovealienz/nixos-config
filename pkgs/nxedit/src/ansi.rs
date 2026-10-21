//! Coloured terminal output (bat, git diff) as ratatui lines: SGR colour
//! and style codes are kept, other escape sequences dropped, tabs expanded.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

pub fn lines(text: &str) -> Vec<Line<'static>> {
    let mut style = Style::default();
    text.lines().map(|l| line(l, &mut style)).collect()
}

fn line(text: &str, style: &mut Style) -> Line<'static> {
    let mut spans = Vec::new();
    let mut buf = String::new();
    let mut col = 0;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\x1b' => {
                if !buf.is_empty() {
                    spans.push(Span::styled(std::mem::take(&mut buf), *style));
                }
                match chars.next() {
                    Some('[') => {
                        let mut params = String::new();
                        let mut fin = None;
                        for c in chars.by_ref() {
                            if ('@'..='~').contains(&c) {
                                fin = Some(c);
                                break;
                            }
                            params.push(c);
                        }
                        if fin == Some('m') {
                            sgr(&params, style);
                        }
                    }
                    Some(']') => {
                        // OSC: up to BEL or ESC \
                        while let Some(c) = chars.next() {
                            if c == '\x07' {
                                break;
                            }
                            if c == '\x1b' {
                                chars.next();
                                break;
                            }
                        }
                    }
                    _ => {}
                }
            }
            '\t' => {
                let n = 4 - col % 4;
                buf.push_str(&" ".repeat(n));
                col += n;
            }
            '\r' => {}
            c if c.is_control() => {}
            c => {
                buf.push(c);
                col += 1;
            }
        }
    }
    if !buf.is_empty() {
        spans.push(Span::styled(buf, *style));
    }
    Line::from(spans)
}

fn sgr(params: &str, style: &mut Style) {
    let nums: Vec<u16> = if params.is_empty() {
        vec![0]
    } else {
        params.split(';').map(|p| p.parse().unwrap_or(0)).collect()
    };
    let mut i = 0;
    while i < nums.len() {
        match nums[i] {
            0 => *style = Style::default(),
            1 => *style = style.add_modifier(Modifier::BOLD),
            2 => *style = style.add_modifier(Modifier::DIM),
            3 => *style = style.add_modifier(Modifier::ITALIC),
            4 => *style = style.add_modifier(Modifier::UNDERLINED),
            7 => *style = style.add_modifier(Modifier::REVERSED),
            22 => *style = style.remove_modifier(Modifier::BOLD | Modifier::DIM),
            23 => *style = style.remove_modifier(Modifier::ITALIC),
            24 => *style = style.remove_modifier(Modifier::UNDERLINED),
            27 => *style = style.remove_modifier(Modifier::REVERSED),
            n @ 30..=37 => *style = style.fg(basic(n - 30, false)),
            n @ 90..=97 => *style = style.fg(basic(n - 90, true)),
            39 => style.fg = None,
            n @ 40..=47 => *style = style.bg(basic(n - 40, false)),
            n @ 100..=107 => *style = style.bg(basic(n - 100, true)),
            49 => style.bg = None,
            n @ (38 | 48) => {
                let colour = match nums.get(i + 1) {
                    Some(5) => {
                        i += 2;
                        nums.get(i).map(|&c| Color::Indexed(c as u8))
                    }
                    Some(2) => {
                        i += 4;
                        match nums.get(i - 2..=i) {
                            Some([r, g, b]) => Some(Color::Rgb(*r as u8, *g as u8, *b as u8)),
                            _ => None,
                        }
                    }
                    _ => None,
                };
                if let Some(c) = colour {
                    *style = if n == 38 { style.fg(c) } else { style.bg(c) };
                }
            }
            _ => {}
        }
        i += 1;
    }
}

fn basic(n: u16, bright: bool) -> Color {
    match (n, bright) {
        (0, false) => Color::Black,
        (1, false) => Color::Red,
        (2, false) => Color::Green,
        (3, false) => Color::Yellow,
        (4, false) => Color::Blue,
        (5, false) => Color::Magenta,
        (6, false) => Color::Cyan,
        (7, false) => Color::Gray,
        (0, true) => Color::DarkGray,
        (1, true) => Color::LightRed,
        (2, true) => Color::LightGreen,
        (3, true) => Color::LightYellow,
        (4, true) => Color::LightBlue,
        (5, true) => Color::LightMagenta,
        (6, true) => Color::LightCyan,
        _ => Color::White,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(l: &Line) -> String {
        l.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn colours_and_resets() {
        let ls = lines("\x1b[31mred\x1b[0m plain \x1b[1;38;5;208morange\x1b[39mbold");
        let l = &ls[0];
        assert_eq!(text(l), "red plain orangebold");
        assert_eq!(l.spans[0].style.fg, Some(Color::Red));
        assert_eq!(l.spans[1].style, Style::default());
        assert_eq!(l.spans[2].style.fg, Some(Color::Indexed(208)));
        assert!(l.spans[2].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(l.spans[3].style.fg, None);
        assert!(l.spans[3].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn truecolour_and_style_carries_over_lines() {
        let ls = lines("\x1b[38;2;1;2;3ma\nb\x1b[m");
        assert_eq!(ls[0].spans[0].style.fg, Some(Color::Rgb(1, 2, 3)));
        assert_eq!(ls[1].spans[0].style.fg, Some(Color::Rgb(1, 2, 3)));
    }

    #[test]
    fn drops_other_sequences_and_expands_tabs() {
        let ls = lines("\x1b]8;;http://x\x1b\\link\x1b[K\ta\tb\r");
        assert_eq!(text(&ls[0]), "link    a   b");
    }
}
