# Right-click "Extract…" for archives in Thunar.
# A fuzzel menu picks where it goes, zenity shows progress. Never
# overwrites (name-2 instead), and a cancel or failure leaves nothing.
# Script: ./extract/extract.py, Thunar menu entry: ./extract/uca_merge.py
{ pkgs, lib, ... }:
let
  extract = pkgs.runCommand "extract" {
    nativeBuildInputs = [ pkgs.makeWrapper ];
    meta.mainProgram = "extract";
  } ''
    install -Dm644 ${./extract/extract.py} $out/libexec/extract.py
    makeWrapper ${pkgs.python3}/bin/python3 $out/bin/extract \
      --add-flags $out/libexec/extract.py \
      --prefix PATH : ${lib.makeBinPath (with pkgs; [
        fuzzel zenity libnotify wl-clipboard
        _7zz-rar   # 7z, rar, encrypted zips (unfree: includes the RAR decoder)
        libarchive # bsdtar: tar.zst, tar.lz, tar.lz4, cpio, deb, rpm
        zstd lz4 lzip gzip  # single .zst/.lz4/.lz/.Z files (lz4 also for tar.lz4)
      ])}
  '';

  # Thunar matches these case-sensitively, so list upper case too
  exts = [
    # common
    "zip" "7z" "rar" "tar" "tar.gz" "tgz" "tar.bz2" "tbz2" "tar.xz" "txz"
    "tar.zst" "tzst" "tar.lzma" "tlz" "tar.lz" "tar.lz4" "tar.Z" "taz"
    # single compressed files
    "gz" "bz2" "xz" "lzma" "zst" "lz4" "lz" "Z"
    # split archives: only the first part (any .rar part works too)
    "001"
    # comics, zip-based, packages
    "cbz" "cbr" "cb7" "zipx" "jar" "apk" "whl" "cpio" "deb" "rpm"
    # disk images and other formats 7-Zip opens
    "iso" "dmg" "vhd" "vhdx" "vmdk" "vdi" "qcow2" "squashfs" "wim" "esd"
    "msi" "cab" "chm" "xar" "pkg" "arj" "lzh" "lha"
  ];
  patterns = lib.concatStringsSep ";"
    (lib.concatMap (e: [ "*.${e}" "*.${lib.toUpper e}" ]) exts);

  # Thunar custom actions, kept in Thunar's own (editable) uca.xml.
  # On every rebuild these are added if missing; afterwards they're yours
  # to edit in Thunar's "Configure custom actions" window. Only the command
  # is kept pointing at the current Nix store path (or replaced if it's
  # still the old default). Your own actions are never touched.
  ucaActions = [
    {
      name = "Open Terminal Here";
      icon = "utilities-terminal";
      "unique-id" = "1789702767618787-1";
      command = "${lib.getExe pkgs.kitty} --directory %f";
      description = "Open kitty in this folder";
      patterns = "*";
      startup_notify = true;
      types = [ "directories" ];
      replace_if = [
        "^exo-open --working-directory %f --launch TerminalEmulator$"
        "^/nix/store/[^ ]+/bin/kitty --directory %f$"
      ];
    }
    {
      name = "Extract…";
      icon = "package-x-generic";
      "unique-id" = "1789702767618787-2";
      command = "${lib.getExe extract} %f";
      description = "Extract this archive";
      inherit patterns;
      types = [ "other-files" ];
      replace_if = [ "^/nix/store/[^ ]+/bin/extract %f$" ];
      own_patterns = "^\\*\\.zip;\\*\\.ZIP;";  # our list starts like this
    }
  ];
in
{
  home.packages = [ extract ];

  home.activation.thunarCustomActions = lib.hm.dag.entryAfter [ "writeBoundary" "linkGeneration" ] ''
    run ${pkgs.python3}/bin/python3 ${./extract/uca_merge.py} \
      ${pkgs.writeText "thunar-actions.json" (builtins.toJSON ucaActions)}
  '';
}
