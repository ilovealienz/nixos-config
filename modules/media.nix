{ pkgs, pkgs-unstable, ... }: {
  environment.systemPackages = with pkgs; [
    mpv
    ffmpeg
    obs-studio
    yt-dlp
    spotify
    streamlink
    streamlink-twitch-gui-bin
  ] ++ (with pkgs-unstable; [
    yacreader
  ]);
}
