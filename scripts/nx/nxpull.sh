# nxpull: pull from GitHub and rebuild; an unpushed local flake.lock is dropped
g=(sudo -E git -C /etc/nixos)
if ! "${g[@]}" diff --quiet -- flake.lock; then
  echo "nxpull: dropping this machine's unpushed flake.lock update"
  "${g[@]}" checkout -q -- flake.lock
fi
if ! GIT_SSH_COMMAND='ssh -i /home/pc/.ssh/id_ed25519' "${g[@]}" pull --ff-only; then
  echo "nxpull: not pulled; see git -C /etc/nixos status"
  exit 1
fi
rc=0
nxrebuild || rc=$?
# secrets changed on another machine
nxsecrets unpack --if-changed || true
exit "$rc"
