{ pkgs, ... }:

let
  # Browser chooser. xdg-open falls back to scanning for generic binary
  # names when no desktop handler matches, so the shims provide those
  # names and route them here.
  urlopen = pkgs.writeShellScriptBin "urlopen" (builtins.readFile ../scripts/urlopen);

  urlopenDesktop = pkgs.makeDesktopItem {
    name = "urlopen";
    desktopName = "Browser chooser";
    exec = "urlopen %U";
    noDisplay = true;
    mimeTypes = [
      "x-scheme-handler/http"
      "x-scheme-handler/https"
      "text/html"
    ];
  };

  browserShim = name:
    pkgs.writeShellScriptBin name ''exec ${urlopen}/bin/urlopen "$@"'';
in

{
  environment.systemPackages = with pkgs; [
    git
    curl
    killall
    wget
    htop
    tree
    bat
    fd
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
    localsend
    urlopen
    urlopenDesktop
    (browserShim "chromium-browser")
    (browserShim "x-www-browser")
  ];
}
