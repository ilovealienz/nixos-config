{ config, pkgs, ... }:

let
  c = config.theme.colors;

  # browser chooser, plus shims for the generic names xdg-open tries
  urlopen = pkgs.writeShellScriptBin "urlopen" (builtins.replaceStrings
    [ "@bg@" "@fg@" "@accent@" "@onAccent@" ] [ c.bg c.fg c.accent c.onAccent ]
    (builtins.readFile ../scripts/urlopen));

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
    fastfetch
    alsa-utils
    (brave.override { commandLineArgs = "--ozone-platform=x11"; })
    librewolf-bin
    vscodium
    neovim
    mission-center
    gnome-disk-utility
    proton-vpn
    xdg-utils
    fzf
    jq
    nix-search-tv
    gparted
    wireguard-tools
    pokeget-rs
    localsend
    urlopen
    urlopenDesktop
    qrencode   # the chooser's "qr"
    (browserShim "chromium-browser")
    (browserShim "x-www-browser")
  ];
}
