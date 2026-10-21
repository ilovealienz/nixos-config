# nxaudio [alsamixer options]: alsamixer (card 0 by default), saved on exit
if [ $# -eq 0 ]; then set -- -c 0; fi
alsamixer "$@"
sudo alsactl store
echo "mixer saved"
