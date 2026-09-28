// xwayland-attention marks XWayland windows urgent in sway when they ask
// for attention, and shows how many times they asked in the title.
//
// Wine (and other X11 apps) request attention by sending a _NET_WM_STATE
// client message that adds _NET_WM_STATE_DEMANDS_ATTENTION. Sway ignores
// that, so this listens for it on the XWayland root window and instead:
//   - sets its title to "(N) <title>", counting requests like Discord does
//
// Focusing the window resets the count and restores the normal title.
// Requests from the window you're already looking at are ignored.
// Talks to sway directly over its IPC socket (no swaymsg processes).
// Everything is event-driven, so it uses no CPU while idle.
package main

import (
	"encoding/binary"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net"
	"os"
	"sync"

	"github.com/jezek/xgb"
	"github.com/jezek/xgb/xproto"
)

const (
	actionAdd    = 1
	actionToggle = 2
)

// sway IPC (same protocol as i3): "i3-ipc" + uint32 length + uint32 type.
const (
	ipcRunCommand  = 0
	ipcSubscribe   = 2
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

// state shared between the X11 listener and the sway listener.
type state struct {
	mu      sync.Mutex
	cmd     net.Conn       // sway connection for commands
	counts  map[uint32]int // X11 window id -> attention requests
	focused uint32         // X11 id of the focused window (0 if not X11)
}

// run sends a sway command and waits for its reply. Caller holds s.mu.
func (s *state) run(cmd string) {
	ipcSend(s.cmd, ipcRunCommand, cmd)
	ipcRecv(s.cmd)
}

// attention is called for every attention request from window wid.
func (s *state) attention(wid uint32) {
	s.mu.Lock()
	if wid == s.focused {
		s.mu.Unlock()
		return
	}
	s.counts[wid]++
	s.run(fmt.Sprintf(`[id=%d] title_format "(%d) %%title"`, wid, s.counts[wid]))
	s.mu.Unlock()
}

// focus is called whenever sway focuses a window (wid 0 = not X11).
func (s *state) focus(wid uint32) {
	s.mu.Lock()
	s.focused = wid
	if _, had := s.counts[wid]; had {
		delete(s.counts, wid)
		s.run(fmt.Sprintf(`[id=%d] title_format "%%title"`, wid))
	}
	s.mu.Unlock()
}

// closed forgets a window that went away.
func (s *state) closed(wid uint32) {
	s.mu.Lock()
	delete(s.counts, wid)
	s.mu.Unlock()
}

type windowEvent struct {
	Change    string `json:"change"`
	Container struct {
		Window *uint32 `json:"window"`
	} `json:"container"`
}

// watchSway follows sway's window events to track focus and closes.
func watchSway(s *state) {
	c := ipcDial()
	ipcSend(c, ipcSubscribe, `["window"]`)
	for {
		typ, body := ipcRecv(c)
		if typ != ipcWindowEvent {
			continue // the subscribe reply
		}
		var ev windowEvent
		if err := json.Unmarshal(body, &ev); err != nil {
			continue
		}
		var wid uint32
		if ev.Container.Window != nil {
			wid = *ev.Container.Window
		}
		switch ev.Change {
		case "focus":
			s.focus(wid)
		case "close":
			if wid != 0 {
				s.closed(wid)
			}
		}
	}
}

func atom(conn *xgb.Conn, name string) xproto.Atom {
	reply, err := xproto.InternAtom(conn, false, uint16(len(name)), name).Reply()
	if err != nil {
		log.Fatalf("intern %s: %v", name, err)
	}
	return reply.Atom
}

func main() {
	s := &state{counts: map[uint32]int{}, cmd: ipcDial()}

	conn, err := xgb.NewConn()
	if err != nil {
		log.Fatalf("connect to X: %v", err)
	}
	defer conn.Close()

	root := xproto.Setup(conn).DefaultScreen(conn).Root
	err = xproto.ChangeWindowAttributesChecked(conn, root, xproto.CwEventMask,
		[]uint32{xproto.EventMaskSubstructureNotify}).Check()
	if err != nil {
		log.Fatalf("listen on root window: %v", err)
	}

	netWMState := atom(conn, "_NET_WM_STATE")
	attention := atom(conn, "_NET_WM_STATE_DEMANDS_ATTENTION")

	go watchSway(s)

	for {
		ev, xerr := conn.WaitForEvent()
		if ev == nil && xerr == nil {
			log.Fatal("X connection closed") // non-zero exit -> systemd restarts us
		}
		msg, ok := ev.(xproto.ClientMessageEvent)
		if !ok || msg.Type != netWMState || msg.Format != 32 {
			continue
		}
		d := msg.Data.Data32
		if (d[0] == actionAdd || d[0] == actionToggle) &&
			(xproto.Atom(d[1]) == attention || xproto.Atom(d[2]) == attention) {
			s.attention(uint32(msg.Window))
		}
	}
}
