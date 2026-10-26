# nxdebug slow|fast: boot and session state, saved to ~/nxdebug-slow.txt or
# ~/nxdebug-fast.txt. nxdebug diff compares them. Temporary.

case "${1:-}" in
  slow|fast) ;;
  diff)
    diff -u ~/nxdebug-fast.txt ~/nxdebug-slow.txt | less -R
    exit 0
    ;;
  *)
    echo "usage: nxdebug slow|fast|diff" >&2
    exit 2
    ;;
esac

out=~/nxdebug-$1.txt
{
  echo "== kernel"; uname -r; cat /etc/specialisation 2>/dev/null || echo "default entry"
  echo "== boot"; systemd-analyze; systemd-analyze blame | head -15; systemd-analyze critical-chain | head -20
  echo "== failed"; systemctl --failed --no-legend; systemctl --user --failed --no-legend
  echo "== spotify"; journalctl --user -b -u spotify-delayed --no-pager | tail -20
  echo "== warnings"; journalctl -b -p 4 --no-pager | tail -60
  echo "== load"; uptime; cat /proc/pressure/cpu /proc/pressure/io /proc/pressure/memory
  echo "== top"; ps -eo pid,pcpu,pmem,etime,comm --sort=-pcpu | head -12
  echo "== gpu"
  for d in /sys/class/drm/card*/device; do
    [ -f "$d/pp_dpm_sclk" ] || continue
    echo "$d"; cat "$d/power_dpm_force_performance_level" "$d/pp_dpm_sclk"
  done
} > "$out" 2>&1
echo "saved to $out"
