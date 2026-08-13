#!/usr/bin/env bash
# nixos-config bootstrap
# usage: chmod +x bootstrap.sh && sudo ./bootstrap.sh
set -uo pipefail

REPO="https://github.com/ilovealienz/nixos-config"
NIXOS_DIR="/etc/nixos"
TMP_CLONE="/tmp/nixos-config-clone"
USER_NAME="pc"

# flakes are off on a fresh install — enable for everything this script runs
export NIX_CONFIG="experimental-features = nix-command flakes"

B=$'\e[1m'; R=$'\e[31m'; G=$'\e[32m'; Y=$'\e[33m'; C=$'\e[36m'; X=$'\e[0m'
say()  { echo "${C}==>${X} $*"; }
ok()   { echo "${G} ok${X} $*"; }
warn() { echo "${Y} !!${X} $*"; }
err()  { echo "${R}!!!${X} $*"; }

[ "$EUID" -ne 0 ] && { err "run with sudo: sudo ./bootstrap.sh"; exit 1; }

# ─────────────────────────────────────────────────────────────────────
# install
# ─────────────────────────────────────────────────────────────────────
install() {
  # don't clobber an existing checkout
  if [ -d "$NIXOS_DIR/.git" ]; then
    warn "$NIXOS_DIR is already a git repo"
    echo "   'full install' would move it aside and re-clone,"
    echo "   losing anything uncommitted."
    read -rp "   continue anyway? [y/N] " a
    [[ "$a" =~ ^[Yy]$ ]] || return 0
  fi

  # warn if the account this config manages isn't the one you're using
  if [ "${SUDO_USER:-root}" != "$USER_NAME" ] && [ "${SUDO_USER:-root}" != "root" ]; then
    echo
    warn "you're '${SUDO_USER}', but this config manages '${USER_NAME}'"
    echo "   nothing gets deleted — you'll just have two accounts."
    echo "   log in as '${USER_NAME}' afterwards; '${SUDO_USER}' stays as a fallback."
    read -rp "   continue? [y/N] " a
    [[ "$a" =~ ^[Yy]$ ]] || return 0
  fi

  say "cloning config..."
  local backup="/etc/nixos-backup-$(date +%s)"
  nix-shell -p git --run "bash -s" <<EOF || { err "clone failed"; return 1; }
set -euo pipefail
shopt -s dotglob
rm -rf '$TMP_CLONE'
git clone '$REPO' '$TMP_CLONE'
mkdir -p '$backup'
mv '$NIXOS_DIR'/* '$backup'/ 2>/dev/null || true
mv '$TMP_CLONE'/* '$NIXOS_DIR'/
rmdir '$TMP_CLONE'
git config --global --add safe.directory '$NIXOS_DIR'
EOF
  ok "cloned (old config backed up to $backup)"

  # pick a host from what's actually in the repo
  local hosts=()
  mapfile -t hosts < <(ls "$NIXOS_DIR/hosts" 2>/dev/null | sed 's/\.nix$//')
  [ ${#hosts[@]} -eq 0 ] && { err "no host files in $NIXOS_DIR/hosts"; return 1; }
  echo
  say "which machine is this?"
  local HOST
  select HOST in "${hosts[@]}"; do [ -n "${HOST:-}" ] && break; done

  say "generating hardware config..."
  nixos-generate-config --show-hardware-config > "$NIXOS_DIR/hardware-configuration.nix"

  say "staging files (untracked files are invisible to the flake)..."
  chown -R "$USER_NAME":users "$NIXOS_DIR" 2>/dev/null || true
  nix-shell -p git --run "
    git -C '$NIXOS_DIR' add -A
    git -C '$NIXOS_DIR' add -f hardware-configuration.nix
  " 2>/dev/null || true

  say "building #$HOST (first build takes a while)..."
  nixos-rebuild switch --flake "$NIXOS_DIR#$HOST" || { err "build failed"; return 1; }
  ok "built and switched"

  # users created by nix have no password — set one or you can't log in
  echo
  say "set a password for '$USER_NAME'"
  until passwd "$USER_NAME"; do warn "try again"; done

  say "creating xdg user directories..."
  sudo -u "$USER_NAME" xdg-user-dirs-update 2>/dev/null || true

  notes
}

# ─────────────────────────────────────────────────────────────────────
# repair
# ─────────────────────────────────────────────────────────────────────
repair() {
  cat <<MENU

${B}repair${X}
  1) set the '$USER_NAME' password      ${Y}(can't log in)${X}
  2) reset home-manager profile       ${Y}(theming/config not applying)${X}
  3) fix ownership + stage files       ${Y}("not tracked by Git" errors)${X}
  4) roll back to previous generation  ${Y}(rebuild broke the system)${X}
  b) back
MENU
  read -rp "> " c
  case "$c" in
    1)
      id "$USER_NAME" >/dev/null 2>&1 || { err "'$USER_NAME' doesn't exist yet"; return; }
      passwd "$USER_NAME"
      ;;
    2)
      echo
      echo " symptoms: gtk/qt theming not applying · waybar, mako or kitty on"
      echo " defaults · wrong mime handlers · autostarts not running ·"
      echo " 'syntax error, unexpected end of file' from env-manifest.nix ·"
      echo " home-manager-$USER_NAME.service failed"
      echo
      echo " cause: the user profile manifest gets corrupted, so home-manager"
      echo " can't install over it and activation dies part-way — silently."
      echo " nothing of yours lives there; it's rebuilt from your config."
      echo
      local h; h=$(getent passwd "$USER_NAME" | cut -d: -f6)
      if [ -z "$h" ] || [ "$h" = "/" ] || [ ! -d "$h" ]; then
        err "no home directory for '$USER_NAME' — refusing to delete anything"
        return
      fi
      read -rp " reset it? [y/N] " a
      [[ "$a" =~ ^[Yy]$ ]] || return
      rm -f  "$h/.nix-profile"
      rm -rf "$h/.local/state/nix/profiles/profile"*
      rm -rf "$h/.local/state/home-manager"
      ok "cleared — rebuild to recreate it (nxrebuild)"
      ;;
    3)
      chown -R "$USER_NAME":users "$NIXOS_DIR" 2>/dev/null || true
      nix-shell -p git --run "
        git -C '$NIXOS_DIR' add -A
        git -C '$NIXOS_DIR' add -f hardware-configuration.nix
      " 2>/dev/null || true
      ok "ownership set to $USER_NAME, all files staged"
      ;;
    4)
      nixos-rebuild list-generations 2>/dev/null | head -6 | sed 's/^/   /'
      read -rp " roll back to the previous generation? [y/N] " a
      [[ "$a" =~ ^[Yy]$ ]] && nixos-rebuild switch --rollback && ok "rolled back"
      ;;
    b|B) return ;;
    *) echo "?" ;;
  esac
}

# ─────────────────────────────────────────────────────────────────────
notes() {
  cat <<NOTE

${B}────────────────────────────────────────────${X}
${B} not handled by the config${X}
${B}────────────────────────────────────────────${X}

 ${C}weather widget${X}   needs an openweathermap api key
     mkdir -p ~/.config/weather
     echo "YOUR_KEY" > ~/.config/weather/key
     chmod 600 ~/.config/weather/key
   (new keys take up to 2h to activate)

 ${C}sober / roblox${X}
     flatpak install flathub org.vinegarhq.Sober

 ${C}check your displays${X}   modes are hardcoded per host
     swaymsg -t get_outputs
   fix sway/monitors-<host>.nix if they don't match

 log out and back in. from now on: ${B}nxrebuild${X}

NOTE
}

# ─────────────────────────────────────────────────────────────────────
while true; do
  cat <<MENU

${B}nixos-config${X}   $(hostname)

  1) install       ${Y}(fresh machine)${X}
  2) repair
  3) post-install notes
  q) quit
MENU
  read -rp "> " c
  case "$c" in
    1) install ;;
    2) repair ;;
    3) notes ;;
    q|Q) exit 0 ;;
    *) echo "?" ;;
  esac
  echo; read -rp "press enter..."
done
