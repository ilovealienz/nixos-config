//! Fuzzy matching like fzf's: every space-separated term must appear in
//! the path in order (case-insensitive). Runs of letters, the start of a
//! word and the file's own name score higher.

/// the score and the matched character positions, or None
pub fn score(query: &str, path: &str) -> Option<(i32, Vec<usize>)> {
    let hay: Vec<char> = path.chars().flat_map(char::to_lowercase).collect();
    // to_lowercase can change the length; positions then point at the
    // lowercased text, which is fine for ASCII paths
    let base = path.rfind('/').map_or(0, |i| path[..=i].chars().count());
    let mut total = 0;
    let mut positions = Vec::new();
    for term in query.split_whitespace() {
        let needle: Vec<char> = term.chars().flat_map(char::to_lowercase).collect();
        let (s, p) = best(&needle, &hay, base)?;
        total += s;
        positions.extend(p);
    }
    positions.sort_unstable();
    positions.dedup();
    Some((total, positions))
}

/// the best greedy match starting from each place the first char occurs
fn best(needle: &[char], hay: &[char], base: usize) -> Option<(i32, Vec<usize>)> {
    let first = *needle.first()?;
    let mut top: Option<(i32, Vec<usize>)> = None;
    for start in (0..hay.len()).filter(|&i| hay[i] == first) {
        let mut pos = vec![start];
        let mut j = start + 1;
        for &c in &needle[1..] {
            while j < hay.len() && hay[j] != c {
                j += 1;
            }
            if j == hay.len() {
                pos.clear();
                break;
            }
            pos.push(j);
            j += 1;
        }
        if pos.is_empty() {
            continue;
        }
        let s = rate(&pos, hay, base);
        if top.as_ref().is_none_or(|(t, _)| s > *t) {
            top = Some((s, pos));
        }
    }
    top
}

fn rate(pos: &[usize], hay: &[char], base: usize) -> i32 {
    let mut s = 0;
    for (k, &p) in pos.iter().enumerate() {
        s += 16;
        if k > 0 && pos[k - 1] + 1 == p {
            s += 12;
        }
        if p == 0 || matches!(hay[p - 1], '/' | '-' | '_' | '.' | ' ') {
            s += 10;
        }
        if p >= base {
            s += 6;
        }
    }
    // a tighter match beats a scattered one
    s - (pos[pos.len() - 1] - pos[0]) as i32 / 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_in_order() {
        assert!(score("swy", "desktop/sway.nix").is_some());
        assert!(score("ywa", "desktop/sway.nix").is_none());
        assert!(score("", "x").is_some());
        assert_eq!(
            score("SWAY", "desktop/sway.nix").unwrap().1,
            vec![8, 9, 10, 11]
        );
    }

    #[test]
    fn every_term_must_match() {
        assert!(score("desk sway", "desktop/sway.nix").is_some());
        assert!(score("desk wallpaper", "desktop/sway.nix").is_none());
    }

    #[test]
    fn file_names_and_runs_win() {
        let a = score("sway", "desktop/sway.nix").unwrap().0;
        let b = score("sway", "desktop/sway-helpers.nix").unwrap().0;
        let c = score("sway", "pkgs/sway-helpers/src/ipc.rs").unwrap().0;
        assert!(a >= b && b > c, "{a} {b} {c}");
        let run = score("nix", "theme.nix").unwrap().0;
        let scattered = score("nix", "pkgs/nxshortcuts/main.go");
        assert!(scattered.is_none_or(|(s, _)| run > s));
    }

    #[test]
    fn picks_the_best_start() {
        // the "theme" in the file name, not the folder's
        let (_, p) = score("theme", "pkgs/theme-edit/testdata/theme.nix").unwrap();
        assert_eq!(p, vec![25, 26, 27, 28, 29]);
    }
}
