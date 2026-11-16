//! Gets install.sh and patches/ when Craftalina runs as a downloaded app.
//!
//! The released app contains only the window. The build script and the patches change more often
//! than the window, so at every start the newest copy is downloaded from GitHub (a small archive
//! of the repository, no developer tools needed) into ~/craft-build/craft-catalina. If that fails
//! (offline), the copy from the last start is used, and as a last resort the copy bundled in the
//! app. A folder that is a git clone (someone ran `git clone` there) is never touched: install.sh
//! updates it with `git pull` itself.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const ARCHIVE_URL: &str = "https://github.com/sudo-mk-sandwich/craft-catalina/archive/HEAD.tar.gz";

/// True when `exe` lies in `<something>.app/Contents/MacOS/`.
pub fn is_bundle_exe(exe: &Path) -> bool {
    let parent = match exe.parent() {
        Some(p) => p,
        None => return false,
    };
    let contents = match parent.parent() {
        Some(p) => p,
        None => return false,
    };
    parent.file_name().map(|n| n == "MacOS").unwrap_or(false)
        && contents.file_name().map(|n| n == "Contents").unwrap_or(false)
        && contents
            .parent()
            .and_then(|a| a.extension())
            .map(|e| e == "app")
            .unwrap_or(false)
}

/// Where the downloaded copy lives: $CRAFT_BUILD_DIR/craft-catalina, default ~/craft-build/craft-catalina.
fn managed_dir() -> Option<PathBuf> {
    if let Ok(d) = std::env::var("CRAFT_BUILD_DIR") {
        if !d.is_empty() {
            return Some(Path::new(&d).join("craft-catalina"));
        }
    }
    let home = std::env::var("HOME").ok()?;
    Some(Path::new(&home).join("craft-build/craft-catalina"))
}

fn bundled_script() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let p = exe.parent()?.join("../Resources/install.sh");
    p.is_file().then_some(p)
}

/// Returns the install.sh to use. May take a few seconds (download), so call it off the UI thread.
pub fn prepare() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("CRAFT_SCRIPT") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    let in_bundle = std::env::current_exe().map(|e| is_bundle_exe(&e)).unwrap_or(false);
    if !in_bundle {
        // Started from the source (cargo run): use the script in the repository.
        return crate::backend::find_script();
    }
    let dest = match managed_dir() {
        Some(d) => d,
        None => return bundled_script(),
    };
    let script = dest.join("install.sh");
    if dest.join(".git").exists() {
        return if script.is_file() { Some(script) } else { bundled_script() };
    }
    let _ = download(ARCHIVE_URL, &dest);
    if script.is_file() {
        Some(script)
    } else {
        bundled_script()
    }
}

fn run(cmd: &mut Command) -> Result<(), String> {
    let st = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| e.to_string())?;
    if st.success() {
        Ok(())
    } else {
        Err(format!("exit status {st}"))
    }
}

/// Downloads the archive at `url` and replaces `dest` with its content. On any error `dest` stays
/// as it was.
pub fn download(url: &str, dest: &Path) -> Result<(), String> {
    let parent = dest.parent().ok_or("no parent folder")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let work = parent.join(".craftalina-download");
    let _ = std::fs::remove_dir_all(&work);
    let unpack = work.join("src");
    std::fs::create_dir_all(&unpack).map_err(|e| e.to_string())?;
    let archive = work.join("src.tar.gz");

    let result = (|| -> Result<(), String> {
        run(Command::new("/usr/bin/curl")
            .args(["-fsSL", "--max-time", "90", "-o"])
            .arg(&archive)
            .arg(url))?;
        run(Command::new("/usr/bin/tar")
            .arg("-xzf")
            .arg(&archive)
            .arg("-C")
            .arg(&unpack)
            .arg("--strip-components=1"))?;
        if !unpack.join("install.sh").is_file() || !unpack.join("patches").is_dir() {
            return Err("the archive has no install.sh and patches/".to_string());
        }
        let old = parent.join("craft-catalina.old");
        let _ = std::fs::remove_dir_all(&old);
        let had_old = dest.exists();
        if had_old {
            std::fs::rename(dest, &old).map_err(|e| e.to_string())?;
        }
        if let Err(e) = std::fs::rename(&unpack, dest) {
            if had_old {
                let _ = std::fs::rename(&old, dest);
            }
            return Err(e.to_string());
        }
        let _ = std::fs::remove_dir_all(&old);
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(&work);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("craftalina-test-{}-{}", std::process::id(), name));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn make_archive(root: &Path, marker: &str) -> PathBuf {
        let src = root.join("repo-main");
        std::fs::create_dir_all(src.join("patches")).unwrap();
        std::fs::write(src.join("install.sh"), marker).unwrap();
        std::fs::write(src.join("patches/a.patch"), "x").unwrap();
        let tgz = root.join("repo.tar.gz");
        run(Command::new("tar").arg("-czf").arg(&tgz).arg("-C").arg(root).arg("repo-main")).unwrap();
        tgz
    }

    #[test]
    fn bundle_detection() {
        assert!(is_bundle_exe(Path::new("/Applications/Craftalina.app/Contents/MacOS/craftalina")));
        assert!(!is_bundle_exe(Path::new("/home/u/craft-catalina/app/target/release/craftalina")));
        assert!(!is_bundle_exe(Path::new("/x/MacOS/craftalina")));
    }

    #[test]
    fn download_replaces_and_keeps_old_on_error() {
        let root = temp("dl");
        let tgz = make_archive(&root, "one");
        let dest = root.join("home/craft-build/craft-catalina");
        let url = format!("file://{}", tgz.display());
        download(&url, &dest).unwrap();
        assert_eq!(std::fs::read_to_string(dest.join("install.sh")).unwrap(), "one");

        // A newer archive replaces the folder completely.
        std::fs::write(root.join("repo-main/install.sh"), "two").unwrap();
        run(Command::new("tar").arg("-czf").arg(&tgz).arg("-C").arg(&root).arg("repo-main")).unwrap();
        download(&url, &dest).unwrap();
        assert_eq!(std::fs::read_to_string(dest.join("install.sh")).unwrap(), "two");
        assert!(!dest.parent().unwrap().join("craft-catalina.old").exists());
        assert!(!dest.parent().unwrap().join(".craftalina-download").exists());

        // A failing download leaves the folder as it was.
        assert!(download("file:///does/not/exist.tar.gz", &dest).is_err());
        assert_eq!(std::fs::read_to_string(dest.join("install.sh")).unwrap(), "two");

        // An archive without install.sh is refused.
        let bad = root.join("bad.tar.gz");
        std::fs::create_dir_all(root.join("bad-main")).unwrap();
        std::fs::write(root.join("bad-main/readme"), "x").unwrap();
        run(Command::new("tar").arg("-czf").arg(&bad).arg("-C").arg(&root).arg("bad-main")).unwrap();
        assert!(download(&format!("file://{}", bad.display()), &dest).is_err());
        assert_eq!(std::fs::read_to_string(dest.join("install.sh")).unwrap(), "two");
        let _ = std::fs::remove_dir_all(&root);
    }
}
