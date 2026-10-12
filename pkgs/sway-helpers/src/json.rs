//! A small JSON reader for sway's replies and events.
//!
//! Values are read straight into the structs below. Fields that aren't
//! needed are skipped without being copied, so a tree with long window
//! titles costs no more than the ids and names that are kept.

pub struct Parser<'a> {
    b: &'a [u8],
    i: usize,
    depth: usize,
}

/// sway's trees are a few levels deep; anything past this is not sway
const MAX_DEPTH: usize = 128;

impl<'a> Parser<'a> {
    pub fn new(b: &'a [u8]) -> Self {
        Parser { b, i: 0, depth: 0 }
    }

    fn ws(&mut self) {
        while self.i < self.b.len() && self.b[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.ws();
        self.b.get(self.i).copied()
    }

    fn eat(&mut self, c: u8) -> Option<()> {
        if self.peek()? == c {
            self.i += 1;
            Some(())
        } else {
            None
        }
    }

    fn word(&mut self, w: &[u8]) -> Option<()> {
        if self.b.get(self.i..)?.starts_with(w) {
            self.i += w.len();
            Some(())
        } else {
            None
        }
    }

    fn enter(&mut self) -> Option<()> {
        self.depth += 1;
        (self.depth <= MAX_DEPTH).then_some(())
    }

    /// Calls `field` for each key of an object. `field` must read or skip
    /// the value. Keys are given as raw bytes (sway's keys have no escapes).
    pub fn object(&mut self, mut field: impl FnMut(&mut Self, &[u8]) -> Option<()>) -> Option<()> {
        self.eat(b'{')?;
        self.enter()?;
        if self.peek()? == b'}' {
            self.i += 1;
            self.depth -= 1;
            return Some(());
        }
        loop {
            self.ws();
            let key = self.raw_string()?;
            self.eat(b':')?;
            field(self, key)?;
            match self.peek()? {
                b',' => self.i += 1,
                b'}' => {
                    self.i += 1;
                    self.depth -= 1;
                    return Some(());
                }
                _ => return None,
            }
        }
    }

    /// Calls `item` for each element of an array (or reads null as empty).
    pub fn array(&mut self, mut item: impl FnMut(&mut Self) -> Option<()>) -> Option<()> {
        if self.peek()? == b'n' {
            return self.word(b"null");
        }
        self.eat(b'[')?;
        self.enter()?;
        if self.peek()? == b']' {
            self.i += 1;
            self.depth -= 1;
            return Some(());
        }
        loop {
            item(self)?;
            match self.peek()? {
                b',' => self.i += 1,
                b']' => {
                    self.i += 1;
                    self.depth -= 1;
                    return Some(());
                }
                _ => return None,
            }
        }
    }

    /// Skips any value without keeping it.
    pub fn skip(&mut self) -> Option<()> {
        match self.peek()? {
            b'{' => self.object(|p, _| p.skip()),
            b'[' => self.array(|p| p.skip()),
            b'"' => self.raw_string().map(|_| ()),
            b't' => self.word(b"true"),
            b'f' => self.word(b"false"),
            b'n' => self.word(b"null"),
            _ => self.number().map(|_| ()),
        }
    }

    /// true/false; null reads as false
    pub fn bool(&mut self) -> Option<bool> {
        match self.peek()? {
            b't' => self.word(b"true").map(|_| true),
            b'f' => self.word(b"false").map(|_| false),
            b'n' => self.word(b"null").map(|_| false),
            _ => None,
        }
    }

    /// An integer; fractions keep their whole part, null reads as 0.
    pub fn int(&mut self) -> Option<i64> {
        if self.peek()? == b'n' {
            return self.word(b"null").map(|_| 0);
        }
        self.number()
    }

    fn number(&mut self) -> Option<i64> {
        self.ws();
        let start = self.i;
        while let Some(&c) = self.b.get(self.i) {
            if c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.' | b'e' | b'E') {
                self.i += 1;
            } else {
                break;
            }
        }
        if start == self.i {
            return None;
        }
        let text = std::str::from_utf8(&self.b[start..self.i]).ok()?;
        let whole = text.split(['.', 'e', 'E']).next()?;
        Some(whole.parse().unwrap_or(0))
    }

