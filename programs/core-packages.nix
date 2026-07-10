{ pkgs, ... }:

{
  environment.systemPackages = with pkgs; [
    git
    curl
    wget
    htop
    tree
    bat
    nh
    tealdeer
    unzip
    fastfetch
    brave
    floorp-bin
    vscodium
    neovim
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
    pokeget-rs
    (writeShellScriptBin "chromium-browser" ''
      if pgrep floorp > /dev/null; then
        floorp "$@" &
        exit 0
      fi
      exec floorp "$@"
    '')
    (writeShellScriptBin "x-www-browser" ''
      if pgrep floorp > /dev/null; then
        floorp "$@" &
        exit 0
      fi
      exec floorp "$@"
    '')
  ];
}
