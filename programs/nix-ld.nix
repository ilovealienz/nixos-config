# nix-ld: generic Linux binaries (ldrun, ~/.bin) find these libraries
{ pkgs, ... }:
{
  programs.nix-ld.enable = true;
  programs.nix-ld.libraries = with pkgs; [
    wayland
    libxkbcommon
    libGL
    mesa
    vulkan-loader
    libx11
    libxcursor
    libxrandr
    libxi
    fontconfig
    freetype
    openssl
    gtk3
    pango
    harfbuzz
    atk
    cairo
    gdk-pixbuf
    glib
    zlib
    stdenv.cc.cc.lib
    libxcb
    libpulseaudio
    alsa-lib
    pipewire
    libjack2
  ];
}
