use std::{
    cell::RefCell,
    fmt::Write as _,
    path::PathBuf,
    rc::Rc,
    time::{Duration, Instant},
};

use crate::app::Agenda;
use gpui::{AnyWindowHandle, App, AsyncApp, Entity};

mod capture;
mod nsh_host;
mod nsh_stage3;

pub fn requested() -> bool {
    std::env::var("AGENDA_E2E").is_ok()
}

pub fn run(window: AnyWindowHandle, agenda: Entity<Agenda>, cx: &mut App) {
    let out = PathBuf::from(std::env::var("AGENDA_E2E_OUT").unwrap_or_else(|_| "e2e-out".into()));
    std::fs::create_dir_all(&out).expect("create AGENDA_E2E_OUT");
    let run = Run {
        window,
        agenda,
        out,
        report: Rc::new(RefCell::new(String::new())),
        failed: Rc::new(RefCell::new(false)),
    };
    cx.spawn(async move |cx| {
        if std::env::var("AGENDA_E2E").as_deref() == Ok("nsh-stage3") {
            run.nsh_stage3(cx).await;
        } else {
            run.shot(cx, "inbox").await;
        }
        let failed = run.finish();
        cx.update(|cx| cx.quit());
        if failed {
            std::process::exit(1);
        }
    })
    .detach();
}

pub(super) struct Run {
    window: AnyWindowHandle,
    agenda: Entity<Agenda>,
    out: PathBuf,
    report: Rc<RefCell<String>>,
    failed: Rc<RefCell<bool>>,
}

impl Run {
    pub(super) fn check(&self, name: &str, ok: bool) {
        let _ = writeln!(
            self.report.borrow_mut(),
            "{} {name}",
            if ok { "PASS" } else { "FAIL" }
        );
        if !ok {
            *self.failed.borrow_mut() = true;
        }
    }

    pub(super) fn read<R>(&self, cx: &mut AsyncApp, f: impl FnOnce(&Agenda) -> R) -> R {
        cx.update(|cx| f(self.agenda.read(cx)))
    }

    pub(super) async fn sleep(&self, cx: &mut AsyncApp, ms: u64) {
        cx.background_executor()
            .timer(Duration::from_millis(ms))
            .await;
    }

    pub(super) async fn wait_until(
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
        self.read(cx, done)
    }

    pub(super) async fn nsh_stage3(&self, cx: &mut AsyncApp) {
        nsh_stage3::run(self, cx).await;
    }

    fn finish(&self) -> bool {
        let report = self.report.borrow();
        std::fs::write(self.out.join("report.txt"), report.as_bytes()).expect("write report");
        print!("{report}");
        *self.failed.borrow()
    }
}
