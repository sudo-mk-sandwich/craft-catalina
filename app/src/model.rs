//! The state of a build and of its result, driven by the "@@" lines of install.sh.
//! No window code in here, so it can be tested without a screen.

use crate::backend::{parse_marker, PidHandle};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

const LOG_LIMIT: usize = 20_000;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

pub struct BuildState {
    pub app: String,
    pub steps: Vec<String>,
    pub log: VecDeque<String>,
    pub error: Option<String>,
    pub kind: Option<String>,
    pub log_path: Option<String>,
    pub done_path: Option<String>,
    pub status: Status,
    pub cancel_requested: bool,
    pub started: Instant,
    pub ended: Option<Instant>,
    pub pid: Option<PidHandle>,
}

impl BuildState {
    pub fn new(app: &str) -> Self {
        BuildState {
            app: app.to_string(),
            steps: Vec::new(),
            log: VecDeque::new(),
            error: None,
            kind: None,
            log_path: None,
            done_path: None,
            status: Status::Running,
            cancel_requested: false,
            started: Instant::now(),
            ended: None,
            pid: None,
        }
    }

    pub fn apply_line(&mut self, line: &str) {
        match parse_marker(line) {
            Some(("STEP", rest)) => self.steps.push(rest.to_string()),
            Some(("ERROR", rest)) => self.error = Some(rest.to_string()),
            Some(("KIND", rest)) => self.kind = Some(rest.to_string()),
            Some(("LOG", rest)) => self.log_path = Some(rest.to_string()),
            Some(("DONE", rest)) => self.done_path = Some(rest.to_string()),
            Some(("WARN", _)) => {} // the same text is in the log already
            Some(_) => {}
            None => {
                if self.log.len() >= LOG_LIMIT {
                    self.log.pop_front();
                }
                self.log.push_back(line.to_string());
            }
        }
    }

    pub fn finish(&mut self) {
        self.ended = Some(Instant::now());
        self.status = if self.done_path.is_some() {
            Status::Succeeded
        } else if self.cancel_requested {
            Status::Cancelled
        } else {
            Status::Failed
        };
    }

    pub fn elapsed(&self) -> Duration {
        self.ended.unwrap_or_else(Instant::now).duration_since(self.started)
    }

    pub fn last_log_line(&self) -> &str {
        self.log.iter().rev().find(|l| !l.trim().is_empty()).map(|s| s.trim()).unwrap_or("")
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Busy {
    Copying,
    Cleaning,
}

#[derive(Default)]
pub struct ResultState {
    pub app: String,
    pub app_path: String,
    pub version: String,
    pub installed_path: Option<String>,
    pub installed_version: String,
    pub size_kb: u64,
    pub busy: Option<Busy>,
    pub copied_to: Option<String>,
    pub cleaned_kb: Option<u64>,
    pub kept: bool,
    pub error: Option<String>,
}

impl ResultState {
    pub fn new(app: &str, app_path: &str) -> Self {
        ResultState { app: app.to_string(), app_path: app_path.to_string(), ..Default::default() }
    }

    pub fn apply_line(&mut self, line: &str) {
        match parse_marker(line) {
            Some(("APP_PATH", rest)) => self.app_path = rest.to_string(),
            Some(("BUILT_VERSION", rest)) => self.version = rest.to_string(),
            Some(("INSTALLED", rest)) => {
                self.installed_path = rest.strip_prefix("yes ").map(|p| p.to_string());
            }
            Some(("INSTALLED_VERSION", rest)) => self.installed_version = rest.to_string(),
            Some(("SIZE_KB", rest)) => self.size_kb = rest.parse().unwrap_or(0),
            Some(("COPIED", rest)) => {
                self.copied_to = Some(rest.to_string());
                self.installed_path = Some(rest.to_string());
                self.error = None;
            }
            Some(("CLEANED", rest)) => self.cleaned_kb = Some(rest.parse().unwrap_or(0)),
            Some(("ERROR", rest)) => self.error = Some(rest.to_string()),
            _ => {}
        }
    }
}

pub fn format_duration(d: Duration) -> String {
    let s = d.as_secs();
    let (h, m, sec) = (s / 3600, (s % 3600) / 60, s % 60);
    if h > 0 {
        format!("{h}:{m:02}:{sec:02}")
    } else {
        format!("{m:02}:{sec:02}")
    }
}

/// Sizes in KB (as `du -sk` reports them) as "812 MB" / "3.4 GB"; `comma` for German.
pub fn format_size(kb: u64, comma: bool) -> String {
    let out = if kb >= 1_048_576 {
        format!("{:.1} GB", kb as f64 / 1_048_576.0)
    } else {
        format!("{} MB", kb / 1024)
    };
    if comma {
        out.replace('.', ",")
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_success_sequence() {
        let mut b = BuildState::new("wordcraft");
        for l in ["@@STEP App: wordcraft", "Compiling foo v1.0", "@@STEP Building", "   Compiling bar", "@@STEP Done", "@@DONE /x/WordCraft.app"] {
            b.apply_line(l);
        }
        b.finish();
        assert_eq!(b.status, Status::Succeeded);
        assert_eq!(b.steps, vec!["App: wordcraft", "Building", "Done"]);
        assert_eq!(b.last_log_line(), "Compiling bar");
        assert_eq!(b.done_path.as_deref(), Some("/x/WordCraft.app"));
    }

    #[test]
    fn build_failure_and_cancel() {
        let mut b = BuildState::new("x");
        b.apply_line("@@LOG /tmp/l.log");
        b.apply_line("@@KIND undefined-symbols");
        b.apply_line("@@ERROR The build stopped");
        b.finish();
        assert_eq!(b.status, Status::Failed);
        assert_eq!(b.kind.as_deref(), Some("undefined-symbols"));
        let mut c = BuildState::new("x");
        c.cancel_requested = true;
        c.finish();
        assert_eq!(c.status, Status::Cancelled);
    }

    #[test]
    fn result_lines() {
        let mut r = ResultState::new("photocraft", "");
        for l in ["@@SIZE_KB 2097152", "@@APP_PATH /b/PhotoCraft.app", "@@BUILT_VERSION 0.23.1", "@@INSTALLED yes /Applications/PhotoCraft.app", "@@INSTALLED_VERSION 0.22.0"] {
            r.apply_line(l);
        }
        assert_eq!(r.version, "0.23.1");
        assert_eq!(r.installed_path.as_deref(), Some("/Applications/PhotoCraft.app"));
        r.apply_line("@@COPIED /Applications/PhotoCraft.app");
        assert!(r.copied_to.is_some());
        r.apply_line("@@CLEANED 2097152");
        assert_eq!(r.cleaned_kb, Some(2097152));
    }

    #[test]
    fn formats() {
        assert_eq!(format_duration(Duration::from_secs(75)), "01:15");
        assert_eq!(format_duration(Duration::from_secs(3 * 3600 + 5)), "3:00:05");
        assert_eq!(format_size(2_621_440, false), "2.5 GB");
        assert_eq!(format_size(2_621_440, true), "2,5 GB");
        assert_eq!(format_size(512_000, false), "500 MB");
    }
}
