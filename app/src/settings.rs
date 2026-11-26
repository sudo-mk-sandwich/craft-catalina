//! Where the apps are installed: the standard Applications folder, the folder Craftalina itself
//! sits in, or a folder the user picks. The choice is kept in a small file between starts.

use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Location {
    Applications,
    NextToApp,
    Custom,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub location: Location,
    /// The folder of the "Custom" choice (kept even while another choice is active).
    pub custom: String,
    /// Also build the disk image (.dmg). Off: faster, and the app is all that is needed.
    pub dmg: bool,
    /// Leave out the command-line tool where the app's build allows it (faster).
    pub skip_cli: bool,
}

impl Default for Settings {
    fn default() -> Self {
        // Always start with Applications, wherever Craftalina itself was unpacked.
        Settings { location: Location::Applications, custom: String::new(), dmg: false, skip_cli: true }
    }
}

impl Settings {
    pub fn parse(text: &str) -> Settings {
        let mut s = Settings::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            match k.trim() {
                "location" => {
                    s.location = match v.trim() {
                        "next" => Location::NextToApp,
                        "custom" => Location::Custom,
                        _ => Location::Applications,
                    }
                }
                "custom" => s.custom = v.trim().to_string(),
                "dmg" => s.dmg = v.trim() == "1",
                "skip_cli" => s.skip_cli = v.trim() != "0",
                _ => {}
            }
        }
        if s.location == Location::Custom && s.custom.is_empty() {
            s.location = Location::Applications;
        }
        s
    }

    pub fn serialize(&self) -> String {
        let loc = match self.location {
            Location::Applications => "applications",
            Location::NextToApp => "next",
            Location::Custom => "custom",
        };
        format!(
            "location={loc}\ncustom={}\ndmg={}\nskip_cli={}\n",
            self.custom,
            u8::from(self.dmg),
            u8::from(self.skip_cli)
        )
    }

    /// The folder for CRAFT_INSTALL_DIR, or None for the standard Applications folder.
    pub fn install_dir(&self, next_to_app: Option<&Path>) -> Option<String> {
        match self.location {
            Location::Applications => None,
            Location::NextToApp => next_to_app.map(|p| p.to_string_lossy().into_owned()),
            Location::Custom => Some(self.custom.clone()).filter(|c| !c.is_empty()),
        }
    }
}

fn file_path() -> Option<PathBuf> {
    let base = match std::env::var("CRAFT_BUILD_DIR") {
        Ok(d) if !d.is_empty() => PathBuf::from(d),
        _ => PathBuf::from(std::env::var("HOME").ok()?).join("craft-build"),
    };
    Some(base.join(".craftalina-settings"))
}

pub fn load() -> Settings {
    file_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|t| Settings::parse(&t))
        .unwrap_or_default()
}

pub fn save(s: &Settings) {
    let Some(path) = file_path() else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, s.serialize());
}

/// The folder that holds Craftalina.app. None when Craftalina does not run from an app bundle
/// (started from the source) or when that folder is the normal Applications folder anyway.
pub fn next_to_app_from(exe: &Path) -> Option<PathBuf> {
    let bundle = exe.ancestors().find(|a| a.extension().map(|e| e == "app").unwrap_or(false))?;
    let parent = bundle.parent()?;
    if parent == Path::new("/Applications") || parent == Path::new("/") {
        return None;
    }
    Some(parent.to_path_buf())
}

pub fn next_to_app() -> Option<PathBuf> {
    next_to_app_from(&std::env::current_exe().ok()?)
}

/// Text of the macOS folder dialog's answer: "/Users/x/ArtCraft/\n" -> "/Users/x/ArtCraft".
pub fn clean_dialog_answer(out: &str) -> Option<String> {
    let t = out.trim();
    if t.is_empty() {
        return None;
    }
    let t = if t.len() > 1 { t.trim_end_matches('/') } else { t };
    Some(t.to_string())
}

