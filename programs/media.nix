{ pkgs, ... }:
{
  environment.systemPackages = with pkgs; [
    mpv
    qbittorrent
    gimp
    ffmpeg
    obs-studio
    yt-dlp
    spotify
  ];
}
