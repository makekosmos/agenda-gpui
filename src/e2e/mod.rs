//! Scripted end-to-end runs against a live Mundus Engine (`--features e2e`).
//!
//! `AGENDA_E2E=<scenario>` drives the real, offscreen Agenda window with real
//! keystrokes, saves a PNG of the rendered frame after every step and writes
//! `report.txt` (one `PASS`/`FAIL` line per check) into `AGENDA_E2E_OUT`.
//! `scripts/e2e-macos.sh` starts a throwaway Engine and runs the scenarios in
//! order; the offline scenario hands Engine control back to it through
//! marker files in the same directory.

use std::{
    cell::RefCell,
    fmt::Write as _,
    path::PathBuf,
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::{AnyWindowHandle, App, AppContext as _, AsyncApp, Entity, Keystroke, Modifiers};

use crate::app::{Agenda, Route};

mod scenarios;
use scenarios::{create, offline, persist};

/// Titles the create scenario writes; the persist scenario reads them back.
const TITLES: [&str; 3] = [
    "E2E купить молоко",
    "E2E позвонить маме",
    "E2E задача проекта",
];
const NOTE: &str = "E2E заметка к задаче";
const RAPID: usize = 5;
const PROJECT: &str = "E2E Ремонт";

pub fn requested() -> bool {
    std::env::var("AGENDA_E2E").is_ok()
}

pub fn run(window: AnyWindowHandle, agenda: Entity<Agenda>, cx: &mut App) {
    let scenario = std::env::var("AGENDA_E2E").unwrap_or_default();
    let out = PathBuf::from(std::env::var("AGENDA_E2E_OUT").unwrap_or_else(|_| "e2e-out".into()));
    std::fs::create_dir_all(&out).expect("create AGENDA_E2E_OUT");
    let run = Run {
        window,
        agenda,
        out,
        report: Rc::new(RefCell::new(String::new())),
        failed: Rc::new(RefCell::new(false)),
    };
    eprintln!("[e2e] scenario {scenario} → {}", run.out.display());
    cx.spawn(async move |cx| {
        match scenario.as_str() {
            "create" => create(&run, cx).await,
            "persist" => persist(&run, cx).await,
            "offline" => offline(&run, cx).await,
            other => run.check(&format!("known scenario '{other}'"), false),
        }
        run.finish();
        cx.update(|cx| cx.quit());
    })
    .detach();
}

struct Run {
    window: AnyWindowHandle,
    agenda: Entity<Agenda>,
    out: PathBuf,
    report: Rc<RefCell<String>>,
    failed: Rc<RefCell<bool>>,
}

impl Run {
    fn check(&self, name: &str, ok: bool) {
        let _ = writeln!(
            self.report.borrow_mut(),
            "{} {name}",
            if ok { "PASS" } else { "FAIL" }
        );
        if !ok {
            *self.failed.borrow_mut() = true;
        }
    }

    fn finish(&self) {
        let report = self.report.borrow();
        std::fs::write(self.out.join("report.txt"), report.as_bytes()).expect("write report");
        print!("{report}");
        if *self.failed.borrow() {
            std::fs::write(self.out.join("FAILED"), b"").ok();
        }
    }

    async fn sleep(&self, cx: &mut AsyncApp, ms: u64) {
        cx.background_executor()
            .timer(Duration::from_millis(ms))
            .await;
    }

    fn read<R>(&self, cx: &mut AsyncApp, f: impl FnOnce(&Agenda) -> R) -> R {
        cx.update(|cx| f(self.agenda.read(cx)))
    }

    fn update(&self, cx: &mut AsyncApp, f: impl FnOnce(&mut Agenda)) {
        cx.update(|cx| {
            self.agenda.update(cx, |agenda, cx| {
                f(agenda);
                cx.notify();
            })
        });
    }

    /// Polls `done` until it holds or `timeout` passes; returns whether it held.
    async fn wait_until(
        &self,
        cx: &mut AsyncApp,
        timeout: Duration,
        done: impl Fn(&Agenda) -> bool,
    ) -> bool {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if self.read(cx, &done) {
                return true;
            }
            self.sleep(cx, 100).await;
        }
        self.read(cx, &done)
    }

    /// Dispatches one keystroke and draws a frame. An onscreen window repaints
    /// on the next vsync; macOS stops driving an offscreen one, so we ask for
    /// the frame that a user's display would have produced.
    fn keystroke(&self, cx: &mut AsyncApp, keystroke: Keystroke) {
        cx.update_window(self.window, |_, window, cx| {
            window.dispatch_keystroke(keystroke, cx);
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .expect("window alive");
    }

    async fn press(&self, cx: &mut AsyncApp, key: &str) {
        self.keystroke(cx, Keystroke::parse(key).expect("valid keystroke"));
        self.sleep(cx, 150).await;
    }

    async fn type_text(&self, cx: &mut AsyncApp, text: &str) {
        for ch in text.chars() {
            self.keystroke(
                cx,
                Keystroke {
                    modifiers: Modifiers::default(),
                    key: ch.to_lowercase().to_string(),
                    key_char: Some(ch.to_string()),
                },
            );
        }
        self.sleep(cx, 150).await;
    }

    /// Lets the window paint the current state, then saves it as `<name>.png`.
    async fn shot(&self, cx: &mut AsyncApp, name: &str) {
        self.sleep(cx, 300).await;
        let image = cx
            .update_window(self.window, |_, window, cx| {
                window.refresh();
                window.draw(cx).clear(cx);
                window.render_to_image()
            })
            .expect("window alive");
        match image {
            Ok(image) => {
                image
                    .save(self.out.join(format!("{name}.png")))
                    .expect("save png");
            }
            Err(e) => self.check(&format!("screenshot {name}: {e}"), false),
        }
    }

    async fn navigate(&self, cx: &mut AsyncApp, route: Route) {
        self.update(cx, |agenda| agenda.navigate(route));
        self.sleep(cx, 200).await;
    }

    /// Opens quick entry with Cmd+N, types the title (and an optional note on
    /// the notes field via Tab) and submits with Enter, or Cmd+Enter from notes.
    async fn quick_add(&self, cx: &mut AsyncApp, title: &str, note: Option<&str>) {
        self.press(cx, "cmd-n").await;
        self.type_text(cx, title).await;
        if let Some(note) = note {
            self.press(cx, "tab").await;
            self.type_text(cx, note).await;
            self.press(cx, "cmd-enter").await;
        } else {
            self.press(cx, "enter").await;
        }
    }

    fn count(agenda: &Agenda, title: &str) -> usize {
        agenda.todos.iter().filter(|t| t.title == title).count()
    }

    async fn ready(&self, cx: &mut AsyncApp) -> bool {
        let ready = self
            .wait_until(cx, Duration::from_secs(15), |a| {
                a.storage_ready && !a.storage_busy
            })
            .await;
        self.check("Agenda loaded from Engine", ready);
        ready
    }
}
