# foot: a light terminal (about 10-20 MB a window, against kitty's ~60)
# for the popups (the msqu menu, the shortcuts list, nmtui, theme-edit's
# rebuild) and on Super+Shift+Return. kitty stays the main terminal
# (Super+Return) for apps that show images (kitty's image protocol).
# Same font and colours as kitty (kitty.nix).
{ osConfig, ... }:
let
  c = osConfig.theme.colors;
  dark = osConfig.theme.variant == "dark";
  colors = {
    foreground = c.fg;
    background = c.bg;
    # black: on a light background it's the dark text
    regular0 = if dark then c.surface else c.fgBright;
    regular1 = c.red;
    regular2 = c.green;
    regular3 = c.orange;
    regular4 = c.blue;
    regular5 = c.magenta;
    regular6 = c.yellow;
    regular7 = c.muted;
    bright0 = if dark then c.surface else c.muted;
    bright1 = c.red;
    bright2 = c.green;
    bright3 = c.accent;
    bright4 = c.blue;
    bright5 = c.magenta;
    bright6 = c.yellow;
    bright7 = c.muted;
    # the same as selected rows in GTK apps
    selection-foreground = c.selectionFg;
    selection-background = c.selectionBg;
    # text under the cursor, then the cursor
    cursor = "${c.bg} ${c.fg}";
  };
in
{
  programs.foot = {
    enable = true;
    settings = {
      main.font = "MonaspiceAr Nerd Font:size=12";
      # the palette already follows theme.nix's dark or light: the same in
      # both, whichever foot picks
      colors-dark = colors;
      colors-light = colors;
    };
  };
}
