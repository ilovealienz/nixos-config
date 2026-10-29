# nxrebuild: rebuild and switch this machine
nixos=/etc/nixos

# files written with sudo (git pull, patches) back to you, so they can be
# edited and saved
sudo find "$nixos" -path "$nixos/.git" -prune -o ! -user "$(id -un)" \
  -exec chown "$(id -un):$(id -gn)" {} +

# your colors.json as local-theme.json: staged so the flake sees it, never
# committed (no colors.json: theme.nix's defaults)
src=~/.config/theme/colors.json
dst=$nixos/local-theme.json
if [ -f "$src" ]; then
  cmp -s "$src" "$dst" || sudo install -m 644 "$src" "$dst"
  sudo git -C "$nixos" add -f local-theme.json
elif [ -e "$dst" ]; then
  sudo git -C "$nixos" rm -q --cached --ignore-unmatch local-theme.json
  sudo rm -f "$dst"
fi

# hardware-configuration.nix: likewise staged, never committed
sudo git -C "$nixos" add -f hardware-configuration.nix
# nh: a build tree while building, then what changed (packages added,
# removed, upgraded) before switching; it uses sudo itself. Activation
# output (units restarted or failed) shown too, as nixos-rebuild did
rc=0
NH_SHOW_ACTIVATION_LOGS=1 nh os switch "$nixos" -H "$(hostname)" || rc=$?

# home-manager failing only shows as a failed unit: say so, with its log
hm="home-manager-$(systemd-escape "$(id -un)").service"
if systemctl is-failed --quiet "$hm"; then
  echo >&2
  echo "nxrebuild: home-manager failed, your home config wasn't applied" >&2
  [ "$rc" -eq 0 ] || echo "nxrebuild: and this generation wasn't added to the boot menu" >&2
  inv=$(systemctl show -p InvocationID --value "$hm")
  if [ -n "$inv" ]; then
    sudo journalctl -q -o cat --no-pager "_SYSTEMD_INVOCATION_ID=$inv" | tail -n 15 >&2
  fi
  [ "$rc" -ne 0 ] || rc=1
fi
exit "$rc"
