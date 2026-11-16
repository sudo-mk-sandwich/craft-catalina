//! Asks GitHub for the newest stable version of an app and compares versions.
//! (Which apps are installed comes from `install.sh --installed`.)
//!
//! It reads the list of tags the same way `git ls-remote --tags` does (the public "smart HTTP"
//! address of the repository, fetched with curl). That needs no git, no login, and is not subject
//! to the strict limit of the GitHub API. The answer is kept for an hour in a small file so that
//! restarting the window does not ask again.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

const CACHE_SECONDS: u64 = 3600;

/// "owner/repo" of an app. Mirrors repo_url() in install.sh.
pub fn repo_slug(app: &str) -> String {
    match app {
        "mailcraft" => "UCS-Care-Group/mailcraft".to_string(),
        "solidcraft" => "tangency-labs/solidcraft".to_string(),
        other => format!("storytold/{other}"),
    }
}

/// Tag names found in the advertisement of the git server (without the `^{}` duplicates).
pub fn parse_tags(data: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(data);
    let mut tags = Vec::new();
    for piece in text.split("refs/tags/").skip(1) {
        let end = piece
            .find(|c: char| c.is_whitespace() || c == '\0')
            .unwrap_or(piece.len());
        let name = &piece[..end];
        if !name.is_empty() && !name.ends_with("^{}") {
            tags.push(name.to_string());
        }
    }
    tags
}

/// Numbers of a version: "v0.18.1" -> [0, 18, 1]. A suffix after '-' or '+' is ignored.
pub fn version_key(v: &str) -> Vec<u64> {
    let v = v.trim().trim_start_matches('v');
    let core = v.split(|c| c == '-' || c == '+').next().unwrap_or("");
    core.split('.').map_while(|p| p.parse::<u64>().ok()).collect()
}

fn cmp_keys(a: &[u64], b: &[u64]) -> std::cmp::Ordering {
    let n = a.len().max(b.len());
    for i in 0..n {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        if x != y {
            return x.cmp(&y);
        }
    }
    std::cmp::Ordering::Equal
}

/// The tag install.sh would build: the newest `v<number>...` tag without a '-' (stable), and only
/// when there is none, the newest one at all. Returned without the leading "v".
pub fn latest_stable(tags: &[String]) -> Option<String> {
    let versions: Vec<&String> = tags
        .iter()
        .filter(|t| t.starts_with('v') && t[1..].starts_with(|c: char| c.is_ascii_digit()))
        .collect();
    let newest = |list: Vec<&&String>| -> Option<String> {
        list.into_iter()
            .max_by(|a, b| cmp_keys(&version_key(a), &version_key(b)))
            .map(|t| t.trim_start_matches('v').to_string())
    };
    let stable: Vec<&&String> = versions.iter().filter(|t| !t.contains('-')).collect();
    if !stable.is_empty() {
        newest(stable)
    } else {
        newest(versions.iter().collect())
    }
}

/// True when `latest` is a higher version than `installed`. Unreadable versions never count.
pub fn is_newer(installed: &str, latest: &str) -> bool {
    let (a, b) = (version_key(installed), version_key(latest));
    !a.is_empty() && !b.is_empty() && cmp_keys(&b, &a) == std::cmp::Ordering::Greater
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn cache_path() -> Option<PathBuf> {
    let base = match std::env::var("CRAFT_BUILD_DIR") {
        Ok(d) if !d.is_empty() => PathBuf::from(d),
        _ => PathBuf::from(std::env::var("HOME").ok()?).join("craft-build"),
    };
    Some(base.join(".craftalina-latest.tsv"))
}

fn read_cache(app: &str) -> Option<String> {
    let text = std::fs::read_to_string(cache_path()?).ok()?;
    for line in text.lines() {
        let mut f = line.split('\t');
        if f.next() == Some(app) {
            let when: u64 = f.next()?.parse().ok()?;
            let tag = f.next()?;
            if now().saturating_sub(when) < CACHE_SECONDS && !tag.is_empty() {
                return Some(tag.to_string());
            }
        }
    }
    None
}

fn write_cache(app: &str, tag: &str) {
    let Some(path) = cache_path() else { return };
    let old = std::fs::read_to_string(&path).unwrap_or_default();
    let mut out: Vec<String> = old
        .lines()
        .filter(|l| l.split('\t').next() != Some(app))
        .map(|l| l.to_string())
        .collect();
    out.push(format!("{app}\t{}\t{tag}", now()));
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, out.join("\n") + "\n");
}

/// Newest stable version of `app` (from the cache or from GitHub). None when it cannot be found
/// (offline, repository renamed, no tags): the window then shows nothing instead of a guess.
pub fn fetch_latest(app: &str) -> Option<String> {
    if let Some(tag) = read_cache(app) {
        return Some(tag);
    }
    let base = std::env::var("CRAFTALINA_GIT_BASE").unwrap_or_else(|_| "https://github.com".to_string());
    let url = format!("{base}/{}/info/refs?service=git-upload-pack", repo_slug(app));
    let out = Command::new("/usr/bin/curl")
        .args(["-fsSL", "--max-time", "25", "-A", "craftalina"])
        .arg(&url)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let tag = latest_stable(&parse_tags(&out.stdout))?;
    write_cache(app, &tag);
    Some(tag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs() {
        assert_eq!(repo_slug("deckcraft"), "storytold/deckcraft");
        assert_eq!(repo_slug("mailcraft"), "UCS-Care-Group/mailcraft");
        assert_eq!(repo_slug("solidcraft"), "tangency-labs/solidcraft");
    }

    #[test]
    fn tags_from_advertisement() {
        let data = b"001e# service=git-upload-pack\n0000004a084654c6456435fb7290f6502f3f635f25c890a3 refs/heads/main\n00480123 refs/tags/v0.9.0\n00480124 refs/tags/v0.9.0^{}\n00480125 refs/tags/v0.10.0\n0000";
        assert_eq!(parse_tags(data), vec!["v0.9.0".to_string(), "v0.10.0".to_string()]);
    }

    #[test]
    fn picks_newest_stable_like_install_sh() {
        let t = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        // 0.10 is newer than 0.9 (numbers, not text).
        assert_eq!(latest_stable(&t(&["v0.9.0", "v0.10.0", "v0.2.0"])), Some("0.10.0".into()));
        // Nightly tags are ignored when a stable one exists.
        assert_eq!(
            latest_stable(&t(&["v0.42.0-nightly.20261115.134.1", "v0.43.0", "v0.43.0-nightly.1"])),
            Some("0.43.0".into())
        );
        // Only pre-releases: take the newest of them.
        assert_eq!(latest_stable(&t(&["v0.1.0-alpha", "v0.2.0-alpha"])), Some("0.2.0-alpha".into()));
        // Tags that are no versions are ignored; nothing at all gives None.
        assert_eq!(latest_stable(&t(&["nightly", "release-1"])), None);
        assert_eq!(latest_stable(&[]), None);
    }

    #[test]
    fn comparing() {
        assert!(is_newer("0.18.1", "0.19.0"));
        assert!(is_newer("0.9.9", "0.10.0"));
        assert!(is_newer("0.18", "0.18.1"));
        assert!(!is_newer("0.18.1", "0.18.1"));
        assert!(!is_newer("0.19.0", "0.18.1"));
        assert!(!is_newer("?", "0.18.1"));
        assert!(!is_newer("0.18.1", ""));
    }
}
