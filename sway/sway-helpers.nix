# Small additions to sway, in one Rust program (standard library only,
# ~2.5MB of RAM in total). Each runs in its own thread and is restarted on
# its own if it crashes.
#
#   group-drag  Super+Shift+drag moves a whole tab group: drag any window
#               of the group and the rest follows it once it's dropped, in
#               the same order. Super+drag still moves a single window.
#   app-group   related apps join one tabbed group when they open; after
#               that they're left alone. Super+Alt+g pauses/resumes it.
#   group-end   a tab group down to one window is removed, leaving a plain
#               window, so new windows open beside it rather than as tabs:
#               at once if the other was closed (Super+q), 10 seconds
#               later if it was moved out or floated, so it can be put back.
#   attention   XWayland apps (Wine/IMVU, Steam chats...) ask for attention
#               in a way sway ignores; the count is shown in the title,
#               e.g. "(3) the rock - IMVU", reset when you focus the window.
#               Super+U toggles also marking them urgent (red border + red
#               workspace). XWayland can still exit when no X11 app is open.
#
# Source: ../pkgs/sway-helpers/src/
{ pkgs, lib, ... }:
let
  # group name -> apps (Wayland app_id or XWayland class, case-insensitive).
  # To check a window: swaymsg -t get_tree | grep -E '"(app_id|class)"'
  groups = {
    chat = [ "vesktop" "signal" "ferdium" "org.telegram.desktop" "TelegramDesktop" ];
    minecraft = [ "org.prismlauncher.PrismLauncher" "com.mojang.minecraft" ];
  };

  # added after the title in every title attention sets; keep it the same
  # as the `title_format "%title [XWayland]"` rule in compositor.nix
  titleSuffix = " [XWayland]";

  swayHelpers = pkgs.rustPlatform.buildRustPackage {
    pname = "sway-helpers";
    version = "1.1.0";
    src = ../pkgs/sway-helpers;
    cargoLock.lockFile = ../pkgs/sway-helpers/Cargo.lock;   # no dependencies
  };

  # flag files the program checks; each toggle flips one
  toggle = name: flag: on: off: title: pkgs.writeShellApplication {
    inherit name;
    runtimeInputs = [ pkgs.coreutils pkgs.libnotify ];
    text = ''
      flag="''${XDG_STATE_HOME:-$HOME/.local/state}/${flag}"
      mkdir -p "$(dirname "$flag")"
      if [ -e "$flag" ]; then
        rm "$flag"; state="${off}"
      else
        touch "$flag"; state="${on}"
      fi
      notify-send -c osd -h string:x-canonical-private-synchronous:osd \
        -t 1500 "${title}" "$state"
    '';
  };
  # the file pauses grouping, so creating it turns grouping off
  groupToggle = toggle "app-group-toggle" "app-group/off" "off" "on" "app grouping";
  urgentToggle = toggle "attention-urgent-toggle" "xwayland-attention/urgent" "on" "off" "xwayland urgent";

  args = lib.concatMap
    (name: [ "--group" "${name}=${lib.concatStringsSep "," groups.${name}}" ])
    (lib.attrNames groups)
    ++ [ "--title-suffix" titleSuffix ];
in
{
  systemd.user.services.sway-helpers = {
    Unit = {
      Description = "Tab group dragging, app grouping, ending groups of one and XWayland attention for sway";
      After = [ "sway-session.target" ];
      PartOf = [ "sway-session.target" ];
    };
    Service = {
      ExecStart = "${swayHelpers}/bin/sway-helpers ${lib.escapeShellArgs args}";
      Restart = "on-failure";
      RestartSec = 3;
    };
    Install.WantedBy = [ "sway-session.target" ];
  };

  wayland.windowManager.sway.config.keybindings = {
    "Mod4+Alt+g" = "exec ${lib.getExe groupToggle}";
    "Mod4+u" = "exec ${lib.getExe urgentToggle}";
  };

  # group-drag: Super then Shift, or Shift then Super (either side)
  wayland.windowManager.sway.extraConfig = ''
    bindsym --no-repeat Mod4+Shift_L nop group-drag
    bindsym --no-repeat Mod4+Shift_R nop group-drag
    bindsym --no-repeat Shift+Super_L nop group-drag
    bindsym --no-repeat Shift+Super_R nop group-drag
    bindsym --release Mod4+Shift_L nop group-drag-off
    bindsym --release Mod4+Shift_R nop group-drag-off
    bindsym --release Shift+Super_L nop group-drag-off
    bindsym --release Shift+Super_R nop group-drag-off
  '';
}
