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
      (k [ "Click idle sliver" ] "Stay awake" "amber: no lock or screen-off")
      (k [ "Right-click idle sliver" ] "Game mode" "green: stay awake, GameMode on")
    ]; }
    { name = "Launch"; items = [
      (k [ "Super" "Return" ] "Kitty" "jumps to workspace 4 while placement is on")
      (k [ "Super" "Shift" "Return" ] "Foot" "lighter terminal, no images; workspace 4 too")
      (k [ "Super" "E" ] "Thunar" "jumps to workspace 5 while placement is on")
      (k [ "Super" "R" ] "App launcher" "fuzzel")
      (k [ "Super" "Shift" "R" ] "Run a command" "wmenu-run")
      (k [ "Super" ";" ] "Emoji picker" "Enter types it, Shift+Enter copies it")
      (k [ "Super" "A" ] "Audio menu" "outputs, mics, mute, hide, rename")
      (k [ "Super" "Shift" "P" ] "Power menu" "profile, battery limit; or click the battery")
      (k [ "Super" "/" ] "This list" "")
    ]; }
    { name = "Windows"; items = [
      (k [ "Super" "Q" ] "Close window" "")
      (k [ "Super" "F" ] "Fullscreen" "floats Proton and Wine games first so they stay fullscreen")
      (k [ "Super" "V" ] "Float / tile" "")
      (k [ "Super" "Alt" "V" ] "Popout window" "small, bottom-right, on every workspace; again: back to the corner, then back to tiling")
      (k [ "Super" "S" ] "Split direction" "next window opens beside or below")
      (k [ "Super" "T" ] "Toggle split layout" "")
      (k [ "Super" "Shift" "Tab" ] "Window picker" "every window, grouped by workspace")
    ]; }
    { name = "Focus, move, resize"; note = "vim keys"; items = [
      (k [ "Super" "H/J/K/L" ] "Focus left / down / up / right" "")
      (k [ "Super" "Shift" "H/L" ] "Move window left / right" "")
      (k [ "Super" "Shift" "J/K" ] "Move window down / up" "steps out of a tab group")
      (k [ "Super" "Alt" "H/L" ] "Move the window edge left / right" "40px")
      (k [ "Super" "Alt" "K/J" ] "Move the window edge up / down" "40px")
    ]; }
    { name = "Tab groups"; note = "one left: back to a split (at once on Super+Q, 10s after moving out)"; items = [
      (k [ "Super" "G" ] "Group with neighbours" "or turn a group back into a split")
      (k [ "Super" "Tab" ] "Next tab in the group" "wraps around")
      (k [ "Super" "Shift" "G" ] "Tabbed / stacked" "")
      (k [ "Super" "Shift" "Drag" ] "Move the whole group" "drag any window of it; the rest follow when you drop it")
    ]; }
    { name = "Workspaces & monitors"; items = [
      (k [ "Super" "1-9" ] "Go to workspace" "")
      (k [ "Super" "Shift" "1-9" ] "Send window to workspace" "")
      (k [ "Super" "Shift" ",/." ] "Move workspace to left / right monitor" "")
    ]; }
    { name = "App placement"; note = "while Super+Alt+P is on"; items = [
      (k [ "1" ] "LibreWolf, Brave, Firefox" "")
      (k [ "2" ] "Spotify, Signal, Vesktop" "")
      (k [ "3" ] "mpv" "")
      (k [ "4" ] "kitty, foot" "")
      (k [ "5" ] "Thunar" "")
      (k [ "6" ] "qBittorrent" "")
      (k [ "7" ] "virt-manager" "")
    ]; }
    { name = "Screen"; items = [
      (k [ "Print" ] "Screenshot menu" "freezes the screen, then annotate, upload, copy, save")
      (k [ "Super" "Shift" "S" ] "Region to clipboard" "")
    ]; }
    { name = "Session"; items = [
      (k [ "Super" "Shift" "X" ] "Lock screen" "")
      (k [ "Super" "Shift" "E" ] "Session menu" "shut down, reboot, log out, suspend, lock")
      (k [ "Super" "Shift" "C" ] "Reload sway" "")
    ]; }
    { name = "msqu"; note = "timer, stopwatch, alarm, repeat, drink reminder"; items = [
      (k [ "Super" "Shift" "A" ] "msqu: quick utils" "start one, or pick from recents")
      (k [ "Click a block" ] "Pause / resume" "")
      (k [ "Right-click a block" ] "Stop or cancel it" "")
      (k [ "Middle-click timer" ] "More or less time" "pick in fuzzel, or type +3, -2, 90s, 1:30")
      (k [ "Middle-click repeat" ] "Start the wait over" "")
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
      (k [ "battery" ] "Click: power menu" "")
      (k [ "clock" ] "Right-click: month / year" "scroll: change month")
      (k [ "dnd" ] "Click: history" "right-click: on / off")
      (k [ "birthday" ] "Click: coming up" "right-click: hide until next login; names in ~/.config/birthdays")
      (k [ "idle" ] "Click: stay awake on / off" "right-click: game mode")
      (k [ "rec" ] "Click: stop recording" "")
    ]; }
    { name = "nx commands"; note = "zsh"; style = "cmd"; items = [
      (k [ "nxrebuild" ] "Rebuild and switch" "")
      (k [ "nxupdate" ] "Update flake inputs, then rebuild" "")
      (k [ "nxpull" ] "Pull the config from GitHub, then rebuild" "drops an unpushed local flake.lock first")
      (k [ "nxpush" ] "Check, commit and push" "nxpush \"message\" (default: update); never commits hardware-configuration.nix")
      (k [ "nxcheck" ] "Check every host still builds" "evaluates pc, laptop, x1carbon without building")
      (k [ "nxclean" ] "Clean old generations, keep 3" "")
      (k [ "nxedit" ] "Pick a config file" "Enter edit · Ctrl+R rebuild · Ctrl+A git add")
      (k [ "theme-edit" ] "Theme editor" "colours, icons and wallpaper, with a preview")
      (k [ "nxshortcuts" ] "This list" "")
      (k [ "nxsearch" ] "Search nixpkgs" "")
      (k [ "nxsrun" ] "Search nixpkgs and run the pick" "")
      (k [ "nxrun <pkg>" ] "Run a package without installing it" "")
      (k [ "ldrun <bin>" ] "Run a generic Linux binary" "")
      (k [ "fpup" ] "Update flatpaks" "")
      (k [ "nxapps" ] "Update the apps in ~/.bin" "uwuplsplay, stremio-cliuwu, zipline-upload; also checked daily")
      (k [ "nxsecrets unpack" ] "Put the weather key and birthdays in place" "from secrets.age; asks for the passphrase (Bitwarden) once per machine. nxpush and nxpull keep them in sync")
      (k [ "nxsecrets rekey" ] "New key and passphrase for the secrets" "if the passphrase is forgotten or leaked; run on a machine that has unpacked")
      (k [ "nxport <port> [close]" ] "Open or close a firewall port" "")
      (k [ "nxaudio [-c N]" ] "alsamixer, saved on exit" "")
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

  # popup in a floating foot (the foot-float rule in sway.nix)
  wayland.windowManager.sway.config.keybindings."Mod4+slash" =
    "exec foot --app-id foot-float ${lib.getExe nxshortcuts}";
}
