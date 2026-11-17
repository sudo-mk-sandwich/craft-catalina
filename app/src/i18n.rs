//! English and German texts. Every text is a variant of `K`, so a missing translation does not
//! compile. The language follows the system setting and can be switched in the window.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    En,
    De,
}

impl Lang {
    /// Picks German when the system language (or $LANG) starts with "de".
    pub fn detect() -> Lang {
        let from_system = std::process::Command::new("defaults")
            .args(["read", "-g", "AppleLanguages"])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string());
        if let Some(text) = from_system {
            // Output looks like: ( "de-DE", "en-DE" ). The first entry is the preferred language.
            let first = text
                .split('"')
                .nth(1)
                .unwrap_or("")
                .to_ascii_lowercase();
            if first.starts_with("de") {
                return Lang::De;
            }
            if !first.is_empty() {
                return Lang::En;
            }
        }
        match std::env::var("LANG") {
            Ok(v) if v.to_ascii_lowercase().starts_with("de") => Lang::De,
            _ => Lang::En,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum K {
    Title,
    Subtitle,
    Requirements,
    ReqTools,
    ReqToolsHelp,
    ReqRust,
    ReqRustHelp,
    ReqTarget,
    ReqDisk,
    ReqDiskHelp,
    ReqArch,
    ReqArchHelp,
    Install,
    InstallingTools,
    InstallingRust,
    CheckAgain,
    ChooseApp,
    Build,
    UpdateBtn,
    UpdateNote,
    PickFirst,
    NeedRequirements,
    Preparing,
    ScriptMissingTitle,
    ScriptMissingText,
    Free,
    GigabyteShort,
    MacosVersion,
    LastLine,
    Cancel,
    Cancelling,
    ShowLog,
    WaitHint,
    BuildFailedTitle,
    BuildCancelled,
    BackToList,
    ShowInFinder,
    OpenApp,
    OpenLogInFinder,
    ReportIssue,
    InstallSection,
    CopyToApps,
    ReplaceInApps,
    CopyingNow,
    CleanupSection,
    KeepFiles,
    KeptNote,
    DeleteNeedsCopy,
    DeletingNow,
    ErrorPrefix,
    HintUndefined,
    HintFramework,
    HintPatch,
    HintGeneric,
    NotAnApp,
}

fn p(l: Lang, en: &'static str, de: &'static str) -> &'static str {
    match l {
        Lang::En => en,
        Lang::De => de,
    }
}

pub fn t(l: Lang, k: K) -> &'static str {
    match k {
        K::Title => "Craftalina",
        K::Subtitle => p(
            l,
            "Builds apps from source for Intel Macs with macOS 10.15 Catalina. Unofficial community tool.",
            "Baut Apps aus dem Quellcode für Intel-Macs mit macOS 10.15 Catalina. Inoffizielles Community-Werkzeug.",
        ),
        K::Requirements => p(l, "1. What is needed", "1. Was gebraucht wird"),
        K::ReqTools => p(l, "Apple developer tools", "Apple-Entwicklerwerkzeuge"),
        K::ReqToolsHelp => p(
            l,
            "Apple opens its own installer window. Click Install there and wait until it is finished; this can take a while. If macOS says the software is not available, see the README.",
            "Apple öffnet ein eigenes Installationsfenster. Dort auf Installieren klicken und warten, bis es fertig ist; das kann dauern. Sagt macOS, die Software sei nicht verfügbar, steht Näheres in der README.",
        ),
        K::ReqRust => p(l, "Rust", "Rust"),
        K::ReqRustHelp => p(
            l,
            "Installed with the official installer from rustup.rs into your home folder. No administrator password needed.",
            "Wird mit dem offiziellen Installer von rustup.rs in deinen Benutzerordner installiert. Kein Administrator-Passwort nötig.",
        ),
        K::ReqTarget => p(l, "Rust target for Intel Macs", "Rust-Ziel für Intel-Macs"),
        K::ReqDisk => p(l, "Free disk space", "Freier Speicherplatz"),
        K::ReqDiskHelp => p(
            l,
            "A build needs several GB. At least 10 GB should be free.",
            "Ein Build braucht mehrere GB. Mindestens 10 GB sollten frei sein.",
        ),
        K::ReqArch => p(l, "Intel Mac", "Intel-Mac"),
        K::ReqArchHelp => p(
            l,
            "This Mac is not an Intel Mac. The apps already run on Apple silicon with macOS 11 or newer, you do not need this tool.",
            "Dieser Mac ist kein Intel-Mac. Die Apps laufen auf Apple-Silicon mit macOS 11 oder neuer ohnehin, du brauchst dieses Werkzeug nicht.",
        ),
        K::Install => p(l, "Install", "Installieren"),
        K::InstallingTools => p(
            l,
            "Waiting for Apple's installer ...",
            "Warte auf Apples Installer ...",
        ),
        K::InstallingRust => p(l, "Installing ...", "Installiere ..."),
        K::CheckAgain => p(l, "Check again", "Erneut prüfen"),
        K::ChooseApp => p(l, "2. Which app do you want to build?", "2. Welche App soll gebaut werden?"),
        K::Build => p(l, "Build", "Bauen"),
        K::UpdateBtn => p(l, "Update", "Aktualisieren"),
        K::UpdateNote => p(
            l,
            "Note: the app's own update function breaks the Catalina patch. To update, use Craftalina.",
            "Hinweis: Die Update-Funktion in der App selbst zerstört den Catalina-Patch. Zum Aktualisieren bitte Craftalina benutzen.",
        ),
        K::PickFirst => p(l, "Pick an app in the list.", "Wähle eine App in der Liste."),
        K::NeedRequirements => p(
            l,
            "Install the missing items above first.",
            "Installiere zuerst die oben fehlenden Teile.",
        ),
        K::Preparing => p(l, "Getting the newest build script ...", "Hole das neueste Bauskript ..."),
        K::ScriptMissingTitle => p(l, "install.sh not found", "install.sh nicht gefunden"),
        K::ScriptMissingText => p(
            l,
            "Craftalina could not get install.sh from GitHub. Check your internet connection and start the program again. (Started from the source? Run it inside the craft-catalina folder, or set CRAFT_SCRIPT to the path of install.sh.)",
            "Craftalina konnte install.sh nicht von GitHub holen. Prüfe die Internetverbindung und starte das Programm neu. (Aus dem Quellcode gestartet? Dann im Ordner craft-catalina ausführen oder CRAFT_SCRIPT auf den Pfad von install.sh setzen.)",
        ),
        K::Free => p(l, "free", "frei"),
        K::GigabyteShort => "GB",
        K::MacosVersion => "macOS",
        K::LastLine => p(l, "Last message", "Letzte Meldung"),
        K::Cancel => p(l, "Cancel build", "Bau abbrechen"),
        K::Cancelling => p(l, "Stopping ...", "Breche ab ..."),
        K::ShowLog => p(l, "Show details (log)", "Details anzeigen (Protokoll)"),
        K::WaitHint => p(
            l,
            "A build can take from 30 minutes to well over an hour. Leave this window open; the Mac is kept awake while it runs.",
            "Ein Build dauert von 30 Minuten bis deutlich über eine Stunde. Lass dieses Fenster offen; der Mac schläft währenddessen nicht ein.",
        ),
        K::BuildFailedTitle => p(l, "The build stopped", "Der Build wurde gestoppt"),
        K::BuildCancelled => p(l, "The build was cancelled.", "Der Build wurde abgebrochen."),
        K::BackToList => p(l, "Back to the list", "Zurück zur Liste"),
        K::ShowInFinder => p(l, "Show in Finder", "Im Finder zeigen"),
        K::OpenApp => p(l, "Open the app", "App öffnen"),
        K::OpenLogInFinder => p(l, "Show log file in Finder", "Protokolldatei im Finder zeigen"),
        K::ReportIssue => p(l, "Report on GitHub", "Auf GitHub melden"),
        K::InstallSection => p(l, "Put the app into your Applications folder?", "App in den Programme-Ordner legen?"),
        K::CopyToApps => p(l, "Copy to Applications", "In Programme kopieren"),
        K::ReplaceInApps => p(l, "Replace in Applications", "In Programme ersetzen"),
        K::CopyingNow => p(l, "Copying ...", "Kopiere ..."),
        K::CleanupSection => p(l, "Keep the source code and build files?", "Quellcode und Build-Dateien behalten?"),
        K::KeepFiles => p(l, "Keep them", "Behalten"),
        K::KeptNote => p(l, "Kept. The next update will be much faster.", "Behalten. Das nächste Update geht deutlich schneller."),
        K::DeleteNeedsCopy => p(
            l,
            "Copy the app to Applications first: the build folder also holds the finished app, and it would be deleted with it.",
            "Kopiere die App zuerst nach Programme: Der Build-Ordner enthält auch die fertige App, sie würde mitgelöscht.",
        ),
        K::DeletingNow => p(l, "Deleting ...", "Lösche ..."),
        K::ErrorPrefix => p(l, "Message", "Meldung"),
        K::HintUndefined => p(
            l,
            "The app uses a system function that Catalina does not have. It needs a patch for this app. Please report it on GitHub and attach the log file.",
            "Die App nutzt eine Systemfunktion, die Catalina nicht hat. Dafür braucht diese App einen Patch. Bitte melde es auf GitHub und hänge die Protokolldatei an.",
        ),
        K::HintFramework => p(
            l,
            "The app needs a system framework that Catalina does not have. It needs a patch for this app. Please report it on GitHub and attach the log file.",
            "Die App braucht ein System-Framework, das Catalina nicht hat. Dafür braucht diese App einen Patch. Bitte melde es auf GitHub und hänge die Protokolldatei an.",
        ),
        K::HintPatch => p(
            l,
            "The patch for this app does not fit the newest version. Please report it on GitHub; meanwhile an older version can be built with the terminal script.",
            "Der Patch für diese App passt nicht zur neuesten Version. Bitte melde es auf GitHub; solange lässt sich über das Terminal-Skript eine ältere Version bauen.",
        ),
        K::HintGeneric => p(
            l,
            "Open the details to see where it stopped. If you cannot tell, report it on GitHub and attach the log file.",
            "Öffne die Details, um zu sehen, wo es stehen blieb. Wenn du es nicht erkennst, melde es auf GitHub und hänge die Protokolldatei an.",
        ),
        K::NotAnApp => p(l, "No finished app was found.", "Es wurde keine fertige App gefunden."),
    }
}

