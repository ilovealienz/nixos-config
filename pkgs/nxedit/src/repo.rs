//! The repo's files with their git state, file type and owner.

use std::collections::HashMap;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Git {
    Clean,
    /// changed and not (all) staged
    Changed,
    /// staged, nothing more since
    Staged,
    /// not in git: the flake can't see it
    New,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Nix,
    Shell,
    Rust,
    Go,
    Python,
    Text,
    Data,
    Image,
    Audio,
    Secret,
    Other,
}

#[derive(Clone, Debug)]
pub struct File {
    /// relative to the repo, with /
    pub path: String,
    pub git: Git,
    pub kind: Kind,
    /// someone else's (root's): saving needs it handed back first
    pub foreign: bool,
}

#[derive(Default, Debug)]
pub struct Branch {
    pub name: String,
    pub ahead: u32,
    pub behind: u32,
}

pub struct Repo {
    pub root: PathBuf,
    pub files: Vec<File>,
    pub branch: Branch,
}

impl Repo {
    pub fn load(root: &Path) -> Repo {
        let uid = own_ids().map(|(u, _)| u);
        let (states, branch) = status(root);
        let mut paths = git_files(root).unwrap_or_else(|| walk(root));
        paths.retain(|p| root.join(p).is_file());
        paths.sort();
        paths.dedup();
        let files = paths
            .into_iter()
            .map(|path| File {
                git: states.get(&path).copied().unwrap_or(Git::Clean),
                kind: kind(&path),
                foreign: match (uid, std::fs::symlink_metadata(root.join(&path))) {
                    (Some(u), Ok(m)) => u != 0 && m.uid() != u,
                    _ => false,
                },
                path,
            })
            .collect();
        Repo {
            root: root.to_path_buf(),
            files,
            branch,
        }
    }

    pub fn counts(&self) -> (usize, usize, usize) {
        let n = |g| self.files.iter().filter(|f| f.git == g).count();
        (n(Git::Changed), n(Git::Staged), n(Git::New))
    }
}

fn git(root: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("--no-optional-locks")
        .args(args)
        .output()
        .ok()?;
    out.status.success().then_some(out.stdout)
}

/// tracked and untracked files (not ignored ones, but force-added ones)
fn git_files(root: &Path) -> Option<Vec<String>> {
    let out = git(
        root,
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ],
    )?;
    Some(split_z(&out).map(str::to_string).collect())
}

/// without git: every file, .git and build output left out
fn walk(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut todo = vec![PathBuf::new()];
    while let Some(dir) = todo.pop() {
        let Ok(rd) = std::fs::read_dir(root.join(&dir)) else {
            continue;
        };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name == ".git" || name == "target" || name == "result" {
                continue;
            }
            let rel = dir.join(&name);
            match e.file_type() {
                Ok(t) if t.is_dir() => todo.push(rel),
                Ok(_) => out.push(rel.to_string_lossy().into_owned()),
                Err(_) => {}
            }
        }
    }
    out
}

fn split_z(b: &[u8]) -> impl Iterator<Item = &str> {
    b.split(|&c| c == 0)
        .filter(|s| !s.is_empty())
        .filter_map(|s| std::str::from_utf8(s).ok())
}

fn status(root: &Path) -> (HashMap<String, Git>, Branch) {
    match git(
        root,
        &[
            "status",
            "--porcelain=v1",
            "-z",
            "--branch",
            "--untracked-files=all",
        ],
    ) {
        Some(out) => parse_status(&out),
        None => (HashMap::new(), Branch::default()),
    }
}

pub fn parse_status(out: &[u8]) -> (HashMap<String, Git>, Branch) {
    let mut states = HashMap::new();
    let mut branch = Branch::default();
    let mut entries = split_z(out);
    while let Some(e) = entries.next() {
        if let Some(b) = e.strip_prefix("## ") {
            branch = parse_branch(b);
            continue;
        }
        if e.len() < 4 {
            continue;
        }
        let (x, y, path) = (e.as_bytes()[0], e.as_bytes()[1], &e[3..]);
        // a rename or copy: the old name follows
        if x == b'R' || x == b'C' {
            entries.next();
        }
        let g = match (x, y) {
            (b'?', b'?') => Git::New,
            (b'!', b'!') => continue,
            (_, b' ') => Git::Staged,
            _ => Git::Changed,
        };
        states.insert(path.to_string(), g);
    }
    (states, branch)
}

