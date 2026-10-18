# nxsecrets: the few private files that aren't in the config (the weather
# key, the birthday list) kept in the repo, encrypted:
#
#   secrets.age      the files, encrypted to a key
#   secrets.key.age  that key, locked with a passphrase (keep it in Bitwarden)
#   secrets.pub      the key's public half, all that encrypting needs
#
# Each machine asks for the passphrase once, on its first unpack, and keeps
# the unlocked key in ~/.local/state/nxsecrets (as private as the files
# themselves in ~/.config). Enough to keep them from anyone browsing the
# repo; not meant for anything serious.
#
#   nxsecrets init              set it up (once, on the pc)
#   nxsecrets unpack            put the repo's copies in place; one you
#                               edited here that also changed in the repo
#                               is kept, with a warning
#   nxsecrets unpack --replace  take the repo's copies over yours anyway
#   nxsecrets pack              encrypt this machine's copies into the repo
#   nxsecrets rekey             a new key and passphrase (a forgotten or
#                               leaked one: run it on a machine that has
#                               unpacked; the old passphrase still opens the
#                               old key in the git history, so reroll the
#                               weather key too if it leaked)
#
# nxpush runs `nxsecrets pack --if-changed` (no passphrase: it only
# re-encrypts when a file changed here, and stops the push if the repo's
# copy is newer); nxpull runs `nxsecrets unpack --if-changed`. On the pc,
# ~/.config/birthdays can be a link to the share.
{ pkgs, ... }:
let
  nxsecrets = pkgs.writeShellApplication {
    name = "nxsecrets";
    # (sudo is NixOS's own, from the PATH)
    runtimeInputs = with pkgs; [ age gnutar coreutils gawk procps ];
    text = ''
      # everything made here is private to you; the repo files are made
      # readable on purpose
      umask 077

      # the files, from your home folder
      files=(.config/weather/key .config/birthdays)
      repo=/etc/nixos
      enc="$repo/secrets.age"     # the files, encrypted to the key
      lock="$repo/secrets.key.age" # the key, locked with the passphrase
      pubf="$repo/secrets.pub"     # the key's public half: encrypting needs only this
      # this machine's unlocked key, what it had the last time it matched the
      # repo ("path sha256" per file), and the repo file's sha256 then
      state="''${XDG_STATE_HOME:-$HOME/.local/state}/nxsecrets"
      key="$state/key"
      # what pack encrypts to: the repo's public key (a new one, while init or
      # rekey make it)
      pubsrc="$pubf"
      mkdir -p "$state"
      cd "$HOME"

      # temporary files, removed however this ends (Ctrl+C included)
      tmp="" out=""
      cleanup() {
        [ -n "$tmp" ] && rm -rf "$tmp"
        [ -n "$out" ] && rm -f "$out"
        # a key being unlocked or made, if this stopped part way
        rm -f "$key.new" "$state/lock.new" "$state/pub.new"
        return 0
      }
      trap cleanup EXIT

      sha() { sha256sum < "$1" | cut -d' ' -f1; }
      # a link whose target isn't there (the pc's share not mounted)
      dangling() { [ -L "$1" ] && [ ! -e "$1" ]; }
      # each file here (through links) as "path sha256"
      fingerprint() {
        local f
        for f in "''${files[@]}"; do
          [ -e "$f" ] && printf '%s %s\n' "$f" "$(sha "$f")"
        done
        return 0
      }
      # a file's sha256 the last time this machine matched the repo
      last() { [ -f "$state/fingerprint" ] && awk -v f="$1" '$1 == f { print $2 }' "$state/fingerprint"; return 0; }
      synced() { sha "$enc" > "$state/repo"; }
      # the repo's copy changed since then (pulled from another machine)
      moved() { [ -f "$enc" ] && [ -f "$state/repo" ] && [ "$(sha "$enc")" != "$(cat "$state/repo")" ]; }
      # the unlocked key here is the repo's current one
      key_ok() { [ -f "$key" ] && [ -f "$pubf" ] && [ "$(age-keygen -y "$key" 2>/dev/null)" = "$(cat "$pubf")" ]; }

      # into /etc/nixos, readable (git and the other machines need it): written
      # beside it, then moved over in one go; sudo where /etc/nixos is root's
      put_repo() {
        chmod 644 "$1"
        if ! { cp "$1" "$2.nxsecrets-new" && chmod 644 "$2.nxsecrets-new" &&
               mv -f "$2.nxsecrets-new" "$2"; } 2>/dev/null; then
          rm -f "$2.nxsecrets-new" 2>/dev/null || true
          sudo cp "$1" "$2.nxsecrets-new" && sudo chmod 644 "$2.nxsecrets-new" &&
            sudo mv -f "$2.nxsecrets-new" "$2"
        fi
      }

      # this machine's unlocked key, asking for the passphrase if it doesn't
      # have the current one
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

      # a new key, locked with a passphrase you pick. Nothing goes in the repo
      # yet: the files are encrypted to it first (newkey, pack, usekey), so if
      # that fails the old key still opens what's there
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
      # the new key into the repo and this machine, once secrets.age uses it
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
            # never synced here: this machine's copies may be old or missing
            # some, so leave the repo's alone
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
        # the repo's is newer: packing would put an old copy over it
        if [ $force = 0 ] && moved; then
          echo "nxsecrets: secrets.age changed in the repo since this machine last had it;" >&2
          echo "  nxsecrets unpack first (or nxsecrets pack --force to put yours over it)" >&2
          return 1
        fi
        # one that was here before and isn't now would drop out of the repo for
        # every machine: left as it is instead
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
        # encrypted to the public key: no passphrase needed
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
          # (nxpull: no passphrase prompt in the middle of it)
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
            # changed here since this machine last matched the repo (if not,
            # it's just old, and the repo's goes in)
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
          # through a link (the pc's birthdays on the share), the file it points
          # to; written beside it, then moved over in one go
          dest=$(readlink -f "$f")
          # never synced here: the repo's wins, yours kept beside it
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
        # the bar's cake, now rather than in up to 10 minutes
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
        # needs this machine's current files and key: the files are encrypted
        # again, and an old passphrase (still good for the old key in the git
        # history) opens nothing new
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
