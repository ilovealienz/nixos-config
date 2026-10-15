# msqu (my sway quick utils): small tools with a menu on Super+Shift+A and,
# if they want one, a block on the bar. In ../pkgs/msqu/: menu/ (Rust: the
# menu, msqu-ui, in a floating terminal, and the bar blocks, msqu-bar),
# msqu.sh (the msqu command), lib.sh (shared by every tool) and one script
# per tool.
#
# The menu: what's running (enter pauses or resumes, ctrl+x stops),
# something new, presets and recent ones (ctrl+x removes one; "clear all"
# at the end empties the list). Typing a time starts it straight away:
# "25 tea" a timer, "7pm" (or "a 7:30 gym") an alarm, "d 45" the drink
# reminder; "r 15" goes on to pick a window for a repeat. A sum ("1024x8",
# "4+4", "(3+4)*2^3") shows its answer; enter copies it.
#
# timer: how long ("5" is minutes; "1:30", "90s", "1h 30m") and a label.
# Up to 3 at once, each its own block. Counts down on the bar in yellow:
# click to pause or resume, right-click to cancel, middle-click for more
# time (fuzzel: +5m, -1m, or type +3, 90s, 1:30). When it ends, a
# notification shows and the alarm sound (alarmSound below) plays until
# it's closed.
#
# stopwatch: counts up, in green; click to pause, right-click to stop
# (a notification shows the time).
#
# alarm: rings at a time of day ("7:30", "19:45", "7pm"; tomorrow if that's
# passed), like the timer: a notification, and the alarm sound until it's
# closed. Up to 3 at once, each its own block. Orange on the bar, showing
# when; right-click to cancel, or enter on it in the menu to change it.
#
# repeat: goes off every so often until stopped, and can bring a window to
# the front each time (the Roblox anti-AFK preset below). Blue on the bar:
# click to pause or resume, right-click to stop.
#
# drink: a reminder to drink water every so often (45 minutes to start
# with), with repeat's bell, until stopped; skipped while the screen is
# locked. Magenta on the bar, showing the minutes to the next one: click
# to pause or resume, right-click to stop.
#
# Adding a tool: write ../pkgs/msqu/NAME.sh (lib.sh says what it needs),
# add it to `tools` below, and teach menu/src its state file (model.rs),
# its menu entries (main.rs) and its block (bin/msqu-bar.rs). A tool with
# `bar = true` gets a waybar block (#custom-msqu-NAME) at the start of the
# right side; clicking it runs "NAME.sh click", right-click "right-click",
# middle-click "middle-click". With `slots = N` it can run N at once, and
# gets a block for each (#custom-msqu-NAME-1, ...; the clicks get the
# number too). The blocks' colours come from the theme (the style below).
{ pkgs, lib, osConfig, ... }:
let
  c = osConfig.theme.colors;

  # repeat presets, in the menu's PRESETS.
  # focus: the window to bring up (app_id=ID or class=CLASS; see
  # `swaymsg -t get_tree | grep -E '"(app_id|class)"'`), or - for none
  repeatPresets = [
    { label = "roblox anti-AFK"; every = "15m"; focus = "app_id=org.vinegarhq.Sober"; }
  ];

  tools = {
    timer = {
      deps = with pkgs; [ systemd libnotify pipewire fuzzel ];
      bar = true;
      slots = 3;
      env.MSQU_ALARM = "${alarmSound}";
    };
    stopwatch = {
      deps = with pkgs; [ libnotify ];
      bar = true;
    };
    alarm = {
      deps = with pkgs; [ systemd libnotify pipewire ];
      bar = true;
      slots = 3;
      env.MSQU_ALARM = "${alarmSound}";
    };
    repeat = {
      deps = with pkgs; [ systemd libnotify pipewire mako sway jq ];
      bar = true;
    };
    drink = {
      deps = with pkgs; [ systemd libnotify pipewire mako procps ];
      bar = true;
    };
  };

  # the order of the blocks on the bar (left to right)
  order = [ "timer" "stopwatch" "alarm" "repeat" "drink" ];

  # repeat's and drink's bell (quiet in do-not-disturb):
  # ../pkgs/msqu/bell.mp3, made FLAC so pw-play can play it whatever its
  # MP3 support
  bellSound = pkgs.runCommand "msqu-bell.flac" { nativeBuildInputs = [ pkgs.ffmpeg-headless ]; } ''
    ffmpeg -loglevel error -i ${../pkgs/msqu/bell.mp3} -f flac $out
  '';

  # timer's and alarm's sound, played until the notification is closed:
  # ../pkgs/msqu/alarm.mp3, brought up to just under full scale and then
  # alarmBoost dB louder still, with a limiter so it doesn't clip
  alarmBoost = 6;
  alarmSound = pkgs.runCommand "msqu-alarm.flac" { nativeBuildInputs = [ pkgs.ffmpeg-headless pkgs.gawk ]; } ''
    src=${../pkgs/msqu/alarm.mp3}
    # audio only: an MP3 can carry cover art too
    peak=$(ffmpeg -hide_banner -nostats -i "$src" -map 0:a:0 -af volumedetect -f null - 2>&1 \
      | sed -n 's/.*max_volume: \(-\{0,1\}[0-9.]*\) dB.*/\1/p')
    gain=$(awk -v p="''${peak:-0}" -v b=${toString alarmBoost} 'BEGIN { print -p + b }')
    ffmpeg -loglevel error -i "$src" -map 0:a:0 -af "volume=''${gain}dB,alimiter=limit=0.89:level=0" \
      -ar 48000 -f flac $out
  '';

  tool = name: t: pkgs.writeShellApplication {
    name = "msqu-${name}";
    runtimeInputs = with pkgs; [ coreutils ] ++ t.deps;
    runtimeEnv = { MSQU_TOOL = name; MSQU_BELL = "${bellSound}"; }
      // lib.optionalAttrs (t ? slots) { MSQU_SLOTS = toString t.slots; }
      // (t.env or { });
    text = builtins.readFile ../pkgs/msqu/lib.sh + "\n" + builtins.readFile (../pkgs/msqu + "/${name}.sh");
  };
  toolPkgs = lib.mapAttrs tool tools;

  # the menu: theme colours and presets, then the program
  menuConfig = pkgs.writeText "msqu-menu.conf" (
    lib.concatStrings (lib.mapAttrsToList (n: v: "color ${n} ${v}\n") c)
    + lib.concatMapStrings (p: "preset ${p.label}|${p.every}|${p.focus}\n") repeatPresets
    + lib.concatStrings (lib.mapAttrsToList (n: t: lib.optionalString (t ? slots) "slots ${n} ${toString t.slots}\n") tools)
  );
  # the menu and the bar blocks (msqu-ui, msqu-bar)
  msquUi = pkgs.rustPlatform.buildRustPackage {
    pname = "msqu-ui";
    version = "1.0.0";
    src = ../pkgs/msqu/menu;
    cargoLock.lockFile = ../pkgs/msqu/menu/Cargo.lock;
    meta.mainProgram = "msqu-ui";
  };
  menu = pkgs.writeShellApplication {
    name = "msqu-menu";
    # wl-copy and notify-send: the calculator's answer to the clipboard
    runtimeInputs = [ msquUi pkgs.wl-clipboard pkgs.libnotify ] ++ lib.attrValues toolPkgs;
    text = ''exec msqu-ui --config ${menuConfig} "$@"'';
  };

  msqu = pkgs.writeShellApplication {
    name = "msqu";
    runtimeInputs = lib.attrValues toolPkgs;
    runtimeEnv = {
      MSQU_TOOLS = lib.concatStringsSep " " (lib.attrNames tools);
      MSQU_MENU = lib.getExe menu;
    };
    text = builtins.readFile ../pkgs/msqu/msqu.sh;
  };

  barTools = lib.filter (n: tools.${n}.bar or false) order;
  run = name: "${lib.getExe toolPkgs.${name}}";
  bar = "${msquUi}/bin/msqu-bar";
  # the bar's blocks: one per tool, or one per slot ("timer" 1, 2, 3)
  blocks = lib.concatMap (n:
    if tools.${n} ? slots
    then map (i: { name = "${n}-${toString i}"; tool = n; arg = " ${toString i}"; }) (lib.range 1 tools.${n}.slots)
    else [{ name = n; tool = n; arg = ""; }]) barTools;

  colours = [ "accent" "red" "orange" "yellow" "green" "blue" "magenta" ];
