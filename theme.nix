# System modules read config.theme.colors.<name>;
# home-manager modules read osConfig.theme.colors.<name>.
{ lib, config, ... }:
let
  palette = {
    # dark tones: change these four together to shift grey <-> brown
    bgDark  = "1d1b16";  # darkest: kitty inactive tabs
    bg      = "24221c";  # main background everywhere
    bgAlt   = "2f2c24";  # raised: gtk views/cards/popovers
    surface = "473f31";  # borders, inactive windows, selections

    # text
    fg       = "d4b07b";  # main text
    fgBright = "ede0c8";  # bright text
    muted    = "87765d";  # dim text, labels

    # accent: change this one line to recolour everything
    # original amber: e5a440
    accent = "e5a440";

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

  # Worked out from the accent, so they follow it automatically.
  # To hand-pick one instead, set it anywhere (e.g. a host file):
  #   theme.colors.selectionBg = "a17938";
  derived = c: {
    selectionBg = mix c.accent c.bg 0.67;  # selected files/rows in GTK apps
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

  hex = lib.types.strMatching "[0-9a-fA-F]{6}";
  opt = name: value: lib.mkOption {
    type = hex;
    default = value;
    description = "Theme colour ${name}: 6-digit hex, no leading #.";
  };
in
{
  options.theme.colors =
    lib.mapAttrs opt palette
    // lib.mapAttrs opt (derived config.theme.colors);
}
