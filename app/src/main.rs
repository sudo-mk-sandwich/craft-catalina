#![forbid(unsafe_code)]
//! Craftalina: a small window around install.sh. It checks what is needed, installs missing parts
//! on a click, builds the chosen app and offers to copy it to Applications and to clean up.
//! (Unofficial community tool, not affiliated with the authors of the apps.)

mod backend;
mod i18n;
mod model;

use backend::{parse_marker, Event, PidHandle, TaskKind};
use model::{format_duration, format_size, Busy, BuildState, ResultState, Status};
use eframe::egui::{self, Align, Color32, Layout, RichText, Shape, Stroke};
use i18n::{app_description, t, Lang, K};

const ISSUES_URL: &str = "https://github.com/sudo-mk-sandwich/craft-catalina/issues";
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

const GREEN: Color32 = Color32::from_rgb(46, 160, 67);
const RED: Color32 = Color32::from_rgb(207, 34, 46);
const ORANGE: Color32 = Color32::from_rgb(219, 135, 0);

#[derive(Clone, PartialEq, Debug)]
enum Check {
    Unknown,
    Ok(String),
    Missing,
    Low(String),
    Other,
}

impl Check {
    fn is_ok(&self) -> bool {
        matches!(self, Check::Ok(_))
    }
}

struct Prereqs {
    tools: Check,
    rust: Check,
    target: Check,
    disk: Check,
    arch: Check,
    macos: String,
}

struct AppEntry {
    name: String,
    descr: String,
}

#[derive(Clone, Copy)]
enum Icon {
    Ok,
    Bad,
    Warn,
    Busy,
    Unknown,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Select,
    Building,
    Done,
    Failed,
}

struct Builder {
    screen: Screen,
    build: Option<BuildState>,
    result: Option<ResultState>,
    show_log: bool,
    cancelled_notice: bool,
    lang: Lang,
    script: Option<PathBuf>,
    ctx: egui::Context,
    tx: Sender<Event>,
    rx: Receiver<Event>,
    prereqs: Prereqs,
    apps: Vec<AppEntry>,
    selected: Option<usize>,
    checking: bool,
    tools_wait: bool,
    rust_busy: bool,
    last_line: String,
    next_poll: Option<Instant>,
}

impl Builder {
    fn new(ctx: &egui::Context) -> Self {
        // Everything one point larger than egui's default.
        ctx.global_style_mut(|style| {
            for font in style.text_styles.values_mut() {
                font.size += 1.0;
            }
        });
        let (tx, rx) = channel();
        let mut b = Builder {
            screen: Screen::Select,
            build: None,
            result: None,
            show_log: false,
            cancelled_notice: false,
            lang: Lang::detect(),
            script: backend::find_script(),
            ctx: ctx.clone(),
            tx,
            rx,
            prereqs: Prereqs {
                tools: Check::Unknown,
                rust: Check::Unknown,
                target: Check::Unknown,
                disk: Check::Unknown,
                arch: Check::Unknown,
                macos: String::new(),
            },
            apps: Vec::new(),
            selected: None,
            checking: false,
            tools_wait: false,
            rust_busy: false,
            last_line: String::new(),
            next_poll: None,
        };
        b.start(TaskKind::ListApps, &["--list-apps"]);
        b.start_check();
        // For tests only: start a build without clicking.
        if let Ok(app) = std::env::var("CRAFTALINA_AUTOSTART") {
            b.start_build(&app);
        }
        b
    }

    fn start(&self, kind: TaskKind, args: &[&str]) -> Option<PidHandle> {
        self.start_awake(kind, args, false)
    }

    fn start_awake(&self, kind: TaskKind, args: &[&str], keep_awake: bool) -> Option<PidHandle> {
        let script = self.script.as_ref()?;
        let ctx = self.ctx.clone();
        Some(backend::spawn(script.clone(), kind, args, keep_awake, self.tx.clone(), move || {
            ctx.request_repaint()
        }))
    }

    fn start_build(&mut self, app: &str) {
        let mut state = BuildState::new(app);
        state.pid = self.start_awake(TaskKind::Build, &["--machine", "--build", app], true);
        self.build = Some(state);
        self.result = None;
        self.show_log = false;
        self.cancelled_notice = false;
        self.screen = Screen::Building;
    }

