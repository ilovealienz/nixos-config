# nxport <port> [close]: open or close a firewall port (tcp and udp) until
# the next rebuild or reboot
[ -n "${1:-}" ] || { echo "usage: nxport <port> [close]" >&2; exit 1; }
port=$1

rule() { sudo iptables "$1" nixos-fw -p "$2" --dport "$port" -j ACCEPT; }

if [ "${2:-}" = close ]; then
  for proto in tcp udp; do
    while rule -C "$proto" 2>/dev/null; do rule -D "$proto"; done
  done
  echo "closed $port"
else
  for proto in tcp udp; do
    rule -C "$proto" 2>/dev/null || rule -I "$proto"
  done
  echo "opened $port (tcp+udp) — until next rebuild/reboot"
fi
