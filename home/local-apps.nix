{ pkgs, lib, ... }:

let
  glibcPath = "${pkgs.glibc}/lib/ld-linux-x86-64.so.2";
  rpath = lib.makeLibraryPath [
    pkgs.openssl
    pkgs.wayland
    pkgs.libxkbcommon
    pkgs.libGL
    pkgs.gcc.cc.lib
  ];
in

{
  home.activation.localApps = lib.hm.dag.entryAfter ["writeBoundary"] ''
    mkdir -p "$HOME/.bin"

    # uwuplsplay
    if [ ! -f "$HOME/.bin/uwuplsplay" ]; then
      ${pkgs.curl}/bin/curl -L \
        "https://github.com/ilovealienz/uwuplsplay/releases/latest/download/uwuplsplay-linux" \
        -o "$HOME/.bin/uwuplsplay"
      ${pkgs.patchelf}/bin/patchelf --set-interpreter ${glibcPath} "$HOME/.bin/uwuplsplay"
    fi
    chmod +x "$HOME/.bin/uwuplsplay"

    # stremio-cliuwu
    if [ ! -f "$HOME/.bin/stremio-cliuwu" ]; then
      ${pkgs.curl}/bin/curl -L \
        "https://github.com/ilovealienz/stremio-cliuwu/releases/latest/download/stremio-cliuwu-linux" \
        -o "$HOME/.bin/stremio-cliuwu"
      ${pkgs.patchelf}/bin/patchelf --set-interpreter ${glibcPath} "$HOME/.bin/stremio-cliuwu"
    fi
    chmod +x "$HOME/.bin/stremio-cliuwu"

    # zipline-upload
    if [ ! -f "$HOME/.bin/zipline-upload" ]; then
      ${pkgs.curl}/bin/curl -L \
        "https://github.com/ilovealienz/my-zipline-uploader/releases/latest/download/zipline-upload" \
        -o "$HOME/.bin/zipline-upload"
      ${pkgs.patchelf}/bin/patchelf \
        --set-interpreter ${glibcPath} \
        --set-rpath ${rpath} \
        "$HOME/.bin/zipline-upload"
    fi
    chmod +x "$HOME/.bin/zipline-upload"

    # uwuplsplay mime/protocol registration
    mkdir -p "$HOME/.local/share/applications" "$HOME/.local/share/mime/packages"

    cat > "$HOME/.local/share/applications/uwuplsplay.desktop" << EOF
[Desktop Entry]
Name=uwuplsplay
Exec=$HOME/.bin/uwuplsplay %u
MimeType=application/x-uwuplsplay;x-scheme-handler/uwupls;
Type=Application
NoDisplay=true
EOF

    cat > "$HOME/.local/share/mime/packages/uwuplsplay.xml" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<mime-info xmlns="http://www.freedesktop.org/standards/shared-mime-info">
  <mime-type type="application/x-uwuplsplay">
    <comment>uwuplsplay stream link</comment>
    <glob pattern="*.uwuplsplay"/>
  </mime-type>
</mime-info>
EOF

    ${pkgs.shared-mime-info}/bin/update-mime-database "$HOME/.local/share/mime" >/dev/null 2>&1
    ${pkgs.xdg-utils}/bin/xdg-mime default uwuplsplay.desktop application/x-uwuplsplay
    ${pkgs.xdg-utils}/bin/xdg-mime default uwuplsplay.desktop x-scheme-handler/uwupls
    ${pkgs.desktop-file-utils}/bin/update-desktop-database "$HOME/.local/share/applications" >/dev/null 2>&1

    # enable-pc-host script
    cat > "$HOME/enable-pc-host.sh" << 'EOF'
#!/usr/bin/env bash
# Uncomments pc-system.nix for this machine
sudo sed -i 's|# ./hosts/pc-system.nix|./hosts/pc-system.nix|' /etc/nixos/configuration.nix
echo "Done. Run 'nxrebuild' to apply."
EOF
    chmod +x "$HOME/enable-pc-host.sh"
  '';
}