    fn cancel_build(&mut self) {
        if let Some(b) = self.build.as_mut() {
            if b.status == Status::Running && !b.cancel_requested {
                b.cancel_requested = true;
                if let Some(pid) = &b.pid {
                    backend::terminate(pid);
                }
            }
        }
    }



    fn start_check(&mut self) {
        if self.checking {
            return;
        }
        self.checking = true;
        self.start(TaskKind::Check, &["--check"]);
    }

    fn handle_check_line(&mut self, rest: &str) {
        let mut parts = rest.splitn(3, ' ');
        let what = parts.next().unwrap_or("");
        let status = parts.next().unwrap_or("");
        let detail = parts.next().unwrap_or("").trim().to_string();
        let value = match (what, status) {
            ("macos", _) => {
                self.prereqs.macos = status.to_string();
                return;
            }
            (_, "ok") | ("arch", "intel") => Check::Ok(detail),
            (_, "missing") => Check::Missing,
            ("disk", "low") => Check::Low(detail),
            ("arch", "other") => Check::Other,
            _ => Check::Unknown,
        };
        match what {
            "tools" => self.prereqs.tools = value,
            "rust" => self.prereqs.rust = value,
            "target" => self.prereqs.target = value,
            "disk" => self.prereqs.disk = value,
            "arch" => self.prereqs.arch = value,
            _ => {}
        }
    }

    fn handle_app_line(&mut self, rest: &str) {
        // Fields: name, tested (0 or 1, not shown in the window), description.
        let mut f = rest.splitn(3, '\t');
        let name = f.next().unwrap_or("").to_string();
        let _tested = f.next();
        let descr = f.next().unwrap_or("").to_string();
        if !name.is_empty() {
            self.apps.push(AppEntry { name, descr });
        }
    }

    fn drain_events(&mut self) {
        while let Ok(ev) = self.rx.try_recv() {
            match ev {
                Event::Line(kind, line) => {
                    let marker = parse_marker(&line);
                    match (kind, marker) {
                        (TaskKind::Check, Some(("CHECK", rest))) => self.handle_check_line(rest),
                        (TaskKind::ListApps, Some(("APP", rest))) => self.handle_app_line(rest),
                        (TaskKind::InstallRust, Some(("STEP", rest))) => self.last_line = rest.to_string(),
                        (TaskKind::InstallRust, None) if !line.trim().is_empty() => {
                            self.last_line = line.trim().to_string()
                        }
                        (TaskKind::InstallRust, Some(("ERROR", rest))) => self.last_line = rest.to_string(),
                        (TaskKind::Build, _) => {
                            if let Some(b) = self.build.as_mut() {
                                b.apply_line(&line);
                            }
                        }
                        (TaskKind::Info | TaskKind::Copy | TaskKind::Clean, _) => {
                            if let Some(r) = self.result.as_mut() {
                                r.apply_line(&line);
                            }
                        }
                        _ => {}
                    }
                }
                Event::Done(kind, _code) => match kind {
                    TaskKind::Check => {
                        self.checking = false;
                        if self.prereqs.tools.is_ok() {
                            self.tools_wait = false;
                        }
                    }
                    TaskKind::InstallRust => {
                        self.rust_busy = false;
                        self.start_check();
                    }
                    TaskKind::InstallTools => {
                        self.next_poll = Some(Instant::now() + Duration::from_secs(4));
                    }
                    TaskKind::ListApps => {}
                    TaskKind::Build => self.build_finished(),
                    TaskKind::Info => {}
                    TaskKind::Copy | TaskKind::Clean => {
                        if let Some(r) = self.result.as_mut() {
                            r.busy = None;
                        }
                    }
                },
            }
        }
    }

    fn build_finished(&mut self) {
        let Some(b) = self.build.as_mut() else { return };
        b.finish();
        match b.status {
            Status::Succeeded => {
                let app = b.app.clone();
                let path = b.done_path.clone().unwrap_or_default();
                self.result = Some(ResultState::new(&app, &path));
                self.start(TaskKind::Info, &["--machine", "--info", &app]);
                self.screen = Screen::Done;
            }
            Status::Cancelled => {
                self.cancelled_notice = true;
                self.screen = Screen::Select;
            }
            _ => {
                self.screen = Screen::Failed;
            }
        }
    }

