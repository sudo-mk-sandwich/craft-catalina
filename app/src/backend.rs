//! Runs install.sh in the background and turns its "@@" lines into events for the window.

use std::io::{BufRead, BufReader};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TaskKind {
    Check,
    ListApps,
    Installed,
    InstallTools,
    InstallRust,
    Build,
    Info,
    Copy,
    Clean,
}

#[derive(Debug)]
pub enum Event {
    /// A line of output (stdout and stderr), without the line break.
    Line(TaskKind, String),
    /// The task ended: Some(exit code), or None when it could not be started or was killed.
    Done(TaskKind, Option<i32>),
}

/// Process id of a running task (0 = not started yet). The task is the leader of its own process
/// group, so stopping the group also stops cargo and the compilers below the script.
pub type PidHandle = Arc<AtomicU32>;

/// Looks for install.sh: CRAFT_SCRIPT, next to the program, in the app bundle's Resources folder,
/// in the current folder, and in ~/craft-build/craft-catalina.
pub fn find_script() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(p) = std::env::var("CRAFT_SCRIPT") {
        candidates.push(PathBuf::from(p));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("install.sh"));
            candidates.push(dir.join("../Resources/install.sh"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("install.sh"));
        candidates.push(cwd.join("../install.sh"));
    }
    if let Ok(home) = std::env::var("HOME") {
        candidates.push(Path::new(&home).join("craft-build/craft-catalina/install.sh"));
    }
    candidates.into_iter().find(|p| p.is_file())
}

/// Starts `bash <script> <args>` on a thread. Events arrive on `tx`; `repaint` wakes the window.
/// While `keep_awake` is set the Mac is kept from sleeping until the task is over.
pub fn spawn(
    script: PathBuf,
    kind: TaskKind,
    args: &[&str],
    keep_awake: bool,
    install_dir: Option<String>,
    tx: Sender<Event>,
    repaint: impl Fn() + Send + Clone + 'static,
) -> PidHandle {
    let pid_handle: PidHandle = Arc::new(AtomicU32::new(0));
    let pid_for_thread = pid_handle.clone();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    std::thread::spawn(move || {
        let mut cmd = Command::new("/bin/bash");
        cmd.arg(&script).args(&args);
        match &install_dir {
            Some(d) => cmd.env("CRAFT_INSTALL_DIR", d),
            None => cmd.env_remove("CRAFT_INSTALL_DIR"),
        };
        let child = cmd
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0)
            .spawn();
        let mut child = match child {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(Event::Line(kind, format!("Could not start bash: {e}")));
                let _ = tx.send(Event::Done(kind, None));
                repaint();
                return;
            }
        };
        pid_for_thread.store(child.id(), Ordering::SeqCst);
        if keep_awake {
            // caffeinate ends by itself when the process it watches ends.
            let _ = Command::new("/usr/bin/caffeinate")
                .args(["-i", "-w", &child.id().to_string()])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        }
        // stderr on its own thread so neither pipe can fill up and block the script.
        let stderr_thread = child.stderr.take().map(|err| {
            let tx = tx.clone();
            let repaint = repaint.clone();
            std::thread::spawn(move || {
                for line in BufReader::new(err).lines().map_while(Result::ok) {
                    let _ = tx.send(Event::Line(kind, line));
                    repaint();
                }
            })
        });
        if let Some(out) = child.stdout.take() {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                let _ = tx.send(Event::Line(kind, line));
                repaint();
            }
        }
        if let Some(t) = stderr_thread {
            let _ = t.join();
        }
        let code = child.wait().ok().and_then(|s| s.code());
        let _ = tx.send(Event::Done(kind, code));
        repaint();
    });
    pid_handle
}

/// Asks a running task (and everything it started) to stop.
pub fn terminate(pid: &PidHandle) {
    let id = pid.load(Ordering::SeqCst);
    if id != 0 {
        let _ = Command::new("/bin/bash")
            .args(["-c", &format!("kill -TERM -- -{id}")])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

/// Starts a program and does not wait for it (for `open`).
pub fn run_detached(program: &str, args: &[&str]) {
    let _ = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

/// One parsed "@@" line. `rest` is everything after the keyword, trimmed.
pub fn parse_marker(line: &str) -> Option<(&str, &str)> {
    let body = line.strip_prefix("@@")?;
    match body.find(|c: char| c == ' ' || c == '\t') {
        Some(i) => Some((&body[..i], body[i + 1..].trim())),
        None => Some((body, "")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_markers() {
        assert_eq!(parse_marker("@@CHECK tools ok 10.15"), Some(("CHECK", "tools ok 10.15")));
        assert_eq!(parse_marker("@@APP\tanicraft\t1\t2D"), Some(("APP", "anicraft\t1\t2D")));
        assert_eq!(parse_marker("@@DONE"), Some(("DONE", "")));
        assert_eq!(parse_marker("==> Building"), None);
    }
}
