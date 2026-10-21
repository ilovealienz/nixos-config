# The desktop's colours, variant, icons and wallpaper: config.theme.* in
# system modules, osConfig.theme.* in home-manager.
#
# The palettes below are the defaults (the Copper preset). Your own choices
# are in ~/.config/theme/colors.json (theme-edit); nxrebuild copies it here
# as local-theme.json, staged but never committed.
{ lib, config, ... }:
let
  palettes = {
    dark = {
      # backgrounds: change these four together to shift the tint
      bgDark  = "1d1916";  # kitty inactive tabs
      bg      = "2b201a";  # main background everywhere
      bgAlt   = "35241c";  # raised: gtk views/cards/popovers
      surface = "473c31";  # borders, inactive windows, launcher selection

      # text
      fg       = "d4a97b";  # main text
      fgBright = "ede0c8";  # bright text
      muted    = "8d744a";  # dim text, labels

      # accent: change this one line to recolour everything
      # Desert Night's amber (a preset): e5a440
      accent = "c86f42";

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

  # The defaults above, as theme-edit's presets show them
  defaultPreset = "Copper";

  # Colour presets in theme-edit, dark and light. The name after # is shown.
  presets = {
    desert-night = {  # Desert Night
      dark = { bgDark = "1d1b16"; bg = "24221c"; bgAlt = "2f2c24"; surface = "473f31"; fg = "d4b07b"; fgBright = "ede0c8"; muted = "87765d"; accent = "e5a440"; onAccent = "24221c"; selectionFg = "ede0c8"; red = "e56b55"; orange = "e18245"; yellow = "bfab36"; green = "99b05f"; blue = "949fb4"; magenta = "d261a5"; };
      light = { bgDark = "e8dfcb"; bg = "f4eee0"; bgAlt = "fbf8f0"; surface = "ddd0b5"; fg = "4a3b28"; fgBright = "2a2017"; muted = "76634a"; accent = "9c5a08"; onAccent = "fbf8f0"; selectionFg = "2a2017"; red = "ac3a26"; orange = "a34f14"; yellow = "7a6c0c"; green = "4f6e1c"; blue = "455a80"; magenta = "983874"; };
    };
  };

  # Worked out from the others. To override, set one anywhere:
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

  # Icon sets in theme-edit: package, borrowed packages (also), theme names
  # for dark and light, and web page. The name after # is shown. Only the
  # set in use is installed.
  iconSets = {
    gruvbox-plus = { package = "gruvbox-plus-icons"; dark = "Gruvbox-Plus-Dark"; light = "Gruvbox-Plus-Light"; page = "https://github.com/SylEleuth/gruvbox-plus-icon-pack"; };  # Gruvbox Plus
    gruvbox-dark = { package = "gruvbox-dark-icons-gtk"; dark = "oomox-gruvbox-dark"; light = "oomox-gruvbox-dark"; page = "https://github.com/jmattheis/gruvbox-dark-icons-gtk"; };  # Gruvbox Dark
    everforest = { package = "everforest-gtk-theme"; dark = "Everforest-Dark"; light = "Everforest-Light"; page = "https://github.com/Fausto-Korpsvart/Everforest-GTK-Theme"; };  # Everforest
    kanagawa = { package = "kanagawa-icon-theme"; dark = "Kanagawa"; light = "Kanagawa"; page = "https://github.com/Fausto-Korpsvart/Kanagawa-GKT-Theme"; };  # Kanagawa
    rose-pine = { package = "rose-pine-icon-theme"; dark = "rose-pine"; light = "rose-pine-dawn"; page = "https://github.com/rose-pine/gtk"; };  # Rosé Pine
    chicago95 = { package = "chicago95"; dark = "Chicago95"; light = "Chicago95"; page = "https://github.com/grassmunk/Chicago95"; };  # Chicago95
    candy = { package = "candy-icons"; dark = "candy-icons"; light = "candy-icons"; page = "https://github.com/EliverLara/candy-icons"; };  # Candy
    sweet-rainbow = { package = "sweet-folders"; also = "candy-icons"; dark = "Sweet-Rainbow"; light = "Sweet-Rainbow"; page = "https://github.com/EliverLara/Sweet-folders"; };  # Sweet Rainbow
    numix-circle = { package = "numix-icon-theme-circle"; also = "numix-icon-theme"; dark = "Numix-Circle"; light = "Numix-Circle-Light"; page = "https://github.com/numixproject/numix-icon-theme-circle"; };  # Numix Circle
    kora = { package = "kora-icon-theme"; dark = "kora"; light = "kora-light"; page = "https://github.com/bikass/kora"; };  # Kora
    zafiro = { package = "zafiro-icons"; dark = "Zafiro-icons-Dark"; light = "Zafiro-icons-Light"; page = "https://github.com/zayronxio/Zafiro-icons"; };  # Zafiro
    paper = { package = "paper-icon-theme"; dark = "Paper"; light = "Paper"; page = "https://github.com/snwh/paper-icon-theme"; };  # Paper
    la-capitaine = { package = "la-capitaine-icon-theme"; dark = "la-capitaine-icon-theme"; light = "la-capitaine-icon-theme"; page = "https://github.com/keeferrourke/la-capitaine-icon-theme"; };  # La Capitaine
    oranchelo = { package = "oranchelo-icon-theme"; dark = "Oranchelo"; light = "oranchelo-light"; page = "https://github.com/OrancheloTeam/oranchelo-icon-theme"; };  # Oranchelo
    papirus = { package = "papirus-icon-theme"; dark = "Papirus-Dark"; light = "Papirus-Light"; page = "https://github.com/PapirusDevelopmentTeam/papirus-icon-theme"; };  # Papirus
    adwaita = { package = "adwaita-icon-theme"; dark = "Adwaita"; light = "Adwaita"; page = "https://gitlab.gnome.org/GNOME/adwaita-icon-theme"; };  # Adwaita
  };
  iconDefault = "gruvbox-plus";

  # local-theme.json (colors.json), e.g.:
  #   { "variant": "light", "dark": { "accent": "4fa3e0" }, "light": { },
  #     "icons": "papirus", "wallpaper": { "from": "/path/x.jpg", "mode": "fill" } }
  # Invalid values fall back with a warning; unknown fields are ignored.
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

  mySet = local.icons or iconDefault;
  iconSet =
    if builtins.isString mySet && iconSets ? ${mySet} then mySet
    else lib.warn "local-theme.json: unknown icon set ${builtins.toJSON mySet}, using ${iconDefault}" iconDefault;

  # sway's wallpaper modes (sway-output(5))
  wallpaperModes = [ "fill" "fit" "center" "tile" "stretch" ];
  myWall = local.wallpaper or null;
  wallMode =
    if !builtins.isAttrs myWall then null
    else if builtins.elem (myWall.mode or "fill") wallpaperModes then myWall.mode or "fill"
    else lib.warn "local-theme.json: unknown wallpaper mode ${builtins.toJSON myWall.mode}, using fill" "fill";

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
    wallpaperMode = lib.mkOption {
      type = lib.types.nullOr (lib.types.enum wallpaperModes);
      default = wallMode;
      description = "How sway fits the wallpaper picked in theme-edit (~/.config/theme/wallpaper); null: none picked, the config's walls/1.png.";
    };
    icons = lib.mkOption {
      type = lib.types.enum (lib.attrNames iconSets);
      default = iconSet;
      description = "The icon set, one of iconSets in theme.nix.";
    };
    icon = lib.mkOption {
      type = lib.types.attrs;
      readOnly = true;
      default = let s = iconSets.${config.theme.icons}; in {
        inherit (s) package;
        also = lib.filter (p: p != "") (lib.splitString " " (s.also or ""));
        name = s.${config.theme.variant};
      };
      description = "The icon theme in use: its package's name in pkgs, the packages it borrows from, and the theme's name.";
    };
    colors =
      lib.mapAttrs (name: _: opt name palette.${name}) palettes.dark
      // lib.mapAttrs opt (derived config.theme.colors);
  };

  config.assertions = [{
    assertion = lib.attrNames palettes.dark == lib.attrNames palettes.light;
    message = "theme.nix: the dark and light palettes must have the same colours";
  }] ++ lib.concatLists (lib.mapAttrsToList (key: p: map (v: {
    assertion = p ? ${v} && lib.attrNames p.${v} == lib.attrNames palettes.dark
      && lib.all (c: builtins.match "[0-9a-fA-F]{6}" c != null) (lib.attrValues p.${v});
    message = "theme.nix: preset ${key}: ${v} must have every colour of the palettes, as 6-digit hex";
  }) [ "dark" "light" ]) presets);
}