    fn poll_tools(&mut self) {
        if !self.tools_wait {
            return;
        }
        if let Some(when) = self.next_poll {
            if Instant::now() >= when && !self.checking {
                self.next_poll = Some(Instant::now() + Duration::from_secs(4));
                self.start_check();
            }
            self.ctx.request_repaint_after(Duration::from_secs(1));
        }
    }

    fn requirements_ok(&self) -> bool {
        self.prereqs.tools.is_ok() && self.prereqs.rust.is_ok() && self.prereqs.target.is_ok()
    }
}

fn icon(ui: &mut egui::Ui, kind: Icon) {
    if let Icon::Busy = kind {
        ui.add(egui::Spinner::new().size(18.0));
        return;
    }
    let (rect, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
    let p = ui.painter();
    let c = rect.center();
    let white = Stroke::new(2.0, Color32::WHITE);
    match kind {
        Icon::Ok => {
            p.circle_filled(c, 9.0, GREEN);
            p.add(Shape::line(
                vec![c + egui::vec2(-4.5, 0.5), c + egui::vec2(-1.5, 3.5), c + egui::vec2(4.5, -3.5)],
                white,
            ));
        }
        Icon::Bad => {
            p.circle_filled(c, 9.0, RED);
            p.line_segment([c + egui::vec2(-3.5, -3.5), c + egui::vec2(3.5, 3.5)], white);
            p.line_segment([c + egui::vec2(-3.5, 3.5), c + egui::vec2(3.5, -3.5)], white);
        }
        Icon::Warn => {
            p.circle_filled(c, 9.0, ORANGE);
            p.line_segment([c + egui::vec2(0.0, -4.5), c + egui::vec2(0.0, 1.5)], white);
            p.circle_filled(c + egui::vec2(0.0, 4.5), 1.3, Color32::WHITE);
        }
        Icon::Unknown => {
            p.circle_stroke(c, 8.0, Stroke::new(1.5, Color32::GRAY));
        }
        Icon::Busy => {}
    }
}

impl Builder {
    /// One row of the requirements list. Returns true when its button was clicked.
    fn req_row(
        &self,
        ui: &mut egui::Ui,
        label: &str,
        state: &Check,
        busy: bool,
        detail: &str,
        button: Option<&str>,
        help: Option<&str>,
    ) -> bool {
        let mut clicked = false;
        let ic = if busy {
            Icon::Busy
        } else {
            match state {
                Check::Ok(_) => Icon::Ok,
                Check::Missing | Check::Other => Icon::Bad,
                Check::Low(_) => Icon::Warn,
                Check::Unknown => Icon::Unknown,
            }
        };
        ui.horizontal(|ui| {
            icon(ui, ic);
            ui.label(RichText::new(label).strong());
            if !detail.is_empty() {
                ui.label(RichText::new(detail).weak());
            }
            if let Some(text) = button {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    // Grey (disabled) when the requirement is already met or while it is running.
                    let enabled = !state.is_ok() && !busy && *state != Check::Unknown;
                    if ui.add_enabled(enabled, egui::Button::new(text)).clicked() {
                        clicked = true;
                    }
                });
            }
        });
        if let Some(h) = help {
            if !state.is_ok() && *state != Check::Unknown {
                ui.label(RichText::new(h).weak());
                ui.add_space(4.0);
            }
        }
        clicked
    }
}