/// Asks for a folder with the normal macOS dialog. None when it was cancelled or failed.
pub fn choose_folder(prompt: &str) -> Option<String> {
    let script = format!(
        "POSIX path of (choose folder with prompt \"{}\")",
        prompt.replace('\\', "").replace('"', "")
    );
    // CRAFTALINA_OSASCRIPT is only used by the tests, which have no macOS dialog.
    let tool = std::env::var("CRAFTALINA_OSASCRIPT").unwrap_or_else(|_| "/usr/bin/osascript".to_string());
    let out = std::process::Command::new(tool)
        .args(["-e", &script])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    clean_dialog_answer(&String::from_utf8_lossy(&out.stdout))
}

/// A folder for display: the home folder is written as "~".
pub fn pretty(path: &str) -> String {
    match std::env::var("HOME") {
        Ok(h) if !h.is_empty() && path == h => "~".to_string(),
        Ok(h) if !h.is_empty() && path.starts_with(&format!("{h}/")) => format!("~{}", &path[h.len()..]),
        _ => path.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_applications() {
        assert_eq!(Settings::default().location, Location::Applications);
        assert_eq!(Settings::parse("").location, Location::Applications);
        assert_eq!(Settings::parse("garbage").location, Location::Applications);
    }

    #[test]
    fn build_options_default_and_old_files() {
        // Faster by default: no disk image, no command-line tool. A settings file from an older
        // Craftalina has neither key and gets the same defaults.
        let d = Settings::default();
        assert!(!d.dmg && d.skip_cli);
        let old = Settings::parse("location=next\ncustom=/x\n");
        assert!(!old.dmg && old.skip_cli);
        assert!(Settings::parse("dmg=1\nskip_cli=0\n").dmg);
        assert!(!Settings::parse("dmg=1\nskip_cli=0\n").skip_cli);
    }

    #[test]
    fn roundtrip() {
        for s in [
            Settings { location: Location::Applications, custom: String::new(), dmg: false, skip_cli: true },
            Settings { location: Location::NextToApp, custom: "/x/y".into(), dmg: true, skip_cli: false },
            Settings { location: Location::Custom, custom: "/Users/a b/ArtCraft".into(), dmg: false, skip_cli: false },
        ] {
            assert_eq!(Settings::parse(&s.serialize()), s);
        }
        // Custom without a folder is not usable.
        assert_eq!(Settings::parse("location=custom\n").location, Location::Applications);
    }

    #[test]
    fn install_dir_choice() {
        let next = PathBuf::from("/Users/a/ArtCraft");
        let s = |location| Settings { location, custom: "/c".into(), ..Settings::default() };
        assert_eq!(s(Location::Applications).install_dir(Some(&next)), None);
        assert_eq!(s(Location::NextToApp).install_dir(Some(&next)), Some("/Users/a/ArtCraft".into()));
        assert_eq!(s(Location::NextToApp).install_dir(None), None);
        assert_eq!(s(Location::Custom).install_dir(None), Some("/c".into()));
    }

    #[test]
    fn next_to_app_paths() {
        let f = |p: &str| next_to_app_from(Path::new(p));
        assert_eq!(
            f("/Applications/ArtCraft/Craftalina.app/Contents/MacOS/craftalina"),
            Some(PathBuf::from("/Applications/ArtCraft"))
        );
        assert_eq!(f("/Applications/Craftalina.app/Contents/MacOS/craftalina"), None);
        assert_eq!(f("/Users/a/Downloads/Craftalina.app/Contents/MacOS/craftalina"), Some(PathBuf::from("/Users/a/Downloads")));
        // Not from an app bundle (cargo run).
        assert_eq!(f("/home/x/craft-catalina/app/target/release/craftalina"), None);
    }

    #[test]
    fn dialog_answer() {
        assert_eq!(clean_dialog_answer("/Users/a/ArtCraft/\n"), Some("/Users/a/ArtCraft".into()));
        assert_eq!(clean_dialog_answer("/\n"), Some("/".into()));
        assert_eq!(clean_dialog_answer("  \n"), None);
    }
}
