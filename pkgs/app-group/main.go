// app-group puts related apps into tabbed groups when they open.
//
// Each group is a list of apps. When a window from a group opens tiled,
// it is moved into the tabbed group the other windows of that group are
// in (on whatever workspace that is). If another window from the group is
// open but not grouped yet, a tabbed group is made around it first. This
// only happens when a window opens; after that the windows are left alone,
// so they can be dragged out of the group.
//
// Usage: app-group NAME=APP,APP... [NAME=APP,APP...]...
// APP is a Wayland app_id or XWayland class, matched case-insensitively.
//
// Grouping is paused while the flag file $XDG_STATE_HOME/app-group/off
// exists (toggled by app-group-toggle).
//
// Talks to sway directly over its IPC socket and sleeps until sway sends
// a window event, so it uses no CPU while idle.
package main

import (
	"encoding/binary"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net"
	"os"
	"strings"
)

// sway IPC (same protocol as i3): "i3-ipc" + uint32 length + uint32 type.
const (
	ipcRunCommand  = 0
	ipcSubscribe   = 2
	ipcGetTree     = 4
	ipcWindowEvent = 0x80000003
)

func ipcDial() net.Conn {
	c, err := net.Dial("unix", os.Getenv("SWAYSOCK"))
	if err != nil {
		log.Fatalf("connect to sway: %v", err)
	}
	return c
}

func ipcSend(c net.Conn, typ uint32, payload string) {
	buf := make([]byte, 14+len(payload))
	copy(buf, "i3-ipc")
	binary.LittleEndian.PutUint32(buf[6:], uint32(len(payload)))
	binary.LittleEndian.PutUint32(buf[10:], typ)
	copy(buf[14:], payload)
	if _, err := c.Write(buf); err != nil {
		log.Fatalf("sway ipc write: %v", err) // systemd restarts us
	}
}

func ipcRecv(c net.Conn) (uint32, []byte) {
	head := make([]byte, 14)
	if _, err := io.ReadFull(c, head); err != nil {
		log.Fatalf("sway ipc read: %v", err) // systemd restarts us
	}
	body := make([]byte, binary.LittleEndian.Uint32(head[6:]))
	if _, err := io.ReadFull(c, body); err != nil {
		log.Fatalf("sway ipc read: %v", err)
	}
	return binary.LittleEndian.Uint32(head[10:]), body
}

type node struct {
	ID       int64  `json:"id"`
	Type     string `json:"type"`
	Name     string `json:"name"`
	Layout   string `json:"layout"`
	Focused  bool   `json:"focused"`
	AppID    string `json:"app_id"`
	WinProps struct {
		Class string `json:"class"`
	} `json:"window_properties"`
	Nodes         []node `json:"nodes"`
	FloatingNodes []node `json:"floating_nodes"`
}

func (n *node) app() string {
	if n.AppID != "" {
		return n.AppID
	}
	return n.WinProps.Class
}

type windowEvent struct {
	Change    string `json:"change"`
	Container node   `json:"container"`
}

// groupOf maps a lower-cased app name to its group name.
var groupOf = map[string]string{}

func group(n *node) string { return groupOf[strings.ToLower(n.app())] }

// paused reports whether the off flag file exists.
func paused() bool {
	dir := os.Getenv("XDG_STATE_HOME")
	if dir == "" {
		dir = os.Getenv("HOME") + "/.local/state"
	}
	_, err := os.Stat(dir + "/app-group/off")
	return err == nil
}

// target is a window to join, and whether it is already in a group.
type target struct {
	id      int64
	grouped bool
}

// findTarget looks for another tiled window from group g, preferring one
// that is already in a tabbed or stacked container.
func findTarget(root *node, skip int64, g string) (target, bool) {
	var best target
	found := false
	var walk func(n *node)
	walk = func(n *node) {
		if n.Type == "workspace" && n.Name == "__i3_scratch" {
			return
		}
		for i := range n.Nodes { // tiled children only
			c := &n.Nodes[i]
			if c.ID != skip && len(c.Nodes) == 0 && group(c) == g {
				grouped := n.Layout == "tabbed" || n.Layout == "stacked"
				if !found || (grouped && !best.grouped) {
					best, found = target{c.ID, grouped}, true
				}
			}
			walk(c)
		}
	}
	walk(root)
	return best, found
}

func main() {
	for _, arg := range os.Args[1:] {
		name, list, ok := strings.Cut(arg, "=")
		if !ok || name == "" || list == "" {
			log.Fatalf("bad group %q, expected NAME=APP,APP...", arg)
		}
		for _, a := range strings.Split(list, ",") {
			groupOf[strings.ToLower(strings.TrimSpace(a))] = name
		}
	}
	if len(groupOf) == 0 {
		log.Fatal("usage: app-group NAME=APP,APP... [NAME=APP,APP...]...")
	}

	cmd := ipcDial()
	sub := ipcDial()
	ipcSend(sub, ipcSubscribe, `["window"]`)

	for {
		typ, body := ipcRecv(sub)
		if typ != ipcWindowEvent {
			continue // the subscribe reply
		}
		var ev windowEvent
		if err := json.Unmarshal(body, &ev); err != nil || ev.Change != "new" {
			continue
		}
		w := ev.Container
		g := group(&w)
		if w.Type != "con" || g == "" || paused() { // floating windows are left alone
			continue
		}

		ipcSend(cmd, ipcGetTree, "")
		_, treeJSON := ipcRecv(cmd)
		var root node
		if err := json.Unmarshal(treeJSON, &root); err != nil {
			continue
		}
		t, ok := findTarget(&root, w.ID, g)
		if !ok {
			continue // first window of this group: nothing to join yet
		}

		mark := "__app_group_" + g
		var b strings.Builder
		if !t.grouped {
			// wrap the existing window in its own tabbed container
			fmt.Fprintf(&b, "[con_id=%d] splitv; [con_id=%d] layout tabbed; ", t.id, t.id)
		}
		fmt.Fprintf(&b, "[con_id=%d] mark --add %s; ", t.id, mark)
		fmt.Fprintf(&b, "[con_id=%d] move container to mark %s; ", w.ID, mark)
		fmt.Fprintf(&b, "[con_id=%d] unmark %s", t.id, mark)
		if w.Focused {
			fmt.Fprintf(&b, "; [con_id=%d] focus", w.ID)
		}
		ipcSend(cmd, ipcRunCommand, b.String())
		ipcRecv(cmd)
	}
}