/// Short descriptions of the apps. Falls back to the English text from install.sh.
pub fn app_description(l: Lang, name: &str, fallback: &str) -> String {
    if l == Lang::En {
        return fallback.to_string();
    }
    let de = match name {
        "anicraft" => "2D-Vektoranimation",
        "cadcraft" => "CAD und technisches Zeichnen",
        "craft-anything" => "Software erkunden und erstellen (frühe Alpha)",
        "deckcraft" => "Präsentationen",
        "designcraft" => "Seitenlayout und Publishing",
        "effectcraft" => "Motion Graphics und VFX",
        "figcraft" => "Oberflächendesign und Prototypen",
        "filmcraft" => "Videoschnitt",
        "gridcraft" => "Tabellen und Datenanalyse",
        "lightcraft" => "Fotobibliothek und RAW-Entwickler",
        "mailcraft" => "Mail, Kalender, Kontakte und Aufgaben (andere Autoren)",
        "materialcraft" => "Prozedurale Materialien",
        "pdfcraft" => "PDF-Werkzeug",
        "photocraft" => "Bildbearbeitung",
        "solidcraft" => "3D-CAD (andere Autoren)",
        "soundcraft" => "Digitale Audio-Workstation",
        "texturecraft" => "3D-Texturen malen",
        "vectorcraft" => "Vektorillustration",
        "wordcraft" => "Textverarbeitung",
        _ => return fallback.to_string(),
    };
    de.to_string()
}

