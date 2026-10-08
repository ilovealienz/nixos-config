// window-picker lists every sway window in fuzzel, grouped under a
// header per workspace: tiled, floating and scratchpad windows.
//
//	Enter        focus the window (sway leaves fullscreen or shows the
//	             scratchpad if that's needed to show it); on a header,
//	             go to that workspace
//	Shift+Enter  open an action menu for the window
//
// Floating windows end with "[floating]", urgent ones with "!!!".
//
// fuzzel and its config are set at build time (-X main.fuzzel=...,
// -X main.config=...). The config must bind custom-1 to Shift+Return.
package main

import (
	"bytes"
	"encoding/binary"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"log"
	"net"
	"os"
	"os/exec"
	"sort"
	"strconv"
	"strings"
	"syscall"
	"time"
)

var (
	fuzzel = "fuzzel"
	config = ""
)

// sway IPC (same protocol as i3): "i3-ipc" + uint32 length + uint32 type.
const (
	ipcRunCommand = 0
	ipcGetTree    = 4
)

func ipc(c net.Conn, typ uint32, payload string) []byte {
	buf := make([]byte, 14+len(payload))
	copy(buf, "i3-ipc")
	binary.LittleEndian.PutUint32(buf[6:], uint32(len(payload)))
	binary.LittleEndian.PutUint32(buf[10:], typ)
	copy(buf[14:], payload)
	if _, err := c.Write(buf); err != nil {
		log.Fatalf("sway ipc write: %v", err)
	}
	head := make([]byte, 14)
	if _, err := io.ReadFull(c, head); err != nil {
		log.Fatalf("sway ipc read: %v", err)
	}
	body := make([]byte, binary.LittleEndian.Uint32(head[6:]))
	if _, err := io.ReadFull(c, body); err != nil {
		log.Fatalf("sway ipc read: %v", err)
	}
	return body
}

func run(c net.Conn, format string, args ...any) {
	ipc(c, ipcRunCommand, fmt.Sprintf(format, args...))
}

type node struct {
	ID       int64  `json:"id"`
	Type     string `json:"type"`
	Name     string `json:"name"`
	PID      int    `json:"pid"`
	Focused  bool   `json:"focused"`
	Urgent   bool   `json:"urgent"`
	Shell    string `json:"shell"` // set only on windows
	AppID    string `json:"app_id"`
	WinProps struct {
		Class string `json:"class"`
	} `json:"window_properties"`
	Nodes         []node `json:"nodes"`
	FloatingNodes []node `json:"floating_nodes"`
}

type window struct {
	id                        int64
	pid                       int
	ws, app, title            string
	focused, urgent, floating bool
}

// workspaceGroup is one workspace and its windows, in tree order.
type workspaceGroup struct {
	name    string
	windows []window
}

// workspaces returns every workspace that has windows, in sway's order
// (numbered first), with the scratchpad last.
func workspaces(c net.Conn) []workspaceGroup {
	var root node
	if err := json.Unmarshal(ipc(c, ipcGetTree, ""), &root); err != nil {
		log.Fatalf("parse tree: %v", err)
	}
	var groups []workspaceGroup
	var scratch *workspaceGroup
	var walk func(n *node, g *workspaceGroup, floating bool)
	walk = func(n *node, g *workspaceGroup, floating bool) {
		if n.Shell != "" && (n.Type == "con" || n.Type == "floating_con") {
			app := n.AppID
			if app == "" {
				app = n.WinProps.Class
			}
			g.windows = append(g.windows, window{n.ID, n.PID, g.name, app, n.Name, n.Focused, n.Urgent, floating})
		}
		for i := range n.Nodes {
			walk(&n.Nodes[i], g, floating)
		}
		for i := range n.FloatingNodes {
			walk(&n.FloatingNodes[i], g, true)
		}
	}
	for i := range root.Nodes { // outputs
		for j := range root.Nodes[i].Nodes { // workspaces
			ws := &root.Nodes[i].Nodes[j]
			g := workspaceGroup{name: ws.Name}
			if ws.Name == "__i3_scratch" {
				g.name = "scratch"
			}
			walk(ws, &g, false)
			if len(g.windows) == 0 {
				continue
			}
			if g.name == "scratch" {
				scratch = &g
			} else {
				groups = append(groups, g)
			}
		}
	}
	sort.SliceStable(groups, func(a, b int) bool { return wsLess(groups[a].name, groups[b].name) })
	if scratch != nil {
		groups = append(groups, *scratch)
	}
	return groups
}

// wsLess orders numbered workspaces numerically, then named ones by name.
func wsLess(a, b string) bool {
	na, ea := strconv.Atoi(a)
	nb, eb := strconv.Atoi(b)
	switch {
	case ea == nil && eb == nil:
		return na < nb
	case ea == nil:
		return true
	case eb == nil:
		return false
	}
	return a < b
}

