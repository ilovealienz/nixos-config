# nxsecrets: the weather key and birthdays, encrypted in the repo.
#
#   secrets.age      the files, encrypted to a key
#   secrets.key.age  that key, locked with a passphrase (in Bitwarden)
#   secrets.pub      the public key, for encrypting
#
#   nxsecrets init              set up (once)
#   nxsecrets unpack            repo's copies into place (local edits kept)
#   nxsecrets unpack --replace  repo's copies over yours
#   nxsecrets pack              encrypt this machine's copies into the repo
#   nxsecrets rekey             new key and passphrase (old ones still open
#                               the git history)
#
# nxpush packs and nxpull unpacks when something changed.
{ pkgs, ... }:
let
  nxsecrets = pkgs.writeShellApplication {
    name = "nxsecrets";
    # sudo comes from the system PATH
    runtimeInputs = with pkgs; [ age gnutar coreutils gawk procps ];
    text = ''
      # private by default; repo files are made readable on purpose
      umask 077

      # relative to home
      files=(.config/weather/key .config/birthdays)
      repo=/etc/nixos
      enc="$repo/secrets.age"
      lock="$repo/secrets.key.age"
      pubf="$repo/secrets.pub"
      # unlocked key and last-synced fingerprints
      state="''${XDG_STATE_HOME:-$HOME/.local/state}/nxsecrets"
      key="$state/key"
      # public key pack encrypts to (a new one during init/rekey)
      pubsrc="$pubf"
      mkdir -p "$state"
      cd "$HOME"

      # temp files, removed on any exit
      tmp="" out=""
      cleanup() {
        [ -n "$tmp" ] && rm -rf "$tmp"
        [ -n "$out" ] && rm -f "$out"
        rm -f "$key.new" "$state/lock.new" "$state/pub.new"
        return 0
      }
      trap cleanup EXIT

      sha() { sha256sum < "$1" | cut -d' ' -f1; }
      # a broken link (share not mounted)
      dangling() { [ -L "$1" ] && [ ! -e "$1" ]; }
      # "path sha256" per file
      fingerprint() {
        local f
        for f in "''${files[@]}"; do
          [ -e "$f" ] && printf '%s %s\n' "$f" "$(sha "$f")"
        done
        return 0
      }
      # a file's sha256 at the last sync
      last() { [ -f "$state/fingerprint" ] && awk -v f="$1" '$1 == f { print $2 }' "$state/fingerprint"; return 0; }
      synced() { sha "$enc" > "$state/repo"; }
      # the repo's copy changed since the last sync
      moved() { [ -f "$enc" ] && [ -f "$state/repo" ] && [ "$(sha "$enc")" != "$(cat "$state/repo")" ]; }
      key_ok() { [ -f "$key" ] && [ -f "$pubf" ] && [ "$(age-keygen -y "$key" 2>/dev/null)" = "$(cat "$pubf")" ]; }

      # atomic write into the repo, readable; sudo if needed
      put_repo() {
        chmod 644 "$1"
        if ! { cp "$1" "$2.nxsecrets-new" && chmod 644 "$2.nxsecrets-new" &&
               mv -f "$2.nxsecrets-new" "$2"; } 2>/dev/null; then
          rm -f "$2.nxsecrets-new" 2>/dev/null || true
          sudo cp "$1" "$2.nxsecrets-new" && sudo chmod 644 "$2.nxsecrets-new" &&
            sudo mv -f "$2.nxsecrets-new" "$2"
        fi
      }

      # unlock the key, asking for the passphrase if needed
      unlock() {
        key_ok && return 0
        [ -f "$lock" ] || { echo "nxsecrets: no $lock (nxsecrets init sets it up)" >&2; return 1; }
        echo "nxsecrets: passphrase for the secrets key (the one in Bitwarden):"
        rm -f "$key.new"
        if ! age -d -o "$key.new" "$lock"; then
          rm -f "$key.new"
          echo "nxsecrets: not unlocked (wrong passphrase?)" >&2
          return 1
        fi
        mv -f "$key.new" "$key" || return 1
        key_ok || { echo "nxsecrets: $lock and $pubf don't match" >&2; return 1; }
      }

      # a new key; only put in the repo after pack succeeds (usekey)
      newkey() {
        rm -f "$key.new" "$state/lock.new" "$state/pub.new"
        age-keygen -o "$key.new" 2>/dev/null || return 1
        age-keygen -y "$key.new" > "$state/pub.new" || return 1
        echo "nxsecrets: pick a passphrase for the secrets key and save it in Bitwarden"
        echo "  (leave it empty and age makes one up: it shows it once, save that)"
        if ! age -p -o "$state/lock.new" "$key.new"; then
          echo "nxsecrets: no key made" >&2
          return 1
        fi
        pubsrc="$state/pub.new"
      }
      usekey() {
        if ! { put_repo "$state/lock.new" "$lock" && put_repo "$state/pub.new" "$pubf" &&
               mv -f "$key.new" "$key"; }; then
          echo "nxsecrets: couldn't put the new key in the repo; run it again" >&2
          return 1
        fi
        rm -f "$state/lock.new" "$state/pub.new"
      }

      pack() {
        local now a f force=0 only_changed=0
        for a in "$@"; do
          case "$a" in
            --if-changed) only_changed=1 ;;
            --force) force=1 ;;
          esac
        done
        if [ ! -f "$pubsrc" ]; then
          [ $only_changed = 1 ] && return 0
          echo "nxsecrets: not set up yet (nxsecrets init)" >&2
          return 1
        fi
        now=$(fingerprint)
        if [ $only_changed = 1 ]; then
          if [ ! -f "$state/repo" ] && [ -f "$enc" ]; then
            # never synced here: leave the repo's alone
            echo "nxsecrets: secrets.age left as it is (nxsecrets unpack once on this machine)" >&2
            return 0
          fi
          [ -f "$state/fingerprint" ] && [ "$now" = "$(cat "$state/fingerprint")" ] && return 0
        fi
        if [ -z "$now" ]; then
          [ $only_changed = 1 ] && return 0
          echo "nxsecrets: none of ''${files[*]} here, nothing to pack" >&2
          return 1
        fi
        # the repo's is newer
        if [ $force = 0 ] && moved; then
          echo "nxsecrets: secrets.age changed in the repo since this machine last had it;" >&2
          echo "  nxsecrets unpack first (or nxsecrets pack --force to put yours over it)" >&2
          return 1
        fi
        # a missing file would drop out of the repo for every machine
        local gone=() broken=()
        if [ -f "$state/fingerprint" ]; then
          while read -r f _; do
            if dangling "$f"; then
              broken+=("$f")
            elif [ ! -e "$f" ] && [ $force = 0 ]; then
              gone+=("$f")
            fi
          done < "$state/fingerprint"
        fi
        for f in "''${files[@]}"; do
          if dangling "$f" && [[ ! " ''${broken[*]} " == *" $f "* ]]; then broken+=("$f"); fi
        done
        if [ "''${#broken[@]}" -gt 0 ]; then
          echo "nxsecrets: ''${broken[*]} links to a file that isn't there (is the share mounted?);" >&2
          echo "  secrets.age left as it is" >&2
          [ $only_changed = 1 ] && return 0
          return 1
        fi
        if [ "''${#gone[@]}" -gt 0 ]; then
          echo "nxsecrets: ''${gone[*]} missing here, so secrets.age left as it is;" >&2
          echo "  nxsecrets unpack gets it back, nxsecrets pack --force drops it from the repo" >&2
          [ $only_changed = 1 ] && return 0
          return 1
        fi
        local have=()
        for f in "''${files[@]}"; do [ -e "$f" ] && have+=("$f"); done
        out=$(mktemp)
        # public key: no passphrase needed
        if ! tar -ch "''${have[@]}" | age -r "$(cat "$pubsrc")" -o "$out"; then
          echo "nxsecrets: not encrypted" >&2
          return 1
        fi
        put_repo "$out" "$enc" || return 1
        rm -f "$out"
        out=""
        printf '%s\n' "$now" > "$state/fingerprint"
        synced
        echo "nxsecrets: secrets.age updated (''${have[*]})"
      }

      unpack() {
        local a replace=0 only_changed=0
        for a in "$@"; do
          case "$a" in
            --replace) replace=1 ;;
            --if-changed) only_changed=1 ;;
          esac
        done
        if [ ! -f "$enc" ]; then
          [ $only_changed = 1 ] && return 0
          echo "nxsecrets: no $enc yet (nxsecrets init sets it up)" >&2
          return 1
        fi
        if [ $only_changed = 1 ]; then
          [ -f "$state/repo" ] && [ "$(sha "$enc")" = "$(cat "$state/repo")" ] && return 0
          # no passphrase prompt during nxpull
          if ! key_ok; then
            echo "nxsecrets: secrets.age changed: nxsecrets unpack for the new weather key / birthdays"
            return 0
          fi
        fi
        unlock || return 1
        local f dest theirs mine prev new="" fresh=0 birthdays=0
        [ -f "$state/repo" ] || fresh=1
        tmp=$(mktemp -d)
        if ! age -d -i "$key" "$enc" | tar -x -C "$tmp"; then
          echo "nxsecrets: couldn't decrypt secrets.age" >&2
          return 1
        fi
        for f in "''${files[@]}"; do
          prev=$(last "$f")
          if dangling "$f"; then
            echo "  $f: links to a file that isn't there (is the share mounted?), skipped"
            [ -n "$prev" ] && new+="$f $prev"$'\n'
            continue
          fi
          [ -e "$tmp/$f" ] || continue
          theirs=$(sha "$tmp/$f")
          if [ -e "$f" ]; then
            mine=$(sha "$f")
            if [ "$mine" = "$theirs" ]; then
              echo "  $f: up to date"
              new+="$f $theirs"$'\n'
              continue
            fi
            # changed here since the last sync: keep it
            if [ $replace = 0 ] && [ $fresh = 0 ] && [ "$mine" != "$prev" ]; then
              if [ "$theirs" = "$prev" ]; then
                echo "  $f: yours is newer, kept (nxpush puts it in the repo)"
              else
                echo "  $f: kept yours (changed here and in the repo; nxpush puts yours in the repo,"
                echo "    nxsecrets unpack --replace takes the repo's)"
              fi
              new+="$f $prev"$'\n'
              continue
            fi
          fi
          mkdir -p "$(dirname "$f")"
          # follows links; atomic write
          dest=$(readlink -f "$f")
          # first sync: back up yours
          if [ -e "$f" ] && [ $fresh = 1 ] && [ $replace = 0 ]; then
            cp "$dest" "$dest.before-nxsecrets"
            echo "  $f: yours differed, saved as $(basename "$dest").before-nxsecrets"
          fi
          cp "$tmp/$f" "$dest.nxsecrets-new"
          mv -f "$dest.nxsecrets-new" "$dest"
          chmod 600 "$dest" 2>/dev/null || true
          echo "  $f: put in place"
          new+="$f $theirs"$'\n'
          [ "$f" = .config/birthdays ] && birthdays=1
        done
        printf '%s' "$new" > "$state/fingerprint"
        synced
        # refresh the bar's birthday icon
        [ $birthdays = 1 ] && { pkill -RTMIN+11 waybar 2>/dev/null || true; }
        return 0
      }

      init() {
        if [ -f "$lock" ]; then
          echo "nxsecrets: already set up: nxsecrets unpack on a new machine," >&2
          echo "  nxsecrets rekey for a new key and passphrase" >&2
          return 1
        fi
        [ -n "$(fingerprint)" ] || { echo "nxsecrets: none of ''${files[*]} here yet" >&2; return 1; }
        newkey || return 1
        pack --force || { echo "nxsecrets: not set up" >&2; return 1; }
        usekey
      }

      rekey() {
        # needs the current files and key
        if moved; then
          echo "nxsecrets: secrets.age changed in the repo; nxsecrets unpack first" >&2
          return 1
        fi
        if ! key_ok; then
          echo "nxsecrets: this machine doesn't have the current key; nxsecrets unpack first" >&2
          echo "  (forgot the passphrase? rekey from a machine that's unpacked before)" >&2
          return 1
        fi
        newkey || return 1
        pack --force || { echo "nxsecrets: kept the old key" >&2; return 1; }
        usekey || return 1
        echo "nxsecrets: new key; other machines ask for the new passphrase on their next unpack"
      }

      case "''${1:-}" in
        init) init ;;
        rekey) rekey ;;
        pack) shift; pack "$@" ;;
        unpack) shift; unpack "$@" ;;
        *)
          echo "usage: nxsecrets init | unpack [--replace] | pack [--force] | rekey" >&2
          echo "  files: ''${files[*]} (in your home folder)" >&2
          exit 1
          ;;
      esac
    '';
  };
in
{
  home.packages = [ nxsecrets ];
}
