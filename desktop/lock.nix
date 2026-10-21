# Locking: Super+Shift+x, 10 minutes idle (screens off at 15), before sleep.
{ pkgs, osConfig, ... }:
let c = osConfig.theme.colors; in
{
  wayland.windowManager.sway.config.keybindings."Mod4+Shift+x" = "exec swaylock";

  wayland.windowManager.sway.extraConfig = ''
    # no idle lock while a window is fullscreen
    for_window [all] inhibit_idle fullscreen
  '';

  # ── idle: lock at 10min, screens off at 15 ──
  services.swayidle = {
    enable = true;
    timeouts = [
      { timeout = 600; command = "${pkgs.swaylock}/bin/swaylock -f"; }
      {
        timeout = 900;
        command = "${pkgs.sway}/bin/swaymsg 'output * power off'";
        resumeCommand = "${pkgs.sway}/bin/swaymsg 'output * power on'";
      }
    ];
    # lock before sleep and on loginctl lock-session
    events = {
      before-sleep = "${pkgs.swaylock}/bin/swaylock -f";
      lock = "${pkgs.swaylock}/bin/swaylock -f";
    };
  };

  programs.swaylock = {
    enable = true;
    settings = {
      color = "${c.bg}";
      indicator-radius = 100;
      indicator-thickness = 10;
      ring-color = "${c.accent}";
      inside-color = "${c.surface}";
      text-color = "${c.fg}";
      key-hl-color = "${c.green}";
      line-color = "${c.bg}";
      separator-color = "${c.bg}";
      show-failed-attempts = true;
    };
  };
}
