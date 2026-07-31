{ pkgs, ... }:
{
  environment.systemPackages = with pkgs; [
    mpv
    qbittorrent
    gimp
    ffmpeg
    (obs-studio.override { browserSupport = false; })
    yt-dlp
    spotify
  ];
}
