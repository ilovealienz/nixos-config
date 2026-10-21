//! What the list shows: the files as a tree (folders open or closed) or
//! as a flat list, narrowed and ordered by the filter.

use crate::fuzzy;
use crate::repo::File;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Row {
    /// a folder: its path, depth (0 at the top) and whether it's open
    Dir {
        path: String,
        depth: usize,
        open: bool,
    },
    /// a file (an index into the files), its depth, and the matched
    /// characters of its path
    File {
        idx: usize,
        depth: usize,
        hits: Vec<usize>,
    },
}

impl Row {
    /// what stays selected when the rows change
    pub fn key(&self, files: &[File]) -> String {
        match self {
            Row::Dir { path, .. } => format!("{path}/"),
            Row::File { idx, .. } => files[*idx].path.clone(),
        }
    }
}

#[derive(Default)]
struct Node {
    dirs: BTreeMap<String, Node>,
    files: Vec<(String, usize)>,
}

/// the matching files' indices with their score and hits; all of them,
/// unscored, without a filter
fn matches(files: &[File], filter: &str) -> HashMap<usize, (i32, Vec<usize>)> {
    files
        .iter()
        .enumerate()
        .filter_map(|(i, f)| {
            if filter.trim().is_empty() {
                Some((i, (0, Vec::new())))
            } else {
                fuzzy::score(filter, &f.path).map(|m| (i, m))
            }
        })
        .collect()
}

/// top-level files first, then folders; in each folder its files, then
/// its folders. A filter opens every folder with a match in it.
pub fn tree(files: &[File], filter: &str, closed: &HashSet<String>) -> Vec<Row> {
    let found = matches(files, filter);
    let filtering = !filter.trim().is_empty();
    let mut root = Node::default();
    for (i, f) in files.iter().enumerate() {
        if !found.contains_key(&i) {
            continue;
        }
        let mut node = &mut root;
        let mut parts: Vec<&str> = f.path.split('/').collect();
        let name = parts.pop().unwrap_or_default();
        for p in parts {
            node = node.dirs.entry(p.to_string()).or_default();
        }
        node.files.push((name.to_string(), i));
    }
    let mut rows = Vec::new();
    emit(&root, "", 0, closed, filtering, &found, &mut rows);
    rows
}

fn emit(
    node: &Node,
    prefix: &str,
    depth: usize,
    closed: &HashSet<String>,
    filtering: bool,
    found: &HashMap<usize, (i32, Vec<usize>)>,
    rows: &mut Vec<Row>,
) {
    let mut files = node.files.clone();
    files.sort_by_key(|(name, _)| order(name));
    for (_, idx) in files {
        rows.push(Row::File {
            idx,
            depth,
            hits: found[&idx].1.clone(),
        });
    }
    let mut dirs: Vec<_> = node.dirs.iter().collect();
    dirs.sort_by_key(|(name, _)| order(name));
    for (name, child) in dirs {
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        let open = filtering || !closed.contains(&path);
        rows.push(Row::Dir {
            path: path.clone(),
            depth,
            open,
        });
        if open {
            emit(child, &path, depth + 1, closed, filtering, found, rows);
        }
    }
}

/// names in order regardless of case or a leading dot
fn order(name: &str) -> String {
    name.trim_start_matches('.').to_lowercase()
}

/// every file by its full path: best match first while filtering
pub fn flat(files: &[File], filter: &str) -> Vec<Row> {
    let found = matches(files, filter);
    let mut idx: Vec<usize> = found.keys().copied().collect();
    let path = |i: &usize| files[*i].path.as_str();
    if filter.trim().is_empty() {
        // top-level files first, then by path
        idx.sort_by_key(|i| (path(i).contains('/'), order(path(i))));
    } else {
        // best first; ties: the shorter path
        idx.sort_by_key(|i| (std::cmp::Reverse(found[i].0), path(i).len(), order(path(i))));
    }
    idx.into_iter()
        .map(|i| Row::File {
            idx: i,
            depth: 0,
            hits: found[&i].1.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::{Git, kind};

    fn files(paths: &[&str]) -> Vec<File> {
        paths
            .iter()
            .map(|p| File {
                path: p.to_string(),
                git: Git::Clean,
                kind: kind(p),
                foreign: false,
            })
            .collect()
    }

    fn show(rows: &[Row], f: &[File]) -> Vec<String> {
        rows.iter()
            .map(|r| match r {
                Row::Dir { path, depth, open } => {
                    format!(
                        "{}{}{}",
                        "  ".repeat(*depth),
                        path,
                        if *open { "/" } else { "/+" }
                    )
                }
                Row::File { idx, depth, .. } => format!("{}{}", "  ".repeat(*depth), f[*idx].path),
            })
            .collect()
    }

    const PATHS: &[&str] = &[
        "desktop/sway.nix",
        "flake.nix",
        "desktop/monitors/pc.nix",
        "home/shell.nix",
        "configuration.nix",
        "desktop/awake.nix",
    ];

    #[test]
    fn tree_order() {
        let f = files(PATHS);
        let rows = tree(&f, "", &HashSet::new());
        assert_eq!(
            show(&rows, &f),
            [
                "configuration.nix",
                "flake.nix",
                "desktop/",
                "  desktop/awake.nix",
                "  desktop/sway.nix",
                "  desktop/monitors/",
                "    desktop/monitors/pc.nix",
                "home/",
                "  home/shell.nix",
            ]
        );
    }

    #[test]
    fn closed_folders_hide_their_files() {
        let f = files(PATHS);
        let closed = HashSet::from(["desktop".to_string()]);
        let rows = tree(&f, "", &closed);
        assert_eq!(
            show(&rows, &f),
            [
                "configuration.nix",
                "flake.nix",
                "desktop/+",
                "home/",
                "  home/shell.nix"
            ]
        );
        // a filter shows matches even in a closed folder
        let rows = tree(&f, "pc", &closed);
        assert_eq!(
            show(&rows, &f),
            [
                "desktop/",
                "  desktop/monitors/",
                "    desktop/monitors/pc.nix"
            ]
        );
    }

    #[test]
    fn flat_is_ranked_while_filtering() {
        let f = files(PATHS);
        let rows = flat(&f, "");
        assert_eq!(show(&rows, &f)[..2], ["configuration.nix", "flake.nix"]);
        let rows = flat(&f, "sway");
        assert_eq!(show(&rows, &f), ["desktop/sway.nix"]);
        let rows = flat(&f, "nix");
        assert_eq!(rows.len(), PATHS.len());
    }

    #[test]
    fn case_and_dots_dont_reorder() {
        let f = files(&["README.md", ".gitignore", "flake.nix", "bootstrap.sh"]);
        let rows = tree(&f, "", &HashSet::new());
        assert_eq!(
            show(&rows, &f),
            ["bootstrap.sh", "flake.nix", ".gitignore", "README.md"]
        );
    }

    #[test]
    fn keys() {
        let f = files(PATHS);
        let rows = tree(&f, "", &HashSet::new());
        assert_eq!(rows[2].key(&f), "desktop/");
        assert_eq!(rows[3].key(&f), "desktop/awake.nix");
    }
}
