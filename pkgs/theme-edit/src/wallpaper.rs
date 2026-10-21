//! The wallpaper picker: thumbnails in the background, and the pick copied
//! to ~/.config/theme/wallpaper (non-PNG/JPEG converted to PNG).

use eframe::egui;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

/// file endings shown in the picker
const ENDINGS: [&str; 4] = ["png", "jpg", "jpeg", "webp"];

/// thumbnails are made this wide or high at most, in pixels
const THUMB: u32 = 320;

/// Folders to look in, labelled: walls/, Pictures/Wallpaper(s), then added
/// ones. Existing ones only, each once.
pub fn folders(theme_nix: &Path, added: &[String]) -> Vec<(String, PathBuf)> {
    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    let mut out = Vec::new();
    if let Some(dir) = theme_nix.parent() {
        out.push(("the config's walls/".to_string(), dir.join("walls")));
    }
    let pictures = pictures_dir(&home);
    for name in ["Wallpaper", "Wallpapers"] {
        out.push((
            format!("{}/{name}", pictures.display()),
            pictures.join(name),
        ));
    }
    for a in added {
        out.push((a.clone(), PathBuf::from(a)));
    }
    let mut seen: Vec<PathBuf> = Vec::new();
    out.retain(|(_, p)| {
        let real = p.canonicalize().unwrap_or_else(|_| p.clone());
        let keep = p.is_dir() && !seen.contains(&real);
        seen.push(real);
        keep
    });
    out
}

/// A folder typed in to add: ~ for home, and it must be a folder; the
/// path as it's saved, or why not.
pub fn check_folder(typed: &str) -> Result<String, String> {
    let typed = typed.trim().trim_end_matches('/');
    let home = std::env::var("HOME").unwrap_or_default();
    let path = match typed.strip_prefix('~') {
        Some(rest) if rest.is_empty() || rest.starts_with('/') => format!("{home}{rest}"),
        _ => typed.to_string(),
    };
    if !path.starts_with('/') {
        return Err("give the whole path, starting with / or ~".into());
    }
    if path.contains(['"', '\\']) || path.chars().any(char::is_control) {
        return Err("that path has a \" or \\ in it".into());
    }
    if !Path::new(&path).is_dir() {
        return Err(format!("{path} isn't a folder"));
    }
    Ok(path)
}

/// XDG_PICTURES_DIR from ~/.config/user-dirs.dirs, or ~/Pictures
fn pictures_dir(home: &Path) -> PathBuf {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));
    let text = std::fs::read_to_string(config.join("user-dirs.dirs")).unwrap_or_default();
    parse_pictures(&text, home).unwrap_or_else(|| home.join("Pictures"))
}

/// `XDG_PICTURES_DIR="$HOME/Pictures"` (or an absolute path)
fn parse_pictures(text: &str, home: &Path) -> Option<PathBuf> {
    let line = text
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("XDG_PICTURES_DIR="))?;
    let v = line["XDG_PICTURES_DIR=".len()..].trim().trim_matches('"');
    if let Some(rest) = v.strip_prefix("$HOME") {
        return Some(home.join(rest.trim_start_matches('/')));
    }
    v.starts_with('/').then(|| PathBuf::from(v))
}

/// subfolders are looked in this deep at most, and this many folders in
/// all, so adding a big folder (or / by mistake) doesn't walk the disk
const DEPTH: usize = 4;
const MOST: usize = 300;

/// Groups to show: each folder, then its subfolders with images.
pub fn groups(folders: &[(String, PathBuf)]) -> Vec<(String, Vec<PathBuf>)> {
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut out = Vec::new();
    for (label, dir) in folders {
        walk(label, dir, dir, 0, &mut seen, &mut out);
    }
    out
}

fn walk(
    label: &str,
    root: &Path,
    dir: &Path,
    depth: usize,
    seen: &mut Vec<PathBuf>,
    out: &mut Vec<(String, Vec<PathBuf>)>,
) {
    // a folder linked from inside itself, or reached twice, once only
    let real = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    if seen.contains(&real) || seen.len() >= MOST {
        return;
    }
    seen.push(real);
    let found = images(dir);
    // an empty folder you added stays as a heading (it says "no images"),
    // unless its subfolders have images: then it's left out, below
    let placeholder = depth == 0 && found.is_empty();
    let at = out.len();
    if depth == 0 || !found.is_empty() {
        let name = match dir.strip_prefix(root) {
            Ok(rel) if !rel.as_os_str().is_empty() => format!("{label} / {}", rel.display()),
            _ => label.to_string(),
        };
        out.push((name, found));
    }
    if depth >= DEPTH {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut subs: Vec<PathBuf> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.is_dir()
                && !p
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with('.'))
        })
        .collect();
    subs.sort_by_key(|p| p.file_name().map(|n| n.to_ascii_lowercase()));
    for s in subs {
        walk(label, root, &s, depth + 1, seen, out);
    }
    if placeholder && out.len() > at + 1 {
        out.remove(at);
    }
}

