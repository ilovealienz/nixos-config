{ pkgs, ... }:

 let
  # swayosd on the focused monitor only (default shows on all three)
  osd = pkgs.writeShellScript "osd-focused" ''
    exec ${pkgs.swayosd}/bin/swayosd-client \
      --monitor "$(${pkgs.hyprland}/bin/hyprctl monitors -j | ${pkgs.jq}/bin/jq -r '.[] | select(.focused == true).name')" \
      "$@"
  '';
 in
 {

  home.packages = [
    (pkgs.writeShellScriptBin "screenshot-menu" ''
      pics="$(${pkgs.xdg-user-dirs}/bin/xdg-user-dir PICTURES)"
      dir="$pics/Screenshots/$(date +%Y-%m)"
      mkdir -p "$dir"
      file="$dir/$(date +%Y-%m-%d_%H-%M-%S).png"

      # capture region; abort if slurp cancelled (Esc)
      ${pkgs.grim}/bin/grim -g "$(${pkgs.slurp}/bin/slurp)" "$file" || exit 0

      choice=$(printf "open\ncopy\nupload (zipline)\nupload advanced\ndelete" \
        | wmenu -f 'Inter 13' -N 24221c -n d4b07b -S e5a440 -s 24221c -p "shot:")

      case "$choice" in
        open)               xdg-open "$file" ;;
        copy)               ${pkgs.wl-clipboard}/bin/wl-copy < "$file" ;;
        "upload (zipline)") $HOME/.bin/zipline-upload "$file" ;;
        "upload advanced")  $HOME/.bin/zipline-upload --advanced "$file" ;;
        delete)             rm "$file" ;;
      esac

    '')
  ];
	
  wayland.windowManager.hyprland = {
    enable = true;
    configType = "hyprlang";
    settings = {
      "$mod" = "SUPER";

      exec-once = [
        "waybar"
        "${pkgs.polkit_gnome}/libexec/polkit-gnome-authentication-agent-1"
	"${pkgs.swaybg}/bin/swaybg -m fill -i ${../walls/1.png}"
      ];

      env = [
        "XCURSOR_THEME,Bibata-Modern-Classic"
        "XCURSOR_SIZE,24"
      ];

      animations.enabled = false;

      general = {
        gaps_in = 2;
        gaps_out = 2;
        border_size = 2;
        "col.active_border" = "rgb(e5a440)";
        "col.inactive_border" = "rgb(473f31)";
        layout = "dwindle";
      };

      decoration = {
        rounding = 0;
        blur.enabled = false;
        shadow.enabled = false;
      };

      input = {
        kb_layout = "gb";
        follow_mouse = 1;
	accel_profile = "flat";
	force_no_accel = true;
      };

      dwindle = {
        preserve_split = true;
      };

      master = {
        mfact = 0.5;                 # master (Spotify) takes left half of WS2
      };

      group = {
        "col.border_active" = "rgb(e5a440)";
        "col.border_inactive" = "rgb(473f31)";
        "groupbar:font_family" = "Inter";
        "groupbar:font_size" = 11;
        "groupbar:height" = 16;
        "groupbar:gradients" = true;
        "groupbar:gaps_in" = 0;
        "groupbar:gaps_out" = 0;
        "groupbar:col.active" = "rgb(e5a440)";
        "groupbar:col.inactive" = "rgb(473f31)";
        "groupbar:text_color" = "rgb(24221c)";
      };

      # WS2 uses master layout so left/right is deterministic
      workspace = [
        "2, layoutname:master"
      ];

      bind = [
        "$mod, Return, exec, kitty"
        "$mod SHIFT, R, exec, fuzzel"
        "$mod, R, exec, wmenu-run -f 'Inter 13' -N 24221c -n d4b07b -S e5a440 -s 24221c"
        "$mod, E, exec, thunar"
        "$mod, Q, killactive"
        "$mod SHIFT, E, exit"
        "$mod ALT, L, exec, hyprlock"
        "$mod, F, fullscreen, 0"
        "$mod, V, togglefloating"
        "$mod, P, pseudo"
        "$mod, S, layoutmsg, togglesplit"

        "$mod, h, movefocus, l"
        "$mod, l, movefocus, r"
        "$mod, k, movefocus, u"
        "$mod, j, movefocus, d"

	"$mod, G, togglegroup"
        "$mod SHIFT, G, moveintoorcreategroup, r"
        "$mod, Tab, changegroupactive, f"

        "$mod SHIFT, h, movewindow, l"
        "$mod SHIFT, l, movewindow, r"
        "$mod SHIFT, k, movewindow, u"
        "$mod SHIFT, j, movewindow, d"

        "$mod, 1, workspace, 1"
        "$mod, 2, workspace, 2"
        "$mod, 3, workspace, 3"
        "$mod, 4, workspace, 4"
        "$mod, 5, workspace, 5"
        "$mod, 6, workspace, 6"
        "$mod, 7, workspace, 7"
        "$mod, 8, workspace, 8"
        "$mod, 9, workspace, 9"

        "$mod SHIFT, 1, movetoworkspace, 1"
        "$mod SHIFT, 2, movetoworkspace, 2"
        "$mod SHIFT, 3, movetoworkspace, 3"
        "$mod SHIFT, 4, movetoworkspace, 4"
        "$mod SHIFT, 5, movetoworkspace, 5"
        "$mod SHIFT, 6, movetoworkspace, 6"
        "$mod SHIFT, 7, movetoworkspace, 7"
        "$mod SHIFT, 8, movetoworkspace, 8"
        "$mod SHIFT, 9, movetoworkspace, 9"

        ''$mod SHIFT, S, exec, grim -g "$(slurp)" - | wl-copy''
	'', Print, exec, screenshot-menu''
      ];

      bindel = [
        ", XF86AudioRaiseVolume, exec, ${osd} --output-volume raise --max-volume 150"
        ", XF86AudioLowerVolume, exec, ${osd} --output-volume lower --max-volume 150"
        ", XF86MonBrightnessUp, exec, ${osd} --brightness raise"
        ", XF86MonBrightnessDown, exec, ${osd} --brightness lower"
      ];

      bindl = [
        ", XF86AudioMute, exec, ${osd} --output-volume mute-toggle"
        ", XF86AudioPlay, exec, playerctl play-pause"
        ", XF86AudioNext, exec, playerctl next"
        ", XF86AudioPrev, exec, playerctl previous"
      ];

      bindm = [
        "$mod, mouse:272, movewindow"
        "$mod, mouse:273, resizewindow"
      ];
    };

    extraConfig = ''

      layerrule = ignore_alpha 0, match:namespace waybar

      # floating popups
      windowrule = float on, size 900 600, center on, match:class kitty-float
      windowrule = float on, match:class pavucontrol
      windowrule = float on, match:class blueman-manager

      # workspace 1 — browsers
      windowrule = workspace 1, match:class firefox
      windowrule = workspace 1, match:class brave-browser
      windowrule = workspace 1, match:class floorp

      # workspace 2 — music + chat (master layout: first-opened = left)
      windowrule = workspace 2, match:class spotify
      windowrule = workspace 2, match:class signal
      windowrule = workspace 2, match:class vesktop

      # workspace 3 — mpv
      windowrule = workspace 3, match:class mpv

      # workspace 4 — terminal
      windowrule = workspace 4, match:class kitty

      # workspace 5 — files
      windowrule = workspace 5, match:class thunar
    '';
  };

  services.hypridle = {
    enable = true;
    settings = {
      general = {
        lock_cmd = "pidof hyprlock || hyprlock";
        before_sleep_cmd = "loginctl lock-session";
      };
   listener = [
        { timeout = 600; on-timeout = "loginctl lock-session"; }
        { timeout = 900; on-timeout = "hyprctl dispatch dpms off"; on-resume = "hyprctl dispatch dpms on"; }
      ];
    };
  };

  services.swayosd = {
    enable = true;
    topMargin = 0.85;
  };

  programs.hyprlock = {
    enable = true;
    settings = {
      background = [{ color = "rgb(36, 34, 28)"; }];
      input-field = [{
        size = "300, 50";
        outline_thickness = 2;
        outer_color = "rgb(229, 164, 64)";
        inner_color = "rgb(71, 63, 49)";
        font_color = "rgb(212, 176, 123)";
        placeholder_text = "password";
      }];
    };
  };
}
