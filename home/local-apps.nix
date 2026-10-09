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

  # Downloads missing release binaries into ~/.bin and patches them.
  # Runs as a user service once the network is up, instead of during
  # home-manager activation, where a rebuild restarting NetworkManager
  # left no DNS. A failed download only warns; the next login or
  # rebuild tries again.
  fetchApps = pkgs.writeShellApplication {
    name = "local-apps-fetch";
    runtimeInputs = [ pkgs.curl pkgs.patchelf pkgs.coreutils pkgs.networkmanager ];
    text = ''
      mkdir -p "$HOME/.bin"

      # only wait for the network if something actually needs downloading
      for f in uwuplsplay stremio-cliuwu zipline-upload; do
        if [ ! -f "$HOME/.bin/$f" ]; then
          nm-online -q -t 60 || echo "offline, skipping downloads" >&2
          break
        fi
      done

      # temp file first, so a failed download never ends up in ~/.bin
      fetch() {
        local dest="$HOME/.bin/$1" url=$2
        [ -f "$dest" ] && return 0
        if curl -fsSL --retry 2 --connect-timeout 10 "$url" -o "$dest.part"; then
          mv "$dest.part" "$dest"
        else
          rm -f "$dest.part"
          echo "couldn't download $1" >&2
        fi
      }
      # patchelf fails on static binaries (no .interp), that's fine
      fix() {
        local f="$HOME/.bin/$1"; shift
        [ -f "$f" ] || return 0
        patchelf --set-interpreter ${glibcPath} "$@" "$f" || true
        chmod +x "$f"
      }

      fetch uwuplsplay "https://github.com/ilovealienz/uwuplsplay/releases/latest/download/uwuplsplay-linux"
      fetch stremio-cliuwu "https://github.com/ilovealienz/stremio-cliuwu/releases/latest/download/stremio-cliuwu-linux-amd64"
      fetch zipline-upload "https://github.com/ilovealienz/my-zipline-uploader/releases/latest/download/zipline-upload"
      fix uwuplsplay
      fix stremio-cliuwu
      fix zipline-upload --set-rpath ${rpath}
    '';
  };
in
{
  systemd.user.services.local-apps = {
    Unit.Description = "Download local app binaries into ~/.bin";
    Service = {
      Type = "oneshot";
      RemainAfterExit = true;
      ExecStart = lib.getExe fetchApps;
    };
    # runs at login, and again on any rebuild that changes this script
    Install.WantedBy = [ "default.target" ];
  };

  # uwuplsplay mime/protocol registration (local only, no network needed)
  home.activation.localApps = lib.hm.dag.entryAfter ["writeBoundary"] ''
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
    ${pkgs.shared-mime-info}/bin/update-mime-database "$HOME/.local/share/mime" >/dev/null 2>&1 || true
    ${pkgs.xdg-utils}/bin/xdg-mime default uwuplsplay.desktop application/x-uwuplsplay || true
    ${pkgs.xdg-utils}/bin/xdg-mime default uwuplsplay.desktop x-scheme-handler/uwupls || true
    ${pkgs.desktop-file-utils}/bin/update-desktop-database "$HOME/.local/share/applications" >/dev/null 2>&1 || true
  '';
}
