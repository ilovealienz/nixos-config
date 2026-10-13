# The desktop's colours. System modules read config.theme.colors.<name>;
# home-manager modules read osConfig.theme.colors.<name>, and
# osConfig.theme.variant ("dark" or "light") for the GTK, icon and Qt
# themes that have to match.
#
# The two palettes below are the defaults. Your own colours, and which
# variant you use, live in ~/.config/theme/colors.json (edit them with
# theme-edit). nxrebuild copies that file here as local-theme.json, staged
# but never committed (like hardware-configuration.nix), so each machine
# keeps its own colours and they stay out of nxpush. Without it, the dark
# defaults are used.
{ lib, config, ... }:
let
  palettes = {
    dark = {
      # backgrounds: change these four together to shift the tint
      bgDark  = "1d1b16";  # kitty inactive tabs
      bg      = "24221c";  # main background everywhere
      bgAlt   = "2f2c24";  # raised: gtk views/cards/popovers
      surface = "473f31";  # borders, inactive windows, launcher selection

      # text
      fg       = "d4b07b";  # main text
      fgBright = "ede0c8";  # bright text
      muted    = "87765d";  # dim text, labels

      # accent: change this one line to recolour everything
      # original amber: e5a440
      accent = "e5a440";

      # text on the accent and on red: tab titles, urgent workspace, buttons
      onAccent = "24221c";

      # text on the selection highlight
      selectionFg = "ede0c8";

      # semantic + terminal colours
      red     = "e56b55";  # urgent, errors
      orange  = "e18245";  # warnings
      yellow  = "bfab36";
      green   = "99b05f";
      blue    = "949fb4";
      magenta = "d261a5";
    };

    light = {
      bgDark  = "e8dfcb";
      bg      = "f4eee0";
      bgAlt   = "fbf8f0";
      surface = "ddd0b5";

      fg       = "4a3b28";
      fgBright = "2a2017";
      muted    = "76634a";

      accent = "9c5a08";

      onAccent = "fbf8f0";

      selectionFg = "2a2017";

      red     = "ac3a26";
      orange  = "a34f14";
      yellow  = "7a6c0c";
      green   = "4f6e1c";
      blue    = "455a80";
      magenta = "983874";
    };
  };

  # Worked out from the others, so they follow them automatically.
  # To hand-pick one instead, set it anywhere (e.g. a host file):
  #   theme.colors.selectionBg = "a17938";
  derived = c: {
    selectionBg = mix c.accent c.bg 0.67;  # selected files/rows, terminal selection
    accentDim   = mix c.accent c.bg 0.22;  # mako progress bar
  };

  # blend hex colour a towards b; t = how much of a (0.0 - 1.0)
  mix = a: b: t:
    let
      ch = h: i: lib.fromHexString (builtins.substring i 2 h);
      pad = s: if builtins.stringLength s == 1 then "0${s}" else s;
      one = i: pad (lib.toLower (lib.toHexString
        (builtins.floor (ch a i * t + ch b i * (1.0 - t) + 0.5))));
    in one 0 + one 2 + one 4;

  # ~/.config/theme/colors.json, copied here by nxrebuild:
  #   { "variant": "light", "dark": { "accent": "4fa3e0" }, "light": { } }
  # Only colours are read from it, and each is checked to be 6-digit hex.
  local =
    if builtins.pathExists ./local-theme.json
    then builtins.fromJSON (builtins.readFile ./local-theme.json)
    else { };
  variant = config.theme.variant;
  mine = local.${variant} or { };
  unknown = lib.attrNames (removeAttrs mine (lib.attrNames palettes.${variant}));
  palette = lib.warnIf (unknown != [ ])
    "local-theme.json: ignoring unknown colours ${lib.concatStringsSep ", " unknown}"
    (palettes.${variant} // builtins.intersectAttrs palettes.${variant} mine);

  hex = lib.types.strMatching "[0-9a-fA-F]{6}";
  opt = name: value: lib.mkOption {
    type = hex;
    default = value;
    description = "Theme colour ${name}: 6-digit hex, no leading #.";
  };
in
{
  options.theme = {
    variant = lib.mkOption {
      type = lib.types.enum [ "dark" "light" ];
      default = local.variant or "dark";
      description = "Dark or light: picks the palette, and the GTK, icon and Qt themes.";
    };
    colors =
      lib.mapAttrs (name: _: opt name palette.${name}) palettes.dark
      // lib.mapAttrs opt (derived config.theme.colors);
  };

  config.assertions = [{
    assertion = lib.attrNames palettes.dark == lib.attrNames palettes.light;
    message = "theme.nix: the dark and light palettes must have the same colours";
  }];
}
