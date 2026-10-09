// nxshortcuts: a searchable list of the desktop's shortcuts in the terminal.
//
// The list comes from a JSON file that Nix builds from sway/shortcuts.nix,
// so the shortcuts are written down in one place. Type to filter (every
// word must match), arrows/PgUp/PgDn to scroll, Tab/Shift+Tab to jump
// between sections, Esc clears the filter or quits, Ctrl+C quits.
// When stdout isn't a terminal it prints plain text instead, for grep.
//
// Standard library only: raw mode and the window size come from ioctls.
package main

import (
	"encoding/json"
	"fmt"
	"os"
	"os/signal"
	"strconv"
	"strings"
	"syscall"
	"unicode"
	"unicode/utf8"
	"unsafe"
)

// set at build time: -X main.dataPath=/nix/store/...-shortcuts.json
var dataPath = ""

type item struct {
	Keys []string `json:"keys"`
	Desc string   `json:"desc"`
	Note string   `json:"note"`
}

type section struct {
	Name  string `json:"name"`
	Note  string `json:"note"`
	Style string `json:"style"` // "keys" (default): keycaps; "cmd": commands
	Items []item `json:"items"`
}

type data struct {
	Colors   map[string]string `json:"colors"`
	Sections []section         `json:"sections"`
}

// ── terminal ──

func ioctl(fd, req uintptr, arg unsafe.Pointer) error {
	if _, _, e := syscall.Syscall(syscall.SYS_IOCTL, fd, req, uintptr(arg)); e != 0 {
		return e
	}
	return nil
}

func isTerminal(f *os.File) bool {
	var t syscall.Termios
	return ioctl(f.Fd(), syscall.TCGETS, unsafe.Pointer(&t)) == nil
}

func size() (rows, cols int) {
	var ws struct{ Row, Col, X, Y uint16 }
	if ioctl(os.Stdout.Fd(), syscall.TIOCGWINSZ, unsafe.Pointer(&ws)) != nil || ws.Col == 0 {
		return 24, 80
	}
	return int(ws.Row), int(ws.Col)
}

func rawMode() (restore func(), err error) {
	fd := os.Stdin.Fd()
	var old syscall.Termios
	if err := ioctl(fd, syscall.TCGETS, unsafe.Pointer(&old)); err != nil {
		return nil, err
	}
	t := old
	t.Iflag &^= syscall.ICRNL | syscall.IXON | syscall.BRKINT | syscall.INPCK | syscall.ISTRIP
	t.Lflag &^= syscall.ECHO | syscall.ICANON | syscall.ISIG | syscall.IEXTEN
	t.Cc[syscall.VMIN] = 1
	t.Cc[syscall.VTIME] = 0
	if err := ioctl(fd, syscall.TCSETS, unsafe.Pointer(&t)); err != nil {
		return nil, err
	}
	return func() { ioctl(fd, syscall.TCSETS, unsafe.Pointer(&old)) }, nil
}

// ── colours ──

type palette struct{ accent, bright, fg, muted, capBg, line string }

func fgc(hex string) string {
	r, g, b := rgb(hex)
	return fmt.Sprintf("\x1b[38;2;%d;%d;%dm", r, g, b)
}

func bgc(hex string) string {
	r, g, b := rgb(hex)
	return fmt.Sprintf("\x1b[48;2;%d;%d;%dm", r, g, b)
}

func rgb(hex string) (int64, int64, int64) {
	hex = strings.TrimPrefix(hex, "#")
	if len(hex) != 6 {
		return 200, 200, 200
	}
	r, _ := strconv.ParseInt(hex[0:2], 16, 0)
	g, _ := strconv.ParseInt(hex[2:4], 16, 0)
	b, _ := strconv.ParseInt(hex[4:6], 16, 0)
	return r, g, b
}

func pick(c map[string]string, k, def string) string {
	if v, ok := c[k]; ok && v != "" {
		return v
	}
	return def
}