    /// The bytes between the quotes, escapes left as they are.
    fn raw_string(&mut self) -> Option<&'a [u8]> {
        self.eat(b'"')?;
        let start = self.i;
        loop {
            match *self.b.get(self.i)? {
                b'"' => {
                    self.i += 1;
                    return Some(&self.b[start..self.i - 1]);
                }
                b'\\' => self.i += 2,
                _ => self.i += 1,
            }
        }
    }

    /// A string with its escapes decoded; null reads as "".
    pub fn string(&mut self) -> Option<String> {
        if self.peek()? == b'n' {
            return self.word(b"null").map(|_| String::new());
        }
        let raw = self.raw_string()?;
        if !raw.contains(&b'\\') {
            return String::from_utf8(raw.to_vec()).ok();
        }
        let mut out = Vec::with_capacity(raw.len());
        let mut i = 0;
        while i < raw.len() {
            let c = raw[i];
            i += 1;
            if c != b'\\' {
                out.push(c);
                continue;
            }
            let e = *raw.get(i)?;
            i += 1;
            match e {
                b'n' => out.push(b'\n'),
                b't' => out.push(b'\t'),
                b'r' => out.push(b'\r'),
                b'b' => out.push(8),
                b'f' => out.push(12),
                b'u' => {
                    let hex = |at: usize| -> Option<u32> {
                        u32::from_str_radix(std::str::from_utf8(raw.get(at..at + 4)?).ok()?, 16).ok()
                    };
                    let mut cp = hex(i)?;
                    i += 4;
                    // surrogate pair (emoji and the like in window titles)
                    if (0xD800..0xDC00).contains(&cp) && raw.get(i..i + 2) == Some(b"\\u") {
                        if let Some(lo) = hex(i + 2).filter(|lo| (0xDC00..0xE000).contains(lo)) {
                            cp = 0x10000 + ((cp - 0xD800) << 10) + (lo - 0xDC00);
                            i += 6;
                        }
                    }
                    let ch = char::from_u32(cp).unwrap_or('\u{FFFD}');
                    out.extend_from_slice(ch.encode_utf8(&mut [0; 4]).as_bytes());
                }
                other => out.push(other), // \" \\ \/
            }
        }
        String::from_utf8(out).ok()
    }
}

// ── the parts of sway's JSON this program uses ──

/// A node of sway's tree: output, workspace, split container or window.
#[derive(Default)]
pub struct Node {
    pub id: i64,
    pub kind: String,   // "root", "output", "workspace", "con", "floating_con"
    pub name: String,   // kept for workspaces only (window titles are skipped)
    pub layout: String, // "splith", "splitv", "tabbed", "stacked", "output", "none"
    pub shell: String,  // "xdg_shell" or "xwayland" for windows, "" otherwise
    pub app_id: String,
    pub class: String, // XWayland: window_properties.class
    pub window: u32,   // XWayland: the X11 window id, 0 otherwise
    pub focused: bool,
    pub nodes: Vec<Node>,          // tiled children
    pub floating_nodes: Vec<Node>, // floating children
}

impl Node {
    pub fn parse(p: &mut Parser) -> Option<Node> {
        let mut n = Node::default();
        let mut name = None;
        p.object(|p, key| {
            match key {
                b"id" => n.id = p.int()?,
                b"type" => n.kind = p.string()?,
                b"name" => name = Some(p.string()?),
                b"layout" => n.layout = p.string()?,
                b"shell" => n.shell = p.string()?,
                b"app_id" => n.app_id = p.string()?,
                b"window" => n.window = p.int()? as u32,
                b"focused" => n.focused = p.bool()?,
                b"window_properties" => {
                    if p.peek()? == b'n' {
                        p.skip()?
                    } else {
                        p.object(|p, k| {
                            if k == b"class" {
                                n.class = p.string()?;
                                Some(())
                            } else {
                                p.skip()
                            }
                        })?
                    }
                }
                b"nodes" => p.array(|p| {
                    n.nodes.push(Node::parse(p)?);
                    Some(())
                })?,
                b"floating_nodes" => p.array(|p| {
                    n.floating_nodes.push(Node::parse(p)?);
                    Some(())
                })?,
                _ => p.skip()?,
            }
            Some(())
        })?;
        if n.kind == "workspace" {
            n.name = name.unwrap_or_default();
        }
        Some(n)
    }

    pub fn from(b: &[u8]) -> Option<Node> {
        Node::parse(&mut Parser::new(b))
    }

    /// Wayland app_id, or the X11 class for XWayland windows.
    pub fn app(&self) -> &str {
        if self.app_id.is_empty() {
            &self.class
        } else {
            &self.app_id
        }
    }

    pub fn is_window(&self) -> bool {
        !self.shell.is_empty()
    }

    pub fn is_scratchpad(&self) -> bool {
        self.kind == "workspace" && self.name == "__i3_scratch"
    }

    /// Every node below this one, tiled and floating.
    pub fn each(&self, f: &mut impl FnMut(&Node)) {
        for c in self.nodes.iter().chain(&self.floating_nodes) {
            f(c);
            c.each(f);
        }
    }
}

