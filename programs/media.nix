{ pkgs, ... }:
{
  # Spotify itself is the com.spotify.Client flatpak
  environment.systemPackages = with pkgs; [
    mpv
    qbittorrent
    gimp
    ffmpeg
    obs-studio
    yt-dlp
    streamlink
    (callPackage ../pkgs/spotifast.nix { })
  ];
}
