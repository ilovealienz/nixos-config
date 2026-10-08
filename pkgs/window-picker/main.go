// window-picker lists every sway window in fuzzel: tiled, floating,
// scratchpad, on every workspace. Urgent windows come first and the
// focused window last.
//
//	Enter        focus the window (sway leaves fullscreen or shows the
//	             scratchpad if that's needed to show it)
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

// windows returns every window, urgent first and the focused one last.
func windows(c net.Conn) []window {
	var root node
	if err := json.Unmarshal(ipc(c, ipcGetTree, ""), &root); err != nil {
		log.Fatalf("parse tree: %v", err)
	}
	var urgent, normal, focused []window
	var walk func(n *node, ws string, floating bool)
	walk = func(n *node, ws string, floating bool) {
		if n.Type == "workspace" {
			ws = n.Name
			if ws == "__i3_scratch" {
				ws = "scratch"
			}
		}
		if n.PID != 0 && (n.Type == "con" || n.Type == "floating_con") {
			app := n.AppID
			if app == "" {
				app = n.WinProps.Class
			}
			w := window{n.ID, n.PID, ws, app, n.Name, n.Focused, n.Urgent, floating}
			switch {
			case w.urgent:
				urgent = append(urgent, w)
			case w.focused:
				focused = append(focused, w)
			default:
				normal = append(normal, w)
			}
		}
		for i := range n.Nodes {
			walk(&n.Nodes[i], ws, floating)
		}
		for i := range n.FloatingNodes {
			walk(&n.FloatingNodes[i], ws, true)
		}
	}
	walk(&root, "", false)
	return append(append(urgent, normal...), focused...)
}

func (w window) row() string {
	app := w.app
	if strings.Count(app, ".") >= 2 { // org.foo.Bar -> Bar
		app = app[strings.LastIndex(app, ".")+1:]
	}
	if r := []rune(app); len(r) > 16 {
		app = string(r[:16])
	}
	title := w.title
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
	row := fmt.Sprintf("%-7s %-16s %s", w.ws, app, title)
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
	{"force kill", true, func(c net.Conn, w window) { _ = syscall.Kill(w.pid, syscall.SIGKILL) }},
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
		ws := windows(c)
		if len(ws) == 0 {
			return
		}
		rows := make([]string, len(ws))
		for i, w := range ws {
			rows[i] = w.row()
		}
		i, rc := pick(rows, "window: ", "enter: go to it · shift+enter: more")
		if i < 0 {
			return // Escape
		}
		w := ws[i]
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
