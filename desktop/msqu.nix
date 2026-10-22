# msqu (my sway quick utils), Super+Shift+A: timer, stopwatch, alarm,
# repeat, drink reminder, countdown to a date, world clock and notes, with
# bar blocks. Source: ../pkgs/msqu/
#
# Adding a tool: menu/src/tool/NAME.rs, bin/msqu-tool.rs and `tools` below,
# plus model.rs (state), main.rs (menu) and bin/msqu-bar.rs (block).
# bar = true gives it a block; slots = N runs N at once.
{ pkgs, lib, osConfig, ... }:
let
  c = osConfig.theme.colors;

  # repeat presets; focus: app_id=ID, class=CLASS or - for none
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
      deps = with pkgs; [ systemd libnotify pipewire mako ];
      bar = true;
    };
    drink = {
      deps = with pkgs; [ systemd libnotify pipewire mako ];
      bar = true;
    };
    # kept across reboots, like clock and notes (~/.local/state/msqu)
    countdown = {
      deps = with pkgs; [ systemd libnotify pipewire mako ];
      bar = true;
      slots = 3;
    };
    clock = {
      deps = [ ];
      bar = true;
      slots = 3;
    };
    # a block only while a reminder is set
    notes = {
      deps = with pkgs; [ systemd libnotify pipewire mako ];
      bar = true;
    };
  };

  # block order on the bar
  order = [ "timer" "stopwatch" "alarm" "repeat" "drink" "notes" "countdown" "clock" ];

  # repeat and drink bell, as FLAC for pw-play
  bellSound = pkgs.runCommand "msqu-bell.flac" { nativeBuildInputs = [ pkgs.ffmpeg-headless ]; } ''
    ffmpeg -loglevel error -i ${../pkgs/msqu/bell.mp3} -f flac $out
  '';

  # timer and alarm sound: normalised, boosted by alarmBoost dB, limited
  alarmBoost = 6;
  alarmSound = pkgs.runCommand "msqu-alarm.flac" { nativeBuildInputs = [ pkgs.ffmpeg-headless pkgs.gawk ]; } ''
    src=${../pkgs/msqu/alarm.mp3}
    # audio only (skip cover art)
    peak=$(ffmpeg -hide_banner -nostats -i "$src" -map 0:a:0 -af volumedetect -f null - 2>&1 \
      | sed -n 's/.*max_volume: \(-\{0,1\}[0-9.]*\) dB.*/\1/p')
    gain=$(awk -v p="''${peak:-0}" -v b=${toString alarmBoost} 'BEGIN { print -p + b }')
    ffmpeg -loglevel error -i "$src" -map 0:a:0 -af "volume=''${gain}dB,alimiter=limit=0.89:level=0" \
      -ar 48000 -f flac $out
  '';

  # msqu-NAME: a compiled wrapper for msqu-tool NAME with its PATH and settings
  toolEnv = name: t: { MSQU_TOOL = name; MSQU_BELL = "${bellSound}"; MSQU_SELF = "$out/bin/msqu-${name}"; }
    // lib.optionalAttrs (t ? slots) { MSQU_SLOTS = toString t.slots; }
    // (t.env or { });
  tool = name: t: pkgs.runCommand "msqu-${name}" {
    nativeBuildInputs = [ pkgs.makeBinaryWrapper ];
    meta.mainProgram = "msqu-${name}";
  } ''
    makeWrapper ${msquUi}/bin/msqu-tool $out/bin/msqu-${name} \
      --add-flags ${name} \
      ${lib.optionalString (t.deps != [ ]) "--prefix PATH : ${lib.makeBinPath t.deps}"} \
      ${lib.concatStringsSep " " (lib.mapAttrsToList (k: v: "--set ${k} \"${v}\"") (toolEnv name t))}
  '';
  toolPkgs = lib.mapAttrs tool tools;

  # menu config: theme colours, presets, slots
  menuConfig = pkgs.writeText "msqu-menu.conf" (
    lib.concatStrings (lib.mapAttrsToList (n: v: "color ${n} ${v}\n") c)
    + lib.concatMapStrings (p: "preset ${p.label}|${p.every}|${p.focus}\n") repeatPresets
    + lib.concatStrings (lib.mapAttrsToList (n: t: lib.optionalString (t ? slots) "slots ${n} ${toString t.slots}\n") tools)
  );
  # msqu-ui, msqu-bar and msqu-tool
  msquUi = pkgs.rustPlatform.buildRustPackage {
    pname = "msqu-ui";
    version = "1.2.0";
    src = ../pkgs/msqu/menu;
    cargoLock.lockFile = ../pkgs/msqu/menu/Cargo.lock;
    meta.mainProgram = "msqu-ui";
  };
  menu = pkgs.writeShellApplication {
    name = "msqu-menu";
    # calculator: copy the answer
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
  # one block per tool, or per slot
  blocks = lib.concatMap (n:
    if tools.${n} ? slots
    then map (i: { name = "${n}-${toString i}"; tool = n; arg = " ${toString i}"; }) (lib.range 1 tools.${n}.slots)
    else [{ name = n; tool = n; arg = ""; }]) barTools;

  colours = [ "accent" "fg" "red" "orange" "yellow" "green" "blue" "magenta" ];

  # right-click on any block: its options in fuzzel (the one block: what's
  # open, each with its options)
  pick = pkgs.runCommand "msqu-pick" { nativeBuildInputs = [ pkgs.makeBinaryWrapper ]; } ''
    makeWrapper ${msquUi}/bin/msqu-tool $out/bin/msqu-pick \
      --add-flags pick \
      --prefix PATH : ${lib.makeBinPath ([ pkgs.fuzzel pkgs.libnotify ] ++ lib.attrValues toolPkgs)}
  '';
  # middle-click: cancel, stop or remove it (notes: the next reminder)
  middle = tool: if tool == "notes" then "right-click" else "cancel";
in
{
  home.packages = [ msqu ];

  # keep repeat and drink out of the history
  services.mako.settings."category=msqu-repeat".history = false;
  services.mako.settings."category=msqu-drink".history = false;
  # ringing shows even in do-not-disturb (closing it stops the sound)
  services.mako.settings."mode=dnd category=msqu-ring".invisible = false;
  # at login: countdown and note reminders armed again (their timers don't
  # outlive a reboot), and any missed meanwhile sent
  systemd.user.services.msqu-rearm = {
    Unit = {
      Description = "Arm msqu's countdown and note reminders again";
      After = [ "sway-session.target" ];
    };
    Service = {
      Type = "oneshot";
      # mako up first, for the missed ones
      ExecStartPre = "${pkgs.coreutils}/bin/sleep 5";
      ExecStart = [ "${run "countdown"} rearm" "${run "notes"} rearm" ];
    };
    Install.WantedBy = [ "sway-session.target" ];
  };

  wayland.windowManager.sway.config = {
    keybindings."Mod4+Shift+a" = "exec ${lib.getExe msqu}";
    window.commands = [{
      criteria.app_id = "^msqu$";
      command = "floating enable, resize set 660 460, move position center";
    }];
  };

  # custom/msqu-all: the one block, when the menu has the bar set to that
  # (the others then show only if pinned). Every block: click opens msqu,
  # right-click its options, middle-click cancels it
  programs.waybar.settings.mainBar = {
    modules-right = lib.mkBefore ([ "custom/msqu-all" ] ++ map (b: "custom/msqu-${b.name}") blocks);
    "custom/msqu-all" = {
      exec = "${bar} all";
      exec-on-event = false;
      restart-interval = 5;
      return-type = "json";
      on-click = lib.getExe msqu;
      on-click-right = "${pick}/bin/msqu-pick all";
    };
  } // lib.listToAttrs (map (b: lib.nameValuePair "custom/msqu-${b.name}" {
    # long-running; prints only on change
    exec = "${bar} ${b.tool}${b.arg}";
    exec-on-event = false;
    restart-interval = 5;
    return-type = "json";
    on-click = lib.getExe msqu;
    on-click-right = "${pick}/bin/msqu-pick ${b.tool}${b.arg}";
    on-click-middle = "${run b.tool} ${middle b.tool}${b.arg}";
  }) blocks);

  # paused: dimmer with a red underline; tabular digits
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
