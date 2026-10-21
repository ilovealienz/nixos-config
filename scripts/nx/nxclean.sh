# nxclean: remove old generations now, keeping the last 3
if [ "$EUID" -eq 0 ]; then
  echo "Do not run nxclean as root/sudo"
  exit 1
fi
nh clean all -k 3
