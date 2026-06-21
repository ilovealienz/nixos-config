{ pkgs, ... }:

{
  environment.systemPackages = with pkgs; [
    mpv
    ffmpeg
    obs-studio
    yt-dlp
    spotify
    streamlink
    streamlink-twitch-gui-bin
    yacreader
  ];
}