const reset = "\x1b[0m"
const bold = "\x1b[1m"

// ── layout ──

func width(s string) int { return len([]rune(s)) }

func cut(s string, n int) string {
	r := []rune(s)
	if n <= 0 {
		return ""
	}
	if len(r) <= n {
		return s
	}
	if n == 1 {
		return "…"
	}
	return string(r[:n-1]) + "…"
}

func isMod(k string) bool {
	switch k {
	case "Super", "Shift", "Alt", "Ctrl":
		return true
	}
	return false
}

// width of the key column for an item
func keysWidth(s section, it item) int {
	if s.Style == "cmd" {
		return width(strings.Join(it.Keys, " "))
	}
	w := 0
	for i, k := range it.Keys {
		if i > 0 {
			w++ // "+"
		}
		w += width(k) + 2
	}
	return w
}

func renderKeys(p palette, s section, it item) string {
	if s.Style == "cmd" {
		return p.accent + bold + strings.Join(it.Keys, " ") + reset
	}
	var b strings.Builder
	for i, k := range it.Keys {
		if i > 0 {
			b.WriteString(p.muted + "+" + reset)
		}
		col := p.bright
		if isMod(k) {
			col = p.accent
		}
		b.WriteString(p.capBg + col + bold + " " + k + " " + reset)
	}
	return b.String()
}

// a screen line: a section header, a blank, or an item
type line struct {
	header bool
	sec    *section
	it     *item
}

func matches(s *section, it *item, terms []string) bool {
	text := strings.ToLower(s.Name + " " + strings.Join(it.Keys, " ") + " " + it.Desc + " " + it.Note)
	for _, t := range terms {
		if !strings.Contains(text, t) {
			return false
		}
	}
	return true
}

func build(d *data, query string) (lines []line, shown int) {
	terms := strings.Fields(strings.ToLower(query))
	for si := range d.Sections {
		s := &d.Sections[si]
		var items []line
		for ii := range s.Items {
			if matches(s, &s.Items[ii], terms) {
				items = append(items, line{sec: s, it: &s.Items[ii]})
			}
		}
		if len(items) == 0 {
			continue
		}
		if len(lines) > 0 {
			lines = append(lines, line{})
		}
		lines = append(lines, line{header: true, sec: s})
		lines = append(lines, items...)
		shown += len(items)
	}
	return lines, shown
}

func total(d *data) (n int) {
	for _, s := range d.Sections {
		n += len(s.Items)
	}
	return n
}

// ── drawing ──

type view struct {
	d      *data
	p      palette
	query  string
	scroll int
	lines  []line
	shown  int
	keyCol int
}

func (v *view) refilter() {
	v.lines, v.shown = build(v.d, v.query)
	v.scroll = 0
}

func (v *view) height() int {
	rows, _ := size()
	return max(rows-4, 1) // search bar + rule on top, rule + hints below
}

func (v *view) clamp() {
	v.scroll = min(v.scroll, max(len(v.lines)-v.height(), 0))
	v.scroll = max(v.scroll, 0)
}

