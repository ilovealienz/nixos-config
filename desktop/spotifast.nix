# Spotifast's "desktop" theme, from theme.colors. Spotifast skips symlinked
# themes, so it's copied in rather than linked.
{ pkgs, lib, osConfig, ... }:
let
  c = osConfig.theme.colors;
  hex = v: "#${v}";
  # bg scaled towards black, so it keeps the palette's tint (bgDark is greyer)
  darker = h: t:
    let
      ch = i: lib.fromHexString (builtins.substring i 2 h);
      pad = s: if builtins.stringLength s == 1 then "0${s}" else s;
      one = i: pad (lib.toLower (lib.toHexString (builtins.floor (ch i * t + 0.5))));
    in one 0 + one 2 + one 4;
  theme = pkgs.writeText "spotifast-desktop.json" (builtins.toJSON {
    base = osConfig.theme.variant;
    colors = lib.mapAttrs (_: hex) {
      window = c.bg;
      panel = darker c.bg (if osConfig.theme.variant == "dark" then 0.78 else 0.95);
      surface = c.bgAlt;
      surface_hover = c.surface;
      surface_active = c.selectionBg;
      outline = c.surface;
      text = c.fg;
      secondary = c.muted;
      dim = c.muted;
      accent = c.accent;
      accent_hover = c.selectionBg;
      on_accent = c.onAccent;
      danger = c.red;
      warning = c.orange;
    };
  });
in
{
  home.activation.spotifastTheme = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
    run install -Dm644 ${theme} "''${XDG_CONFIG_HOME:-$HOME/.config}/spotifast/themes/desktop.json"
  '';
}