// Texts with a value in them.

pub fn building_title(l: Lang, app: &str) -> String {
    match l {
        Lang::En => format!("Building {app}"),
        Lang::De => format!("{app} wird gebaut"),
    }
}

pub fn elapsed(l: Lang, time: &str) -> String {
    match l {
        Lang::En => format!("Elapsed time: {time}"),
        Lang::De => format!("Vergangene Zeit: {time}"),
    }
}

pub fn built_title(l: Lang, app: &str, version: &str) -> String {
    let v = if version.is_empty() { String::new() } else { format!(" {version}") };
    match l {
        Lang::En => format!("{app}{v} is ready"),
        Lang::De => format!("{app}{v} ist fertig"),
    }
}

pub fn already_installed(l: Lang, version: &str) -> String {
    let v = if version.is_empty() { String::new() } else { format!(" ({version})") };
    match l {
        Lang::En => format!("A version{v} is already in the Applications folder. Copying replaces it."),
        Lang::De => format!("Im Programme-Ordner liegt schon eine Version{v}. Beim Kopieren wird sie ersetzt."),
    }
}

pub fn copied_to(l: Lang, path: &str) -> String {
    match l {
        Lang::En => format!("Copied to the Applications folder{path}"),
        Lang::De => format!("In den Programme-Ordner kopiert{path}"),
    }
}

pub fn cleanup_text(l: Lang, size: &str) -> String {
    match l {
        Lang::En => format!("The sources and build files use {size}. Keeping them makes the next update of this app much faster; deleting frees the space."),
        Lang::De => format!("Die Quellen und Build-Dateien belegen {size}. Behält man sie, geht das nächste Update dieser App viel schneller; Löschen gibt den Platz frei."),
    }
}

pub fn delete_button(l: Lang, size: &str) -> String {
    match l {
        Lang::En => format!("Delete ({size})"),
        Lang::De => format!("Löschen ({size})"),
    }
}

pub fn deleted_note(l: Lang, size: &str) -> String {
    match l {
        Lang::En => format!("Deleted, {size} freed."),
        Lang::De => format!("Gelöscht, {size} frei geworden."),
    }
}

/// Status text in the app list for an installed app. `latest` is the newest stable version when it
/// is known: the text then says whether the installed one is current or can be updated.
pub fn installed_status(l: Lang, installed: &str, latest: Option<&str>, update: bool) -> String {
    match (l, latest, update) {
        (Lang::En, Some(n), true) => format!("Update: {installed} -> {n}"),
        (Lang::De, Some(n), true) => format!("Update: {installed} -> {n}"),
        (Lang::En, Some(_), false) => format!("Installed {installed}, current"),
        (Lang::De, Some(_), false) => format!("Installiert {installed}, aktuell"),
        (Lang::En, None, _) => format!("Installed {installed}"),
        (Lang::De, None, _) => format!("Installiert {installed}"),
    }
}
