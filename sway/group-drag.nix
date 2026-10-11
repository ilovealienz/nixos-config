# Super+Shift+drag moves a whole tab group: drag any window of the group
# the normal way and the rest of the group follows it once it's dropped,
# in the same order. Dropped on the middle of another window, the two swap
# places as usual and that window is untabbed. Super+drag still moves a
# single window out of its group.
#
# Pressing Super and Shift together runs "nop group-drag" (releasing them
# "nop group-drag-off"); the watcher only looks for drags in between.
# Written in Rust with only the standard library (~2MB of RAM).
# Source: ../pkgs/group-drag/src/main.rs
{ pkgs, ... }:
let
  groupDrag = pkgs.rustPlatform.buildRustPackage {
    pname = "group-drag";
    version = "2.0.0";
    src = ../pkgs/group-drag;
    cargoLock.lockFile = ../pkgs/group-drag/Cargo.lock;   # no dependencies
  };
in
{
  systemd.user.services.group-drag = {
    Unit = {
      Description = "Make tab groups follow a window dragged with Super+Shift";
      After = [ "sway-session.target" ];
      PartOf = [ "sway-session.target" ];
    };
    Service = {
      ExecStart = "${groupDrag}/bin/group-drag";
      Restart = "on-failure";
      RestartSec = 3;
    };
    Install.WantedBy = [ "sway-session.target" ];
  };

  # Super then Shift, or Shift then Super (either side)
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
