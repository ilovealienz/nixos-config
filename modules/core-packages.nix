{ pkgs, ... }:

{
  environment.systemPackages = with pkgs; [
    git
    curl
    wget
    htop
    unzip
    fastfetch
    brave
    vscodium
    vim
    mission-center
    gnome-disk-utility
    wl-clipboard
    libnotify
    proton-vpn
    xdg-utils
    fzf
    nix-search-tv
    gparted
    wireguard-tools
    (writeShellScriptBin "chromium-browser" ''
      if pgrep librewolf > /dev/null; then
        librewolf "$@" &
        exit 0
      fi
      exec librewolf "$@"
    '')
    (writeShellScriptBin "x-www-browser" ''
      if pgrep librewolf > /dev/null; then
        librewolf "$@" &
        exit 0
      fi
      exec librewolf "$@"
    '')
  ];
}