/// the images in a folder (not its subfolders), by name
pub fn images(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| ENDINGS.contains(&e.to_ascii_lowercase().as_str()))
        })
        .collect();
    out.sort_by_key(|p| p.file_name().map(|n| n.to_ascii_lowercase()));
    out
}

/// Makes thumbnails one after another on a thread of its own, so a big
/// image doesn't hold up the window.
pub struct Thumbs {
    send: mpsc::Sender<PathBuf>,
    recv: mpsc::Receiver<(PathBuf, Result<egui::ColorImage, String>)>,
}

impl Thumbs {
    pub fn new(ctx: &egui::Context) -> Self {
        let (send, jobs) = mpsc::channel::<PathBuf>();
        let (done, recv) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            for path in jobs {
                // a broken file that makes a decoder panic shouldn't stop
                // the thumbnails after it
                let r = std::panic::catch_unwind(|| thumbnail(&path))
                    .unwrap_or_else(|_| Err("the decoder failed".into()));
                if done.send((path, r)).is_err() {
                    break;
                }
                ctx.request_repaint();
            }
        });
        Thumbs { send, recv }
    }

    pub fn ask(&self, path: PathBuf) {
        let _ = self.send.send(path);
    }

    /// the thumbnails made since the last call
    pub fn ready(&self) -> Vec<(PathBuf, Result<egui::ColorImage, String>)> {
        self.recv.try_iter().collect()
    }
}

fn thumbnail(path: &Path) -> Result<egui::ColorImage, String> {
    let img = image::ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?
        .decode()
        .map_err(|e| e.to_string())?
        .thumbnail(THUMB, THUMB)
        .to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    Ok(egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()))
}

/// ~/.config/theme/wallpaper: where the picked image is copied to, and
/// where sway loads it from
pub fn copy_path(colors_json: &Path) -> PathBuf {
    colors_json.with_file_name("wallpaper")
}