impl Builder {
    fn ui_select(&mut self, ui: &mut egui::Ui) {
        let l = self.lang;
        if self.cancelled_notice {
            ui.colored_label(ORANGE, t(l, K::BuildCancelled));
        }
        // ---- requirements ----
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(RichText::new(t(l, K::Requirements)).heading());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add_enabled(!self.checking, egui::Button::new(t(l, K::CheckAgain)))
                    .clicked()
                {
                    self.start_check();
                }
                if self.checking {
                    ui.add(egui::Spinner::new().size(14.0));
                }
            });
        });
        ui.add_space(4.0);

        let tools = self.prereqs.tools.clone();
        let detail = match &tools {
            Check::Ok(v) => format!("SDK {v}"),
            _ => String::new(),
        };
        if self.req_row(
            ui,
            t(l, K::ReqTools),
            &tools,
            self.tools_wait,
            &if self.tools_wait { t(l, K::InstallingTools).to_string() } else { detail },
            Some(t(l, K::Install)),
            Some(t(l, K::ReqToolsHelp)),
        ) {
            self.tools_wait = true;
            self.next_poll = Some(Instant::now() + Duration::from_secs(4));
            self.start(TaskKind::InstallTools, &["--install-tools"]);
        }

        let rust = self.prereqs.rust.clone();
        let detail = match &rust {
            Check::Ok(v) => v.clone(),
            _ => String::new(),
        };
        let busy_text = if self.rust_busy { t(l, K::InstallingRust).to_string() } else { detail };
        if self.req_row(
            ui,
            t(l, K::ReqRust),
            &rust,
            self.rust_busy,
            &busy_text,
            Some(t(l, K::Install)),
            Some(t(l, K::ReqRustHelp)),
        ) {
            self.rust_busy = true;
            self.last_line.clear();
            self.start(TaskKind::InstallRust, &["--machine", "--install-rust"]);
        }

        let target = self.prereqs.target.clone();
        // The Intel target is added by the same action as Rust itself.
        if self.req_row(
            ui,
            t(l, K::ReqTarget),
            &target,
            self.rust_busy,
            "x86_64-apple-darwin",
            Some(t(l, K::Install)),
            None,
        ) {
            self.rust_busy = true;
            self.last_line.clear();
            self.start(TaskKind::InstallRust, &["--machine", "--install-rust"]);
        }
        if self.rust_busy && !self.last_line.is_empty() {
            ui.label(
                RichText::new(format!("{}: {}", t(l, K::LastLine), self.last_line))
                    .weak()
                    .small(),
            );
        }

        let disk = self.prereqs.disk.clone();
        let detail = match &disk {
            Check::Ok(g) | Check::Low(g) => format!("{g} {} {}", t(l, K::GigabyteShort), t(l, K::Free)),
            _ => String::new(),
        };
        self.req_row(ui, t(l, K::ReqDisk), &disk, false, &detail, None, Some(t(l, K::ReqDiskHelp)));

        let arch = self.prereqs.arch.clone();
        let detail = if self.prereqs.macos.is_empty() {
            String::new()
        } else {
            format!("{} {}", t(l, K::MacosVersion), self.prereqs.macos)
        };
        self.req_row(ui, t(l, K::ReqArch), &arch, false, &detail, None, Some(t(l, K::ReqArchHelp)));

        // ---- app list ----
        ui.add_space(8.0);
        ui.separator();
        ui.label(RichText::new(t(l, K::ChooseApp)).heading());
        ui.add_space(4.0);
        let list_height = (ui.available_height() - 8.0).max(120.0);
        // A scroll bar that takes its own space instead of floating over the last column.
        ui.spacing_mut().scroll.floating = false;
        egui::ScrollArea::vertical().max_height(list_height).auto_shrink([false, false]).show(ui, |ui| {
            for i in 0..self.apps.len() {
                let selected = self.selected == Some(i);
                let (name, descr) = {
                    let a = &self.apps[i];
                    (a.name.clone(), app_description(l, &a.name, &a.descr))
                };
                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(150.0, 24.0),
                        Layout::left_to_right(Align::Center),
                        |ui| {
                            ui.set_min_size(egui::vec2(150.0, 24.0));
                            if ui.selectable_label(selected, name.as_str()).clicked() {
                                self.selected = Some(i);
                            }
                        },
                    );
                    ui.label(descr);
                });
            }
        });
    }

    fn ui_building(&mut self, ui: &mut egui::Ui) {
        let l = self.lang;
        let Some(b) = self.build.as_ref() else { return };
        ui.separator();
        ui.label(RichText::new(i18n::building_title(l, &b.app)).heading());
        ui.label(i18n::elapsed(l, &format_duration(b.elapsed())));
        ui.label(RichText::new(t(l, K::WaitHint)).weak());
        ui.add_space(8.0);
        let running = b.status == Status::Running;
        let last = b.steps.len().saturating_sub(1);
        for (i, step) in b.steps.iter().enumerate() {
            ui.horizontal(|ui| {
                if i == last && running {
                    icon(ui, Icon::Busy);
                } else {
                    icon(ui, Icon::Ok);
                }
                ui.label(step);
            });
        }
        if running {
            let line: String = b.last_log_line().chars().take(110).collect();
            if !line.is_empty() {
                ui.add_space(4.0);
                ui.label(RichText::new(line).weak().monospace());
            }
            self.ctx.request_repaint_after(Duration::from_secs(1));
        }
        ui.add_space(8.0);
        self.log_view(ui);
    }

    fn log_view(&mut self, ui: &mut egui::Ui) {
        let l = self.lang;
        let Some(b) = self.build.as_ref() else { return };
        egui::CollapsingHeader::new(t(l, K::ShowLog))
            .default_open(self.show_log)
            .show(ui, |ui| {
                let row_h = ui.text_style_height(&egui::TextStyle::Monospace);
                egui::ScrollArea::vertical()
                    .max_height((ui.available_height() - 10.0).clamp(120.0, 300.0))
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show_rows(ui, row_h, b.log.len(), |ui, range| {
                        for i in range {
                            if let Some(line) = b.log.get(i) {
                                ui.label(RichText::new(line).monospace().small());
                            }
                        }
                    });
            });
    }

    fn ui_failed(&mut self, ui: &mut egui::Ui) {
        let l = self.lang;
        let Some(b) = self.build.as_ref() else { return };
        ui.separator();
        ui.horizontal(|ui| {
            icon(ui, Icon::Bad);
            ui.label(RichText::new(t(l, K::BuildFailedTitle)).heading());
        });
        ui.add_space(4.0);
        if let Some(e) = &b.error {
            ui.label(format!("{}: {}", t(l, K::ErrorPrefix), e));
        }
        let hint = match b.kind.as_deref() {
            Some("undefined-symbols") => K::HintUndefined,
            Some("framework-not-found") => K::HintFramework,
            Some("patch") => K::HintPatch,
            _ => K::HintGeneric,
        };
        ui.add_space(4.0);
        ui.label(t(l, hint));
        ui.add_space(8.0);
        let log_path = b.log_path.clone();
        ui.horizontal(|ui| {
            if ui.button(t(l, K::ReportIssue)).clicked() {
                backend::run_detached("/usr/bin/open", &[ISSUES_URL]);
            }
            if let Some(p) = &log_path {
                if ui.button(t(l, K::OpenLogInFinder)).clicked() {
                    backend::run_detached("/usr/bin/open", &["-R", p]);
                }
            }
        });
        ui.add_space(8.0);
        self.log_view(ui);
    }

    fn ui_done(&mut self, ui: &mut egui::Ui) {
        let l = self.lang;
        let comma = l == Lang::De;
        let mut copy = false;
        let mut delete = false;
        let Some(r) = self.result.as_mut() else { return };
        ui.separator();
        ui.horizontal(|ui| {
            icon(ui, Icon::Ok);
            ui.label(RichText::new(i18n::built_title(l, &r.app, &r.version)).heading());
        });
        if r.app_path.is_empty() {
            ui.label(t(l, K::NotAnApp));
            return;
        }
        // After "delete" the finished app in the build folder is gone as well.
        if r.cleaned_kb.is_none() {
            ui.label(RichText::new(&r.app_path).weak());
            ui.horizontal(|ui| {
                if ui.button(t(l, K::ShowInFinder)).clicked() {
                    backend::run_detached("/usr/bin/open", &["-R", &r.app_path]);
                }
                if ui.button(t(l, K::OpenApp)).clicked() {
                    backend::run_detached("/usr/bin/open", &[&r.app_path]);
                }
            });
        }

        // ---- copy to Applications ----
        ui.add_space(10.0);
        ui.separator();
        ui.label(RichText::new(t(l, K::InstallSection)).strong());
        if let Some(path) = &r.copied_to {
            ui.horizontal(|ui| {
                icon(ui, Icon::Ok);
                ui.label(i18n::copied_to(l, ""));
            });
            ui.label(RichText::new(path).weak());
            if ui.button(t(l, K::OpenApp)).clicked() {
                backend::run_detached("/usr/bin/open", &[path]);
            }
        } else {
            if r.installed_path.is_some() {
                ui.label(RichText::new(i18n::already_installed(l, &r.installed_version)).weak());
            }
            let label = if r.busy == Some(Busy::Copying) {
                t(l, K::CopyingNow)
            } else if r.installed_path.is_some() {
                t(l, K::ReplaceInApps)
            } else {
                t(l, K::CopyToApps)
            };
            if ui.add_enabled(r.busy.is_none(), egui::Button::new(label)).clicked() {
                copy = true;
            }
        }
        if let Some(e) = &r.error {
            ui.colored_label(RED, e);
        }

        // ---- clean up ----
        ui.add_space(10.0);
        ui.separator();
        ui.label(RichText::new(t(l, K::CleanupSection)).strong());
        let size = format_size(r.size_kb, comma);
        if let Some(kb) = r.cleaned_kb {
            ui.horizontal(|ui| {
                icon(ui, Icon::Ok);
                ui.label(i18n::deleted_note(l, &format_size(kb, comma)));
            });
        } else if r.kept {
            ui.label(t(l, K::KeptNote));
        } else {
            ui.label(RichText::new(i18n::cleanup_text(l, &size)).weak());
            let can_delete = r.copied_to.is_some() && r.busy.is_none();
            ui.horizontal(|ui| {
                if ui.add_enabled(r.busy.is_none(), egui::Button::new(t(l, K::KeepFiles))).clicked() {
                    r.kept = true;
                }
                let label = if r.busy == Some(Busy::Cleaning) {
                    t(l, K::DeletingNow).to_string()
                } else {
                    i18n::delete_button(l, &size)
                };
                if ui.add_enabled(can_delete, egui::Button::new(label)).clicked() {
                    delete = true;
                }
            });
            if r.copied_to.is_none() {
                ui.label(RichText::new(t(l, K::DeleteNeedsCopy)).weak());
            }
        }

        let app = r.app.clone();
        if copy {
            r.busy = Some(Busy::Copying);
        }
        if delete {
            r.busy = Some(Busy::Cleaning);
        }
        if copy {
            self.start(TaskKind::Copy, &["--machine", "--copy", &app]);
        }
        if delete {
            self.start(TaskKind::Clean, &["--machine", "--clean", &app]);
        }
    }
}