in
{
  home.packages = [ msqu ];

  # repeat's and drink's notifications come every so often: keep them out
  # of the notification history (the others stay in)
  services.mako.settings."category=msqu-repeat".history = false;
  services.mako.settings."category=msqu-drink".history = false;
  # a timer or alarm going off shows even in do-not-disturb, so it can be
  # closed (which stops the sound)
  services.mako.settings."mode=dnd category=msqu-ring".invisible = false;
  wayland.windowManager.sway.config = {
    keybindings."Mod4+Shift+a" = "exec ${lib.getExe msqu}";
    # the menu's terminal: floating, in the middle
    window.commands = [{
      criteria.app_id = "^msqu$";
      command = "floating enable, resize set 660 460, move position center";
    }];
  };

  programs.waybar.settings.mainBar = {
    modules-right = lib.mkBefore (map (b: "custom/msqu-${b.name}") blocks);
  } // lib.listToAttrs (map (b: lib.nameValuePair "custom/msqu-${b.name}" {
    # runs for as long as the bar does, printing only when the block
    # changes (msqu-bar: small, and asleep unless something's counting)
    exec = "${bar} ${b.tool}${b.arg}";
    exec-on-event = false;
    # started again if it ever stops (waybar otherwise leaves it gone)
    restart-interval = 5;
    return-type = "json";
    on-click = "${run b.tool} click${b.arg}";
    on-click-right = "${run b.tool} right-click${b.arg}";
    on-click-middle = "${run b.tool} middle-click${b.arg}";
  }) blocks);

  # blocks: coloured text on a faint tint of the same colour; paused, both
  # dimmer, with a red line along the bottom. Same-width digits so the bar
  # doesn't shift as they count.
  programs.waybar.style = lib.mkAfter (''
    .msqu {
      padding: 0 8px;
      margin-right: 4px;
      font-feature-settings: "tnum";
    }
    .msqu.paused {
      box-shadow: inset 0 -2px #${c.red};
    }
  '' + lib.concatMapStrings (n: ''
    .msqu.${n} {
      color: #${c.${n}};
      background: alpha(#${c.${n}}, 0.15);
    }
    .msqu.${n}.paused {
      color: alpha(#${c.${n}}, 0.6);
      background: alpha(#${c.${n}}, 0.07);
    }
  '') colours);
}
