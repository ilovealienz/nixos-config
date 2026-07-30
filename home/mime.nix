{ lib, config, ... }:
let
  video   = "mpv.desktop";
  audio   = "mpv.desktop";
  image   = "feh.desktop";
  browser = "floorp.desktop";
  editor  = "codium.desktop";
  files   = "thunar.desktop";
  archive = "xarchiver.desktop";

  defaults = {
    "video/mp4" = video;
    "video/x-matroska" = video;
    "video/webm" = video;
    "video/quicktime" = video;
    "video/x-msvideo" = video;
    "video/mpeg" = video;
    "video/x-flv" = video;
    "video/3gpp" = video;
    "video/x-ms-wmv" = video;
    "video/ogg" = video;
    "video/x-m4v" = video;

    "audio/mpeg" = audio;
    "audio/flac" = audio;
    "audio/x-wav" = audio;
    "audio/ogg" = audio;
    "audio/mp4" = audio;
    "audio/aac" = audio;
    "audio/x-m4a" = audio;
    "audio/opus" = audio;

    "image/png" = image;
    "image/jpeg" = image;
    "image/gif" = image;
    "image/webp" = image;
    "image/bmp" = image;
    "image/tiff" = image;
    "image/svg+xml" = image;

    "application/pdf" = browser;

    "text/plain" = editor;
    "text/markdown" = editor;
    "text/x-python" = editor;
    "text/x-shellscript" = editor;
    "text/x-nix" = editor;
    "application/json" = editor;
    "application/xml" = editor;
    "text/html" = browser;

    "application/zip" = archive;
    "application/x-tar" = archive;
    "application/x-compressed-tar" = archive;
    "application/x-7z-compressed" = archive;
    "application/x-rar" = archive;
    "application/vnd.rar" = archive;
    "application/gzip" = archive;
    "application/x-xz" = archive;

    "inode/directory" = files;

    "x-scheme-handler/http" = browser;
    "x-scheme-handler/https" = browser;
  };

  toLines = lib.concatStringsSep "\n"
    (lib.mapAttrsToList (mime: app: "${mime}=${app}") defaults);
in
{
  home.activation.mimeDefaults = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
    file="$HOME/.config/mimeapps.list"
    mkdir -p "$HOME/.config"
    {
      echo "[Default Applications]"
      cat <<'EOF'
${toLines}
EOF
    } > "$file"
  '';
}