impl eframe::App for Builder {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.drain_events();
        self.poll_tools();
        let l = self.lang;

        // The bottom panel has to be added before the central one.
        if self.script.is_some() {
            egui::Panel::bottom("bottom").show(root, |ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| match self.screen {
                    Screen::Select => {
                        let ready = self.selected.is_some()
                            && self.requirements_ok()
                            && !self.rust_busy
                            && !self.tools_wait;
                        if ui.add_enabled(ready, egui::Button::new(t(l, K::Build))).clicked() {
                            if let Some(i) = self.selected {
                                let name = self.apps[i].name.clone();
                                self.start_build(&name);
                            }
                        }
                        let hint = if self.selected.is_none() {
                            t(l, K::PickFirst)
                        } else if !self.requirements_ok() {
                            t(l, K::NeedRequirements)
                        } else {
                            ""
                        };
                        ui.label(RichText::new(hint).weak());
                    }
                    Screen::Building => {
                        let cancelling = self.build.as_ref().map(|b| b.cancel_requested).unwrap_or(false);
                        let label = if cancelling { t(l, K::Cancelling) } else { t(l, K::Cancel) };
                        if ui.add_enabled(!cancelling, egui::Button::new(label)).clicked() {
                            self.cancel_build();
                        }
                    }
                    Screen::Done | Screen::Failed => {
                        let busy = self.result.as_ref().map(|r| r.busy.is_some()).unwrap_or(false);
                        if ui.add_enabled(!busy, egui::Button::new(t(l, K::BackToList))).clicked() {
                            self.screen = Screen::Select;
                            self.build = None;
                            self.result = None;
                            self.start_check();
                        }
                    }
                });
                ui.add_space(6.0);
            });
        }

        egui::CentralPanel::default().show(root, |ui| {
            ui.horizontal(|ui| {
                ui.heading(t(l, K::Title));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.selectable_value(&mut self.lang, Lang::De, "DE");
                    ui.selectable_value(&mut self.lang, Lang::En, "EN");
                });
            });
            ui.label(RichText::new(t(l, K::Subtitle)).weak());
            ui.add_space(8.0);

            if self.script.is_none() {
                ui.separator();
                ui.colored_label(RED, t(l, K::ScriptMissingTitle));
                ui.label(t(l, K::ScriptMissingText));
                return;
            }
            match self.screen {
                Screen::Select => self.ui_select(ui),
                Screen::Building => self.ui_building(ui),
                Screen::Failed => self.ui_failed(ui),
                Screen::Done => self.ui_done(ui),
            }
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Closing the window must not leave a build running in the background.
        self.cancel_build();
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Craftalina")
            .with_inner_size([780.0, 720.0])
            .with_min_inner_size([640.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Craftalina",
        options,
        Box::new(|cc| Ok(Box::new(Builder::new(&cc.egui_ctx)))),
    )
}