func (v *view) draw() {
	rows, cols := size()
	p := v.p
	var b strings.Builder
	b.WriteString("\x1b[H")

	// search bar
	count := fmt.Sprintf("%d/%d", v.shown, total(v.d))
	prompt := p.accent + bold + " nxshortcuts " + reset + p.muted + "› " + reset
	q := cut(v.query, cols-width(count)-18)
	b.WriteString(prompt + p.bright + q + reset + p.accent + "▏" + reset)
	if v.query == "" {
		b.WriteString(p.muted + cut("type to filter", cols-width(count)-18) + reset)
	}
	b.WriteString("\x1b[K")
	b.WriteString(fmt.Sprintf("\x1b[1;%dH", max(cols-width(count), 1)))
	b.WriteString(p.muted + count + reset + "\r\n")
	b.WriteString(p.line + strings.Repeat("─", cols) + reset + "\x1b[K\r\n")

	h := v.height()
	v.clamp()
	descCol := v.keyCol + 4
	for i := 0; i < h; i++ {
		idx := v.scroll + i
		if idx < len(v.lines) {
			l := v.lines[idx]
			switch {
			case l.header:
				name := strings.ToUpper(l.sec.Name)
				b.WriteString(" " + p.accent + bold + cut(name, cols-2) + reset)
				if l.sec.Note != "" && cols > width(name)+6 {
					b.WriteString("  " + p.muted + cut(l.sec.Note, cols-width(name)-5) + reset)
				}
			case l.it != nil:
				kw := keysWidth(*l.sec, *l.it)
				b.WriteString("  " + renderKeys(p, *l.sec, *l.it))
				pad := max(descCol-2-kw, 2)
				b.WriteString(strings.Repeat(" ", pad))
				room := cols - 2 - kw - pad - 1
				desc := cut(l.it.Desc, room)
				b.WriteString(p.bright + desc + reset)
				room -= width(desc)
				if l.it.Note != "" && room > 6 {
					b.WriteString(p.muted + "  " + cut(l.it.Note, room-2) + reset)
				}
			}
		} else if idx == 0 && len(v.lines) == 0 {
			b.WriteString("  " + p.muted + "nothing matches" + reset)
		}
		b.WriteString("\x1b[K\r\n")
	}

	b.WriteString(p.line + strings.Repeat("─", cols) + reset + "\x1b[K\r\n")
	hints := "↑↓ pgup/pgdn scroll · tab next section · esc clear/quit"
	b.WriteString(" " + p.muted + cut(hints, cols-2) + reset + "\x1b[K")
	_ = rows
	os.Stdout.WriteString(b.String())
}

// scroll so the next (or previous) section header is at the top
func (v *view) jump(dir int) {
	for i := v.scroll + dir; i >= 0 && i < len(v.lines); i += dir {
		if v.lines[i].header {
			v.scroll = i
			return
		}
	}
	if dir < 0 {
		v.scroll = 0
	}
}

// ── plain output ──

func plain(d *data) {
	for i, s := range d.Sections {
		if i > 0 {
			fmt.Println()
		}
		fmt.Println(s.Name)
		for _, it := range s.Items {
			k := strings.Join(it.Keys, "+")
			if s.Style == "cmd" {
				k = strings.Join(it.Keys, " ")
			}
			line := fmt.Sprintf("  %-26s %s", k, it.Desc)
			if it.Note != "" {
				line += "  (" + it.Note + ")"
			}
			fmt.Println(line)
		}
	}
}

// ── main ──

