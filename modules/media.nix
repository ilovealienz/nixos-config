{ pkgs, ... }:

{
  environment.systemPackages = with pkgs; [
    mpv
    ffmpeg
    obs-studio
    yt-dlp
    spotify
  ];
}
