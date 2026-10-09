# nxshortcuts: searchable list of the desktop's shortcuts, in the terminal
# (`nxshortcuts`) or as a popup on Super+/. The list below is the one place
# they're written down; when you add or change a binding, update it here.
# `nxshortcuts | grep …` prints plain text.
# Source: ../pkgs/nxshortcuts/main.go
{ pkgs, lib, osConfig, ... }:
let
  # [ keys ] "what it does" "extra detail (optional)"
  k = keys: desc: note: { inherit keys desc note; };

  sections = [
    { name = "Toggles"; note = "press again to switch back"; items = [
      (k [ "Super" "Alt" "P" ] "Window placement" "off: apps open on the workspace you're on")
      (k [ "Super" "Alt" "G" ] "App grouping" "off: related apps stop joining one tab group")
      (k [ "Super" "U" ] "Urgent marking" "red border when IMVU, Steam chats… want attention")
      (k [ "Shift" "Print" ] "Screen recording" "or click the red dot in waybar to stop")
      (k [ "Right-click DND" ] "Do not disturb" "the DND icon in waybar; left-click shows history")
      (k [ "Click idle icon" ] "Stay awake" "waybar idle inhibitor: no lock or screen-off")
    ]; }
    { name = "Launch"; items = [
      (k [ "Super" "Return" ] "Kitty" "jumps to workspace 4 while placement is on")
      (k [ "Super" "E" ] "Thunar" "jumps to workspace 5 while placement is on")
      (k [ "Super" "R" ] "App launcher" "fuzzel")
      (k [ "Super" "Shift" "R" ] "Run a command" "wmenu-run")
      (k [ "Super" ";" ] "Emoji picker" "Enter types it, Shift+Enter copies it")
      (k [ "Super" "A" ] "Audio menu" "outputs, mics, mute, hide, rename")
      (k [ "Super" "/" ] "This list" "")
      (k [ "Super" "Shift" "X" ] "Lock screen" "")
    ]; }
    { name = "Windows"; items = [
      (k [ "Super" "Q" ] "Close window" "")
      (k [ "Super" "F" ] "Fullscreen" "floats Proton and Wine games first so they stay fullscreen")
      (k [ "Super" "V" ] "Float / tile" "")
      (k [ "Super" "Shift" "Tab" ] "Window picker" "every window, grouped by workspace")
      (k [ "Super" "S" ] "Split direction" "next window opens beside or below")
      (k [ "Super" "T" ] "Toggle split layout" "")
      (k [ "Super" "Shift" "C" ] "Reload sway" "")
      (k [ "Super" "Shift" "E" ] "Exit sway" "asks first")
    ]; }
    { name = "Focus, move, resize"; note = "vim keys"; items = [
      (k [ "Super" "H/J/K/L" ] "Focus left / down / up / right" "")
      (k [ "Super" "Shift" "H/L" ] "Move window left / right" "")
      (k [ "Super" "Shift" "J/K" ] "Move window down / up" "steps out of a tab group")
      (k [ "Super" "Alt" "H/L" ] "Narrower / wider" "40px")
      (k [ "Super" "Alt" "K/J" ] "Shorter / taller" "40px")
    ]; }
    { name = "Tab groups"; items = [
      (k [ "Super" "G" ] "Group with neighbours" "or turn a group back into a split")
      (k [ "Super" "Tab" ] "Next tab in the group" "wraps around")
      (k [ "Super" "Shift" "G" ] "Tabbed / stacked" "")
    ]; }
    { name = "Workspaces & monitors"; items = [
      (k [ "Super" "1-9" ] "Go to workspace" "")
      (k [ "Super" "Shift" "1-9" ] "Send window to workspace" "")
      (k [ "Super" "Shift" ",/." ] "Move workspace to left / right monitor" "")
    ]; }
    { name = "App placement"; note = "while Super+Alt+P is on"; items = [
      (k [ "1" ] "Firefox, Brave, Floorp" "")
      (k [ "2" ] "Spotify, Signal, Vesktop" "")
      (k [ "3" ] "mpv" "")
      (k [ "4" ] "kitty" "")
      (k [ "5" ] "Thunar" "")
      (k [ "6" ] "qBittorrent" "")
      (k [ "7" ] "virt-manager" "")
    ]; }
    { name = "Screen"; items = [
      (k [ "Print" ] "Screenshot menu" "freezes the screen, then annotate, upload, copy, save")
      (k [ "Super" "Shift" "S" ] "Region to clipboard" "")
    ]; }
    { name = "Media keys"; items = [
      (k [ "Vol +/-" ] "Volume" "with on-screen level")
      (k [ "Mute" ] "Mute output" "")
      (k [ "Bright +/-" ] "Brightness" "")
      (k [ "Play/Next/Prev" ] "Media control" "whatever is playing")
    ]; }
    { name = "Window picker"; note = "inside Super+Shift+Tab"; items = [
      (k [ "Enter" ] "Go to the window" "on a header: go to that workspace")
      (k [ "Shift" "Enter" ] "Actions" "bring here, float, scratchpad, close, force kill")
    ]; }
    { name = "Audio menu"; note = "inside Super+A"; items = [
      (k [ "Enter" ] "Make it the default" "audio moves with it; on the current mic: mute")
      (k [ "Shift" "Enter" ] "Device options" "hide, rename, reset name")
    ]; }
    { name = "Emoji picker"; note = "inside Super+;"; items = [
      (k [ "Enter" ] "Type it into the window" "")
      (k [ "Shift" "Enter" ] "Copy only" "")
    ]; }
    { name = "Waybar"; style = "cmd"; items = [
      (k [ "volume" ] "Click: mute" "right-click: pavucontrol")
      (k [ "network" ] "Click: nmtui" "right-click: Bluetooth")
      (k [ "dnd" ] "Click: history" "right-click: on / off")
      (k [ "idle" ] "Click: stay awake on / off" "")
      (k [ "rec" ] "Click: stop recording" "")
    ]; }
    { name = "nx commands"; note = "zsh"; style = "cmd"; items = [
      (k [ "nxrebuild" ] "Rebuild and switch" "")
      (k [ "nxupdate" ] "Update flake inputs, then rebuild" "")
      (k [ "nxpull" ] "Pull the config from GitHub, then rebuild" "")
      (k [ "nxpush" ] "Commit everything as \"update\" and push" "")
      (k [ "nxedit" ] "Pick a config file" "Enter edit · Ctrl+R rebuild · Ctrl+A git add")
      (k [ "nxshortcuts" ] "This list" "")
      (k [ "nxsearch" ] "Search nixpkgs" "")
      (k [ "nxsrun" ] "Search nixpkgs and run the pick" "")
      (k [ "nxrun <pkg>" ] "Run a package without installing it" "")
      (k [ "nxclean" ] "Clean old generations, keep 3" "")
      (k [ "nxport <port> [close]" ] "Open or close a firewall port" "")
      (k [ "nxaudio [-c N]" ] "alsamixer, saved on exit" "")
      (k [ "ldrun <bin>" ] "Run a generic Linux binary" "")
      (k [ "fpup" ] "Update flatpaks" "")
      (k [ "v" ] "nvim" "")
    ]; }
  ];

  list = pkgs.writeText "shortcuts.json" (builtins.toJSON {
    colors = osConfig.theme.colors;
    inherit sections;
  });

  nxshortcuts = pkgs.buildGoModule {
    pname = "nxshortcuts";
    version = "1.0";
    src = ../pkgs/nxshortcuts;
    vendorHash = null;   # standard library only
    ldflags = [ "-s" "-w" "-X main.dataPath=${list}" ];
    meta.mainProgram = "nxshortcuts";
  };
in
{
  home.packages = [ nxshortcuts ];

  # popup in the floating kitty (kitty-float rule in compositor.nix)
  wayland.windowManager.sway.config.keybindings."Mod4+slash" =
    "exec kitty --class kitty-float -e ${lib.getExe nxshortcuts}";
}
