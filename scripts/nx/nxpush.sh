# nxpush [--no-check] [message]: check every host, commit and push
# (never hardware-configuration.nix or local-theme.json)
g=(sudo -E git -C /etc/nixos)
skip=(-- . ':(exclude)hardware-configuration.nix' ':(exclude)local-theme.json')
check=1
if [ "${1:-}" = --no-check ]; then check=0; shift; fi

# changed secrets into secrets.age; stops if the repo's is newer
nxsecrets pack --if-changed || { echo "nxpush: not pushed"; exit 1; }
"${g[@]}" add -A
if "${g[@]}" diff --cached --quiet "${skip[@]}" &&
   [ "$("${g[@]}" rev-list --count '@{u}..HEAD' 2>/dev/null)" = 0 ]; then
  echo "nxpush: nothing to push"
  exit 0
fi
if [ "$check" = 1 ] && ! nxcheck; then
  echo "nxpush: not pushed (nxpush --no-check to push anyway)"
  exit 1
fi
if ! "${g[@]}" diff --cached --quiet "${skip[@]}"; then
  "${g[@]}" commit -q -m "${1:-update}" "${skip[@]}"
fi
GIT_SSH_COMMAND='ssh -i /home/pc/.ssh/id_ed25519' "${g[@]}" push