/// Atomically copies `from` to `to`, converting non-PNG/JPEG to PNG.
pub fn install(from: &Path, to: &Path) -> Result<(), String> {
    let dir = to.parent().ok_or("no folder")?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let tmp = dir.join(".wallpaper.tmp");
    let bytes = std::fs::read(from).map_err(|e| format!("can't read {}: {e}", from.display()))?;
    let res = if is_png(&bytes) || is_jpeg(&bytes) {
        std::fs::write(&tmp, &bytes).map_err(|e| e.to_string())
    } else {
        image::load_from_memory(&bytes)
            .map_err(|e| format!("can't read {}: {e}", from.display()))
            .and_then(|img| {
                img.save_with_format(&tmp, image::ImageFormat::Png)
                    .map_err(|e| e.to_string())
            })
    };
    if let Err(e) = res {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    std::fs::rename(&tmp, to).map_err(|e| e.to_string())
}

fn is_png(b: &[u8]) -> bool {
    b.starts_with(b"\x89PNG\r\n\x1a\n")
}

fn is_jpeg(b: &[u8]) -> bool {
    b.starts_with(&[0xff, 0xd8, 0xff])
}

/// the swaymsg command that shows `file` now, `mode` and colour `bg`
/// (rrggbb) around it
pub fn sway_command(file: &Path, mode: &str, bg: &str) -> String {
    let f = file
        .display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    format!("output * bg \"{f}\" {mode} #{bg}")
}

/// shows it in sway now; an error if sway isn't running or refused
pub fn show(file: &Path, mode: &str, bg: &str) -> Result<(), String> {
    let out = std::process::Command::new("swaymsg")
        .arg(sway_command(file, mode, bg))
        .output()
        .map_err(|e| format!("can't run swaymsg: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        let msg = String::from_utf8_lossy(&out.stdout).to_string()
            + &String::from_utf8_lossy(&out.stderr);
        Err(format!("sway didn't take it: {}", msg.trim()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pictures_from_user_dirs() {
        let home = Path::new("/home/me");
        let t = "# comment\nXDG_DESKTOP_DIR=\"$HOME/Desktop\"\nXDG_PICTURES_DIR=\"/mnt/Shared/User/Pictures\"\n";
        assert_eq!(
            parse_pictures(t, home),
            Some(PathBuf::from("/mnt/Shared/User/Pictures"))
        );
        assert_eq!(
            parse_pictures("XDG_PICTURES_DIR=\"$HOME/Pics\"", home),
            Some(PathBuf::from("/home/me/Pics"))
        );
        assert_eq!(
            parse_pictures("XDG_PICTURES_DIR=\"$HOME\"", home),
            Some(PathBuf::from("/home/me/"))
        );
        assert_eq!(parse_pictures("XDG_MUSIC_DIR=\"$HOME/Music\"", home), None);
        assert_eq!(parse_pictures("XDG_PICTURES_DIR=\"relative\"", home), None);
    }

    #[test]
    fn copies_png_and_jpeg_converts_webp() {
        let dir = std::env::temp_dir().join(format!("theme-edit-wp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let img =
            image::RgbImage::from_fn(8, 6, |x, y| image::Rgb([x as u8 * 30, y as u8 * 40, 7]));
        for (name, fmt) in [
            ("a.png", image::ImageFormat::Png),
            ("b.jpg", image::ImageFormat::Jpeg),
            ("c.webp", image::ImageFormat::WebP),
        ] {
            img.save_with_format(dir.join(name), fmt).unwrap();
        }
        std::fs::write(dir.join("notes.txt"), "x").unwrap();
        std::fs::create_dir(dir.join("sub.png")).unwrap();
        let found: Vec<_> = images(&dir)
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(found, ["a.png", "b.jpg", "c.webp"]);

        let to = dir.join("theme/wallpaper");
        for name in ["a.png", "b.jpg"] {
            install(&dir.join(name), &to).unwrap();
            assert_eq!(
                std::fs::read(&to).unwrap(),
                std::fs::read(dir.join(name)).unwrap()
            );
        }
        install(&dir.join("c.webp"), &to).unwrap();
        let b = std::fs::read(&to).unwrap();
        assert!(is_png(&b));
        assert_eq!(
            image::load_from_memory(&b).unwrap().to_rgb8(),
            img,
            "webp is lossless here"
        );
        assert!(!dir.join("theme/.wallpaper.tmp").exists());
        // a broken image leaves the copy as it was
        std::fs::write(dir.join("bad.webp"), "not an image").unwrap();
        assert!(install(&dir.join("bad.webp"), &to).is_err());
        assert_eq!(std::fs::read(&to).unwrap(), b);
        assert!(!dir.join("theme/.wallpaper.tmp").exists());
        assert!(thumbnail(&dir.join("a.png")).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn added_folders() {
        let dir = std::env::temp_dir().join(format!("theme-edit-fold-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("walls")).unwrap();
        std::fs::create_dir_all(dir.join("mine")).unwrap();
        let mine = dir.join("mine").display().to_string();
        let walls = dir.join("walls").display().to_string();
        let got = folders(
            &dir.join("theme.nix"),
            &[
                mine.clone(),
                walls.clone(),
                format!("{mine}/../mine"),
                "/nope".into(),
            ],
        );
        let paths: Vec<_> = got.iter().map(|(_, p)| p.display().to_string()).collect();
        // walls/ once (it's built in), mine once, the missing one left out
        assert_eq!(paths, [walls.clone(), mine.clone()]);
        assert_eq!(check_folder(&format!("{mine}/")), Ok(mine.clone()));
        assert!(check_folder("relative/path").is_err());
        assert!(check_folder("/nope/nothing").is_err());
        assert!(check_folder(&format!("{walls}/x.png")).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn subfolders_in_groups() {
        let dir = std::env::temp_dir().join(format!("theme-edit-sub-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for d in [
            "top/b-sub",
            "top/a-sub/deeper",
            "top/empty",
            "top/.hidden",
            "other",
        ] {
            std::fs::create_dir_all(dir.join(d)).unwrap();
        }
        for f in [
            "top/x.png",
            "top/a-sub/1.jpg",
            "top/a-sub/deeper/2.png",
            "top/b-sub/3.webp",
            "top/.hidden/4.png",
        ] {
            std::fs::write(dir.join(f), "").unwrap();
        }
        // a link back up: must not loop
        std::os::unix::fs::symlink(dir.join("top"), dir.join("top/b-sub/loop")).unwrap();
        let g = groups(&[
            ("top".into(), dir.join("top")),
            ("other".into(), dir.join("other")),
            // already shown inside top
            ("again".into(), dir.join("top/a-sub")),
        ]);
        let got: Vec<(String, usize)> = g.iter().map(|(l, f)| (l.clone(), f.len())).collect();
        assert_eq!(
            got,
            [
                ("top".to_string(), 1),
                ("top / a-sub".to_string(), 1),
                ("top / a-sub/deeper".to_string(), 1),
                ("top / b-sub".to_string(), 1),
                ("other".to_string(), 0),
            ]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn folder_with_only_subfolders() {
        let dir = std::env::temp_dir().join(format!("theme-edit-only-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("a/s")).unwrap();
        std::fs::create_dir_all(dir.join("b")).unwrap();
        std::fs::write(dir.join("a/s/1.png"), "").unwrap();
        let g = groups(&[("a".into(), dir.join("a")), ("b".into(), dir.join("b"))]);
        let got: Vec<(String, usize)> = g.iter().map(|(l, f)| (l.clone(), f.len())).collect();
        // a's own (empty) heading left out; b, empty all through, kept
        assert_eq!(got, [("a / s".to_string(), 1), ("b".to_string(), 0)]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sway_commands() {
        assert_eq!(
            sway_command(
                Path::new("/home/pc/.config/theme/wallpaper"),
                "fill",
                "2b201a"
            ),
            "output * bg \"/home/pc/.config/theme/wallpaper\" fill #2b201a"
        );
        assert_eq!(
            sway_command(Path::new("/a \"b\"/w"), "fit", "000000"),
            "output * bg \"/a \\\"b\\\"/w\" fit #000000"
        );
    }
}
