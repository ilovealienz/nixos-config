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

  # Keeps release binaries in ~/.bin up to date. For each app it asks
  # GitHub's API for the latest release, and downloads the asset when
  # it's missing or changed since last time (by the asset's sha256 digest,
  # or for assets without one, the release tag and upload time). The
  # download is checked against that digest before it replaces anything,
  # and patched while it's still a temporary file, so ~/.bin never has a
  # half-downloaded or unpatched binary. What's installed is kept in
  # ~/.bin/.versions. Offline, or past GitHub's limit (60 checks an hour),
  # it keeps what's there, and only fetches missing apps the old way
  # (latest/download, unchecked).
  #
  # Runs at login, once a day, and on `nxapps`.
  fetchApps = pkgs.writeShellApplication {
    name = "local-apps-fetch";
    runtimeInputs = with pkgs; [ curl jq patchelf coreutils util-linux networkmanager libnotify ];
    text = ''
      bin="$HOME/.bin" state="$HOME/.bin/.versions"
      mkdir -p "$bin"
      touch "$state"
      # one run at a time (login, the timer and nxapps can overlap)
      exec 9>"$bin/.lock"
      flock 9

      nm-online -q -t 60 || echo "network not up yet, trying anyway" >&2

      # the key stored for an app, if any
      have() { awk -v n="$1" '$1 == n { print $2 }' "$state"; }
      forget() {
        { grep -v "^$1 " "$state" || true; } > "$state.new"
        mv "$state.new" "$state"
      }
      remember() {
        forget "$1"
        echo "$1 $2" >> "$state"
      }

      # app, owner/repo, asset name, patchelf options
      update() {
        local name=$1 repo=$2 asset=$3; shift 3
        local dest="$bin/$name" part="$bin/.$name.part" json url digest key
        if ! json=$(curl -fsSL --retry 2 --connect-timeout 10 --max-time 30 \
            -H "Accept: application/vnd.github+json" \
            "https://api.github.com/repos/$repo/releases/latest" 2>/dev/null) \
            || ! jq -e '.tag_name and (.assets | type == "array")' >/dev/null 2>&1 <<<"$json"; then
          # offline, over the limit, or not GitHub's answer (a wifi login
          # page, say): never fed to the rest, which would stop the script
          if [ -f "$dest" ]; then
            echo "$name: can't check for updates, keeping it" >&2
            return 0
          fi
          json=""
        fi
        if [ -n "$json" ]; then
          url=$(jq -r --arg a "$asset" '.assets[] | select(.name == $a) | .browser_download_url' <<<"$json")
          digest=$(jq -r --arg a "$asset" '.assets[] | select(.name == $a) | .digest // empty' <<<"$json")
          key=$(jq -r --arg a "$asset" '.tag_name as $t | .assets[] | select(.name == $a)
            | (.digest // ($t + "@" + .updated_at))' <<<"$json")
          if [ -z "$url" ]; then
            echo "$name: the latest release has no $asset" >&2
            return 0
          fi
          if [ -f "$dest" ] && [ "$(have "$name")" = "$key" ]; then
            return 0
          fi
        else
          url="https://github.com/$repo/releases/latest/download/$asset" digest="" key=""
        fi

        # gives up if it stalls (under 1 kB/s for a minute), so a stuck
        # download can't hold the lock forever
        if ! curl -fsSL --retry 2 --connect-timeout 10 --speed-limit 1024 --speed-time 60 \
            "$url" -o "$part"; then
          rm -f "$part"
          echo "$name: couldn't download" >&2
          return 0
        fi
        if [ -n "$digest" ] && [ "sha256:$(sha256sum "$part" | cut -d' ' -f1)" != "$digest" ]; then
          rm -f "$part"
          echo "$name: download doesn't match its sha256, not installed" >&2
          return 0
        fi
        # all three are Linux programs: anything else (a wifi login page
        # instead of the file, say) isn't installed
        if [ "$(head -c 4 "$part" | od -An -tx1 | tr -d ' \n')" != 7f454c46 ]; then
          rm -f "$part"
          echo "$name: the download isn't a Linux program, not installed" >&2
          return 0
        fi
        # patchelf fails on static binaries (no .interp), that's fine
        patchelf --set-interpreter ${glibcPath} "$@" "$part" 2>/dev/null || true
        chmod +x "$part"
        # an update (not a first install, or the first check after this
        # was set up, which fetches everything once)
        local was=""
        [ -f "$dest" ] && [ -n "$(have "$name")" ] && was=1
        # a running copy keeps going; the next start gets the new one
        mv "$part" "$dest"
        # without a key (fetched unchecked), the next check fetches it again
        if [ -n "$key" ]; then remember "$name" "$key"; else forget "$name"; fi
        local tag
        tag=$(jq -r '.tag_name // empty' <<<"$json" 2>/dev/null || true)
        if [ -n "$was" ]; then
          echo "$name: updated''${tag:+ to $tag}"
          notify-send -a apps "$name updated" "''${tag:-new version}" 2>/dev/null || true
        else
          echo "$name: installed''${tag:+ $tag}"
        fi
      }

      update uwuplsplay ilovealienz/uwuplsplay uwuplsplay-linux
      update stremio-cliuwu ilovealienz/stremio-cliuwu stremio-cliuwu-linux-amd64
      update zipline-upload ilovealienz/my-zipline-uploader zipline-upload --set-rpath ${rpath}
    '';
  };
in
{
  # nxapps: check for updates now
  home.packages = [ fetchApps ];
  programs.zsh.shellAliases.nxapps = "local-apps-fetch";

  systemd.user.services.local-apps = {
    Unit.Description = "Download local app binaries into ~/.bin";
    Service = {
      Type = "oneshot";
      RemainAfterExit = true;
      ExecStart = lib.getExe fetchApps;
      TimeoutStartSec = "15min";
    };
    # runs at login, and again on any rebuild that changes this script
    Install.WantedBy = [ "default.target" ];
  };

  # the daily check: its own service, since local-apps stays "active"
  # after it runs and starting it again would do nothing
  systemd.user.services.local-apps-update = {
    Unit.Description = "Check local app binaries for updates";
    Service = {
      Type = "oneshot";
      ExecStart = lib.getExe fetchApps;
      TimeoutStartSec = "15min";
    };
  };
  systemd.user.timers.local-apps-update = {
    Unit.Description = "Check local app binaries for updates daily";
    Timer = {
      OnCalendar = "daily";
      Persistent = true;
      RandomizedDelaySec = "1h";
    };
    Install.WantedBy = [ "timers.target" ];
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