/// "main...origin/main [ahead 2, behind 1]"
fn parse_branch(b: &str) -> Branch {
    let b = b.strip_prefix("No commits yet on ").unwrap_or(b);
    let (head, rest) = b.split_once(' ').unwrap_or((b, ""));
    let name = head.split("...").next().unwrap_or(head).to_string();
    let num = |key: &str| {
        rest.split(['[', ']', ','])
            .map(str::trim)
            .find_map(|p| p.strip_prefix(key))
            .and_then(|n| n.trim().parse().ok())
            .unwrap_or(0)
    };
    Branch {
        name,
        ahead: num("ahead "),
        behind: num("behind "),
    }
}

pub fn kind(path: &str) -> Kind {
    let name = path.rsplit('/').next().unwrap_or(path);
    let ext = name
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "nix" => Kind::Nix,
        "sh" | "bash" | "zsh" => Kind::Shell,
        "rs" => Kind::Rust,
        "go" => Kind::Go,
        "py" => Kind::Python,
        "md" | "txt" => Kind::Text,
        "json" | "toml" | "lock" | "mod" | "sum" | "yaml" | "yml" | "ini" | "conf" => Kind::Data,
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "svg" => Kind::Image,
        "mp3" | "flac" | "ogg" | "wav" | "opus" => Kind::Audio,
        "age" | "pub" | "key" => Kind::Secret,
        // scripts without an extension (scripts/urlopen)
        "" if path.starts_with("scripts/") => Kind::Shell,
        _ if name == ".gitignore" => Kind::Data,
        _ => Kind::Other,
    }
}

/// this process's uid and gid, from /proc
pub fn own_ids() -> Option<(u32, u32)> {
    let s = std::fs::read_to_string("/proc/self/status").ok()?;
    let field = |k: &str| {
        s.lines()
            .find_map(|l| l.strip_prefix(k))
            .and_then(|v| v.split_whitespace().next())
            .and_then(|v| v.parse().ok())
    };
    Some((field("Uid:")?, field("Gid:")?))
}

/// a NUL in the first 8 KiB
pub fn is_binary(path: &Path) -> bool {
    use std::io::Read;
    let mut buf = [0u8; 8192];
    match std::fs::File::open(path).and_then(|mut f| f.read(&mut buf)) {
        Ok(n) => buf[..n].contains(&0),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_and_branch() {
        let out = b"## main...origin/main [ahead 2, behind 1]\0 M desktop/sway.nix\0A  home/nx.nix\0MM theme.nix\0?? scripts/new.sh\0R  new.nix\0old.nix\0!! junk\0";
        let (s, b) = parse_status(out);
        assert_eq!(b.name, "main");
        assert_eq!((b.ahead, b.behind), (2, 1));
        assert_eq!(s["desktop/sway.nix"], Git::Changed);
        assert_eq!(s["home/nx.nix"], Git::Staged);
        assert_eq!(s["theme.nix"], Git::Changed);
        assert_eq!(s["scripts/new.sh"], Git::New);
        assert_eq!(s["new.nix"], Git::Staged);
        assert!(!s.contains_key("old.nix"));
        assert!(!s.contains_key("junk"));
    }

    #[test]
    fn branch_without_upstream() {
        let (_, b) = parse_status(b"## main\0");
        assert_eq!((b.name.as_str(), b.ahead), ("main", 0));
        let (_, b) = parse_status(b"## No commits yet on main\0");
        assert_eq!(b.name, "main");
    }

    #[test]
    fn kinds() {
        assert_eq!(kind("flake.nix"), Kind::Nix);
        assert_eq!(kind("scripts/urlopen"), Kind::Shell);
        assert_eq!(kind("pkgs/x/Cargo.lock"), Kind::Data);
        assert_eq!(kind("walls/1.png"), Kind::Image);
        assert_eq!(kind("secrets.age"), Kind::Secret);
        assert_eq!(kind("README.md"), Kind::Text);
        assert_eq!(kind("bootstrap.sh"), Kind::Shell);
        assert_eq!(kind("LICENSE"), Kind::Other);
    }

    #[test]
    fn ids() {
        assert!(own_ids().is_some());
    }
}
