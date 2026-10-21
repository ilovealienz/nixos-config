# nxcheck: evaluate every host the way a rebuild would, without building
flake=/etc/nixos

untracked=$(git -C "$flake" ls-files --others --exclude-standard || true)
if [ -n "$untracked" ]; then
  echo "nxcheck: not seen until staged (git add):"
  printf '%s\n' "$untracked" | sed 's/^/  /'
fi

hosts=()
read -ra hosts < <(nix eval --raw "$flake#nixosConfigurations" \
  --apply 'c: builtins.concatStringsSep " " (builtins.attrNames c)' 2>/dev/null; echo) || true
if [ "${#hosts[@]}" -eq 0 ]; then
  echo "nxcheck: the flake itself doesn't evaluate:"
  nix eval "$flake#nixosConfigurations" --apply builtins.attrNames 2>&1 | tail -20 || true
  exit 1
fi

# all hosts in parallel
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
for h in "${hosts[@]}"; do
  (
    rc=0
    nix eval --raw "$flake#nixosConfigurations.$h.config.system.build.toplevel.drvPath" \
      >/dev/null 2>"$tmp/$h.err" || rc=$?
    echo "$rc" >"$tmp/$h.rc"
  ) &
done
wait

fail=0
for h in "${hosts[@]}"; do
  if [ "$(cat "$tmp/$h.rc")" = 0 ]; then
    echo "✓ $h"
  else
    echo "✗ $h"
    { grep -v "^warning: Git tree" "$tmp/$h.err" || true; } | tail -20 | sed 's/^/  /'
    fail=1
  fi
done
exit "$fail"
