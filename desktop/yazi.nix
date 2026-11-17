# Yazi: terminal file manager on Super+Alt+E, in kitty for image previews
# (Thunar stays on Super+E). Arrow keys work like hjkl; F1 lists every key.
{ pkgs, osConfig, ... }:
let
  # Super+Alt+E: starts where the user folders are (/mnt/Shared/User on pc, ~ elsewhere)
  yaziFiles = pkgs.writeShellApplication {
    name = "yazi-files";
    runtimeInputs = [ pkgs.xdg-user-dirs pkgs.coreutils ];
    text = ''
      dl=$(xdg-user-dir DOWNLOAD 2>/dev/null || true)
      dir=$HOME
      if [ -n "$dl" ] && [ "$dl" != "$HOME" ] && [ -d "$(dirname "$dl")" ]; then
        dir=$(dirname "$dl")
      fi
      exec yazi "$dir"
    '';
  };

  # yazi's own theme in theme.colors; icons get the palette colour nearest theirs
  theme = pkgs.runCommandLocal "yazi-theme.toml" { nativeBuildInputs = [ pkgs.python3 ]; } ''
    python3 ${./yazi-theme.py} \
      ${pkgs.yazi-unwrapped.passthru.srcs.code_src}/yazi-config/preset/theme-${osConfig.theme.variant}.toml \
      ${pkgs.writeText "palette.json" (builtins.toJSON osConfig.theme.colors)} $out
  '';
in
{
  home.packages = [ yaziFiles ];
  xdg.configFile."yazi/theme.toml".source = theme;

  programs.yazi = {
    enable = true;
    # y: opens yazi, and the shell goes where you left it
    enableZshIntegration = true;
    plugins.smart-enter = pkgs.yaziPlugins.smart-enter;
    # copy or cut in one yazi window, paste in another
    initLua = ''
      require("session"):setup { sync_yanked = true }
    '';
    # Enter goes into folders too (by default it opens them in Thunar)
    keymap.mgr.prepend_keymap = [
      { on = "<Enter>"; run = "plugin smart-enter"; desc = "Enter the folder, or open the file"; }
    ];
  };
}