func main() {
	path := dataPath
	if len(os.Args) > 1 {
		path = os.Args[1]
	}
	raw, err := os.ReadFile(path)
	if err != nil {
		fmt.Fprintln(os.Stderr, "nxshortcuts:", err)
		os.Exit(1)
	}
	var d data
	if err := json.Unmarshal(raw, &d); err != nil {
		fmt.Fprintln(os.Stderr, "nxshortcuts: bad shortcut list:", err)
		os.Exit(1)
	}

	if !isTerminal(os.Stdout) || !isTerminal(os.Stdin) {
		plain(&d)
		return
	}

	c := d.Colors
	p := palette{
		accent: fgc(pick(c, "accent", "e5a440")),
		bright: fgc(pick(c, "fgBright", "ede0c8")),
		fg:     fgc(pick(c, "fg", "d4b07b")),
		muted:  fgc(pick(c, "muted", "87765d")),
		capBg:  bgc(pick(c, "surface", "473f31")),
		line:   fgc(pick(c, "surface", "473f31")),
	}

	restore, err := rawMode()
	if err != nil {
		plain(&d)
		return
	}
	os.Stdout.WriteString("\x1b[?1049h\x1b[?25l\x1b[2J")
	quit := func() {
		os.Stdout.WriteString(reset + "\x1b[?25h\x1b[?1049l")
		restore()
	}
	defer quit()

	v := &view{d: &d, p: p}
	for _, s := range d.Sections {
		for _, it := range s.Items {
			v.keyCol = max(v.keyCol, keysWidth(s, it))
		}
	}
	v.keyCol = min(v.keyCol, 28)
	v.refilter()
	v.draw()

	keys := make(chan []byte)
	go func() {
		buf := make([]byte, 64)
		for {
			n, err := os.Stdin.Read(buf)
			if err != nil {
				close(keys)
				return
			}
			keys <- append([]byte(nil), buf[:n]...)
		}
	}()
	sig := make(chan os.Signal, 1)
	signal.Notify(sig, syscall.SIGWINCH, syscall.SIGTERM, syscall.SIGHUP)

	for {
		select {
		case s := <-sig:
			if s != syscall.SIGWINCH {
				return
			}
			os.Stdout.WriteString("\x1b[2J")
		case k, ok := <-keys:
			if !ok {
				return
			}
			for _, key := range split(k) {
				if !v.handle(key) {
					return
				}
			}
		}
		v.draw()
	}
}

// handle returns false to quit
func (v *view) handle(k []byte) bool {
	page := v.height() - 1
	switch string(k) {
	case "\x03", "\x04": // Ctrl+C, Ctrl+D
		return false
	case "\x1b": // Esc
		if v.query == "" {
			return false
		}
		v.query = ""
		v.refilter()
	case "\x1b[A", "\x1bOA", "\x10": // up, Ctrl+P
		v.scroll--
	case "\x1b[B", "\x1bOB", "\x0e": // down, Ctrl+N
		v.scroll++
	case "\x1b[5~":
		v.scroll -= page
	case "\x1b[6~", " ":
		if string(k) == " " && v.query != "" {
			v.typed(k)
			break
		}
		v.scroll += page
	case "\x1b[H", "\x1b[1~", "\x1bOH":
		v.scroll = 0
	case "\x1b[F", "\x1b[4~", "\x1bOF":
		v.scroll = len(v.lines)
	case "\t":
		v.jump(1)
	case "\x1b[Z":
		v.jump(-1)
	case "\x7f", "\x08": // backspace
		if r := []rune(v.query); len(r) > 0 {
			v.query = string(r[:len(r)-1])
			v.refilter()
		}
	case "\x15": // Ctrl+U
		v.query = ""
		v.refilter()
	case "\x17": // Ctrl+W: delete the last word
		q := strings.TrimRightFunc(v.query, unicode.IsSpace)
		if i := strings.LastIndexFunc(q, unicode.IsSpace); i >= 0 {
			v.query = q[:i+1]
		} else {
			v.query = ""
		}
		v.refilter()
	default:
		v.typed(k)
	}
	v.clamp()
	return true
}

// split a read into separate keys: escape sequences, or single characters
// (several keys can arrive in one read, e.g. a paste or a held key)
func split(b []byte) [][]byte {
	var out [][]byte
	for len(b) > 0 {
		n := 1
		if b[0] == 0x1b && len(b) > 2 && (b[1] == '[' || b[1] == 'O') {
			n = 2
			for n < len(b) && (b[n] < 0x40 || b[n] > 0x7e) {
				n++
			}
			n = min(n+1, len(b))
		} else if b[0] >= 0x80 {
			_, size := utf8.DecodeRune(b)
			n = size
		}
		out = append(out, b[:n])
		b = b[n:]
	}
	return out
}

func (v *view) typed(k []byte) {
	if len(k) == 0 || k[0] == 0x1b {
		return // unknown escape sequence
	}
	s := string(k)
	for _, r := range s {
		if !unicode.IsPrint(r) {
			return
		}
	}
	v.query += s
	v.refilter()
}
