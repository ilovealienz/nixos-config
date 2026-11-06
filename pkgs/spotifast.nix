# Spotifast: a native Spotify client (github.com/crmne/spotifast), from its
# prebuilt release
{ stdenv, lib, fetchurl, autoPatchelfHook, makeWrapper, pipewire, alsa-lib, dbus, libxkbcommon, wayland, libGL }:
stdenv.mkDerivation rec {
  pname = "spotifast";
  version = "0.17.1";
  src = fetchurl {
    url = "https://github.com/crmne/spotifast/releases/download/v${version}/spotifast-v${version}-x86_64-unknown-linux-gnu.tar.gz";
    hash = "sha256-ZRcC1PGL+cM47/lhN3I8Ei4Yf9SJAq9L5WLcmwZMSXA=";
  };
  nativeBuildInputs = [ autoPatchelfHook makeWrapper ];
  buildInputs = [ pipewire alsa-lib stdenv.cc.cc.lib ];
  installPhase = ''
    install -Dm755 spotifast $out/bin/spotifast
    install -Dm644 packaging/applications/spotifast.desktop $out/share/applications/spotifast.desktop
    install -Dm644 packaging/icons/spotifast.svg $out/share/icons/hicolor/scalable/apps/spotifast.svg
  '';
  # the window, GL and D-Bus libraries are loaded at run time
  postFixup = ''
    wrapProgram $out/bin/spotifast \
      --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath [ dbus libxkbcommon wayland libGL ]}
  '';
  meta.mainProgram = "spotifast";
}