/// A window event: what changed, and the window it happened to.
pub struct WindowEvent {
    pub change: String,
    pub container: Node,
}

impl WindowEvent {
    pub fn from(b: &[u8]) -> Option<WindowEvent> {
        let mut change = String::new();
        let mut container = None;
        Parser::new(b).object(|p, key| {
            match key {
                b"change" => change = p.string()?,
                b"container" => container = Some(Node::parse(p)?),
                _ => p.skip()?,
            }
            Some(())
        })?;
        Some(WindowEvent { change, container: container? })
    }
}

/// The command of a binding event, e.g. "nop group-drag".
pub fn binding_command(b: &[u8]) -> Option<String> {
    let mut command = None;
    Parser::new(b).object(|p, key| {
        if key == b"binding" {
            p.object(|p, k| {
                if k == b"command" {
                    command = Some(p.string()?);
                    Some(())
                } else {
                    p.skip()
                }
            })
        } else {
            p.skip()
        }
    })?;
    command
}

/// The errors in a RUN_COMMAND or SUBSCRIBE reply. A command reply is a
/// list with one result per command; a subscribe reply is one object.
pub fn reply_errors(b: &[u8]) -> Vec<String> {
    let mut errors = Vec::new();
    let mut result = |p: &mut Parser| {
        let mut ok = true;
        let mut error = String::new();
        p.object(|p, key| {
            match key {
                b"success" => ok = p.bool()?,
                b"error" => error = p.string()?,
                _ => p.skip()?,
            }
            Some(())
        })?;
        if !ok {
            errors.push(if error.is_empty() { "failed".into() } else { error });
        }
        Some(())
    };
    let mut p = Parser::new(b);
    let parsed = match p.peek() {
        Some(b'[') => p.array(&mut result),
        _ => result(&mut p),
    };
    if parsed.is_none() {
        errors.push("unreadable reply".into());
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_keeps_what_it_needs() {
        let j = br#"{"id": 1, "type": "root", "name": "root", "nodes": [
            {"id": 2, "type": "output", "nodes": [
              {"id": 3, "type": "workspace", "name": "1", "layout": "splith",
               "nodes": [{"id": 4, "type": "con", "name": "a \"title\" \ud83d\ude00 \/x",
                          "shell": "xwayland", "window": 4194314, "focused": true,
                          "app_id": null, "window_properties": {"class": "Steam", "title": "x"},
                          "rect": {"x": 0, "y": 0}, "marks": [], "percent": 0.5, "nodes": []}],
               "floating_nodes": [{"id": 5, "type": "floating_con", "app_id": "foot",
                                   "shell": "xdg_shell", "window": null, "nodes": []}]}]}]}"#;
        let root = Node::from(j).unwrap();
        let ws = &root.nodes[0].nodes[0];
        assert_eq!(ws.name, "1");
        let w = &ws.nodes[0];
        assert_eq!((w.id, w.window, w.focused), (4, 4194314, true));
        assert_eq!(w.app(), "Steam");
        assert!(w.name.is_empty()); // titles aren't kept
        assert_eq!(ws.floating_nodes[0].app(), "foot");
        let mut n = 0;
        root.each(&mut |_| n += 1);
        assert_eq!(n, 4);
    }

    #[test]
    fn strings_decode() {
        let mut p = Parser::new(br#""a\"b\\c\/d\u00e9\ud83d\ude00\n""#);
        assert_eq!(p.string().unwrap(), "a\"b\\c/dé😀\n");
    }

    #[test]
    fn events_and_replies() {
        let ev = br#"{"change": "new", "container": {"id": 9, "app_id": "vesktop", "nodes": []}}"#;
        let e = WindowEvent::from(ev).unwrap();
        assert_eq!((e.change.as_str(), e.container.id, e.container.app()), ("new", 9, "vesktop"));
        let b = br#"{"change": "run", "binding": {"command": "nop group-drag", "symbol": "Shift_L"}}"#;
        assert_eq!(binding_command(b).unwrap(), "nop group-drag");
        assert!(reply_errors(br#"[{"success": true}, {"success": true}]"#).is_empty());
        assert_eq!(reply_errors(br#"[{"success": false, "parse_error": false, "error": "No matching node"}]"#),
                   vec!["No matching node".to_string()]);
        assert!(reply_errors(br#"{"success": true}"#).is_empty());
    }

    #[test]
    fn bad_input_is_rejected() {
        assert!(Node::from(b"").is_none());
        assert!(Node::from(br#"{"id": 1, "nodes": [{"id": 2"#).is_none());
        assert!(Node::from(br#"{"id": "#).is_none());
        let deep = "[".repeat(1000);
        assert!(Parser::new(deep.as_bytes()).skip().is_none());
    }
}