func (w window) row() string {
	app := w.app
	if strings.Count(app, ".") >= 2 { // org.foo.Bar -> Bar
		app = app[strings.LastIndex(app, ".")+1:]
	}
	if r := []rune(app); len(r) > 16 {
		app = string(r[:16])
	}
	title := strings.NewReplacer("\n", " ", "\r", " ").Replace(w.title)
	if title == "" {
		title = "(no title)"
	}
	var tags []string
	if w.floating && w.ws != "scratch" { // scratchpad windows always float
		tags = append(tags, "[floating]")
	}
	if w.urgent {
		tags = append(tags, "!!!")
	}
	row := fmt.Sprintf("   %-16s %s", app, title)
	if len(tags) > 0 {
		row += "   " + strings.Join(tags, " ")
	}
	return row
}

// pick shows lines in fuzzel. It returns the chosen index (-1 if
// cancelled) and fuzzel's exit code (10 = custom-1, i.e. Shift+Enter).
func pick(lines []string, prompt, placeholder string) (int, int) {
	args := []string{"--dmenu", "--index", "--prompt", prompt, "--width", "90"}
	if config != "" {
		args = append([]string{"--config", config}, args...)
	}
	if placeholder != "" {
		args = append(args, "--placeholder", placeholder)
	}
	cmd := exec.Command(fuzzel, args...)
	cmd.Stdin = strings.NewReader(strings.Join(lines, "\n") + "\n")
	var out bytes.Buffer
	cmd.Stdout = &out
	cmd.Stderr = os.Stderr
	rc := 0
	if err := cmd.Run(); err != nil {
		var ee *exec.ExitError
		if !errors.As(err, &ee) {
			log.Fatalf("run fuzzel: %v", err)
		}
		rc = ee.ExitCode()
	}
	i, err := strconv.Atoi(strings.TrimSpace(out.String()))
	if err != nil || i < 0 || i >= len(lines) {
		return -1, rc
	}
	return i, rc
}

type action struct {
	label  string
	reopen bool // show the window list again afterwards
	do     func(c net.Conn, w window)
}

var actions = []action{
	{"focus", false, func(c net.Conn, w window) { run(c, "[con_id=%d] focus", w.id) }},
	{"bring here", false, func(c net.Conn, w window) {
		run(c, "[con_id=%d] move container to workspace current; [con_id=%d] focus", w.id, w.id)
	}},
	{"toggle floating", true, func(c net.Conn, w window) { run(c, "[con_id=%d] floating toggle", w.id) }},
	{"send to scratchpad", true, func(c net.Conn, w window) { run(c, "[con_id=%d] move scratchpad", w.id) }},
	{"close", true, func(c net.Conn, w window) { run(c, "[con_id=%d] kill", w.id) }},
	{"force kill", true, func(c net.Conn, w window) {
		// the list may be stale: only kill if the window is still there
		// with the same pid, so a reused pid is never hit
		for _, g := range workspaces(c) {
			for _, now := range g.windows {
				if now.id != w.id {
					continue
				}
				if now.pid > 1 && now.pid == w.pid {
					_ = syscall.Kill(now.pid, syscall.SIGKILL)
				} else { // pid unknown: ask the app to close instead
					run(c, "[con_id=%d] kill", now.id)
				}
				return
			}
		}
	}},
}

func main() {
	c, err := net.Dial("unix", os.Getenv("SWAYSOCK"))
	if err != nil {
		log.Fatalf("connect to sway: %v", err)
	}
	labels := make([]string, len(actions))
	for i, a := range actions {
		labels[i] = a.label
	}

	for {
		// one line per header and window; entries[i] is nil for a header
		var lines []string
		var entries []*window
		var headers []string
		var firstScratch int64
		for _, g := range workspaces(c) {
			lines = append(lines, "── "+g.name+" "+strings.Repeat("─", max(2, 40-len(g.name))))
			entries = append(entries, nil)
			headers = append(headers, g.name)
			if g.name == "scratch" {
				firstScratch = g.windows[0].id
			}
			for i := range g.windows {
				lines = append(lines, g.windows[i].row())
				entries = append(entries, &g.windows[i])
				headers = append(headers, g.name)
			}
		}
		if len(lines) == 0 {
			return
		}
		i, rc := pick(lines, "window: ", "enter: go to it · shift+enter: more")
		if i < 0 {
			return // Escape
		}
		if entries[i] == nil { // header: go to the workspace
			if headers[i] == "scratch" {
				// focus shows it; "scratchpad show" would toggle instead
				run(c, "[con_id=%d] focus", firstScratch)
			} else {
				run(c, "workspace %q", headers[i])
			}
			return
		}
		w := *entries[i]
		if rc != 10 { // plain Enter
			actions[0].do(c, w)
			return
		}
		a, _ := pick(labels, w.app+": ", "")
		if a < 0 {
			continue // Escape in the menu: back to the list
		}
		actions[a].do(c, w)
		if !actions[a].reopen {
			return
		}
		time.Sleep(300 * time.Millisecond) // let sway update before listing again
	}
}
