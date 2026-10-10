//! The scenarios `scripts/e2e-macos.sh` runs, in order, against one Engine.
use std::{
    path::Path,
    time::{Duration, Instant},
};

use gpui::AsyncApp;

use super::{Run, NOTE, PROJECT, RAPID, TITLES};
use crate::app::{MenuAction, Route};
use agenda_core::{is_due_this_week, is_inbox, LocalDay};

pub(super) async fn create(run: &Run, cx: &mut AsyncApp) {
    if !run.ready(cx).await {
        run.shot(cx, "00-not-ready").await;
        return;
    }
    run.shot(cx, "01-start").await;

    run.navigate(cx, Route::Inbox).await;
    run.press(cx, "cmd-n").await;
    run.shot(cx, "02-quick-entry").await;
    run.press(cx, "escape").await;

    run.quick_add(cx, TITLES[0], None).await;
    let saved = run
        .wait_until(cx, Duration::from_secs(5), |a| {
            Run::count(a, TITLES[0]) == 1 && !a.storage_busy
        })
        .await;
    run.shot(cx, "03-inbox-created").await;
    run.check("Inbox: task created once", saved);
    run.check(
        "Inbox: no error banner",
        run.read(cx, |a| a.storage_error.is_none()),
    );

    run.navigate(cx, Route::Week).await;
    run.quick_add(cx, TITLES[1], Some(NOTE)).await;
    let saved = run
        .wait_until(cx, Duration::from_secs(5), |a| {
            Run::count(a, TITLES[1]) == 1
        })
        .await;
    run.shot(cx, "04-week-created").await;
    run.check("Week: task created once", saved);
    run.check(
        "Week: task is listed in this week",
        run.read(cx, |a| {
            a.todos.iter().any(|t| {
                t.title == TITLES[1]
                    && is_due_this_week(
                        t,
                        &LocalDay::now().week_bounds().unwrap(),
                        &LocalDay::now(),
                    )
            })
        }),
    );
    run.check(
        "Week: task keeps its note",
        run.read(cx, |a| {
            a.todos
                .iter()
                .any(|t| t.title == TITLES[1] && t.notes.as_deref() == Some(NOTE))
        }),
    );

    // A new project opens with its heading selected: typing names it.
    run.update(cx, |a| a.create_project());
    run.shot(cx, "05a-project-new").await;
    run.type_text(cx, PROJECT).await;
    run.press(cx, "enter").await;
    let named = run
        .wait_until(cx, Duration::from_secs(5), |a| {
            a.projects.iter().any(|p| p.title == PROJECT) && !a.storage_busy
        })
        .await;
    run.shot(cx, "05b-project-named").await;
    run.check("Project: named by typing into the heading", named);
    run.quick_add(cx, TITLES[2], None).await;
    let in_project = run
        .wait_until(cx, Duration::from_secs(5), |a| {
            a.todos
                .iter()
                .any(|t| t.title == TITLES[2] && t.project_id.is_some())
        })
        .await;
    let project_saved = run
        .wait_until(cx, Duration::from_secs(5), |a| !a.storage_busy)
        .await
        && run.read(cx, |a| a.storage_error.is_none());
    run.shot(cx, "05-project-created").await;
    run.check("Project: task created inside the project", in_project);
    run.check("Project: saved without an error", project_saved);

    run.navigate(cx, Route::Inbox).await;
    for i in 1..=RAPID {
        run.quick_add(cx, &format!("E2E быстро {i}"), None).await;
    }
    let all = run
        .wait_until(cx, Duration::from_secs(8), |a| {
            (1..=RAPID).all(|i| Run::count(a, &format!("E2E быстро {i}")) == 1) && !a.storage_busy
        })
        .await;
    run.shot(cx, "06-rapid-five").await;
    run.check(
        "Rapid: tasks land in Inbox, not in Week",
        run.read(cx, |a| {
            a.todos
                .iter()
                .filter(|t| t.title.starts_with("E2E быстро"))
                .all(|t| !t.is_today && t.project_id.is_none())
        }),
    );
    run.check("Rapid: 5 tasks, none lost or doubled", all);
    run.check(
        "Rapid: no error banner",
        run.read(cx, |a| a.storage_error.is_none()),
    );
}

pub(super) async fn persist(run: &Run, cx: &mut AsyncApp) {
    if !run.ready(cx).await {
        return;
    }
    run.navigate(cx, Route::Inbox).await;
    run.shot(cx, "07-after-restart-inbox").await;
    for title in TITLES {
        run.check(
            &format!("Restart: '{title}' still there"),
            run.read(cx, |a| Run::count(a, title) == 1),
        );
    }
    run.check(
        "Restart: note survived",
        run.read(cx, |a| {
            a.todos
                .iter()
                .any(|t| t.title == TITLES[1] && t.notes.as_deref() == Some(NOTE))
        }),
    );
    run.check(
        "Restart: project survived with its name",
        run.read(cx, |a| {
            a.projects.len() == 1 && a.projects[0].title == PROJECT
        }),
    );
    run.check(
        "Restart: week task still in this week",
        run.read(cx, |a| {
            a.todos.iter().any(|t| {
                t.title == TITLES[1]
                    && is_due_this_week(
                        t,
                        &LocalDay::now().week_bounds().unwrap(),
                        &LocalDay::now(),
                    )
            })
        }),
    );
    run.check(
        "Restart: rapid tasks survived",
        run.read(cx, |a| {
            (1..=RAPID).all(|i| Run::count(a, &format!("E2E быстро {i}")) == 1)
        }),
    );

    let Some(pid) = run.read(cx, |a| a.projects.first().map(|p| p.id.clone())) else {
        run.check("Delete: a project to delete exists", false);
        return;
    };
    run.update(cx, |a| a.run_menu_action(MenuAction::DeleteProject(pid)));
    let deleted = run
        .wait_until(cx, Duration::from_secs(5), |a| {
            a.projects.is_empty() && !a.storage_busy && a.storage_error.is_none()
        })
        .await;
    run.shot(cx, "07b-project-deleted").await;
    run.check("Delete: project removed without an error", deleted);
    run.check(
        "Delete: its task moved to Inbox",
        run.read(cx, |a| {
            a.todos
                .iter()
                .any(|t| t.title == TITLES[2] && t.project_id.is_none() && is_inbox(t))
        }),
    );
}

/// Marker handshake with the runner: we ask it to stop or start Engine and
/// wait until it confirms.
async fn handshake(run: &Run, cx: &mut AsyncApp, ask: &str, done: &str) -> bool {
    std::fs::write(run.out.join(ask), b"").ok();
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(60) {
        if Path::new(&run.out.join(done)).exists() {
            return true;
        }
        run.sleep(cx, 200).await;
    }
    false
}

pub(super) async fn offline(run: &Run, cx: &mut AsyncApp) {
    if !run.ready(cx).await {
        return;
    }
    run.check(
        "Restart: deleted project stays deleted",
        run.read(cx, |a| {
            a.projects.is_empty() && Run::count(a, TITLES[2]) == 1
        }),
    );
    run.navigate(cx, Route::Inbox).await;
    if !handshake(run, cx, "stop-engine", "engine-stopped").await {
        run.check("runner stopped Engine", false);
        return;
    }
    run.quick_add(cx, "E2E без движка", None).await;
    let banner = run
        .wait_until(cx, Duration::from_secs(20), |a| a.storage_error.is_some())
        .await;
    run.shot(cx, "08-offline-banner").await;
    run.check("Offline: error message shown", banner);
    let text = run.read(cx, |a| a.storage_error.as_ref().map(|e| e.text.clone()));
    run.check(
        &format!("Offline: message is human-readable ({text:?})"),
        text.as_deref()
            .is_some_and(|t| !t.contains("Engine отклонил операцию")),
    );
    let hidden = run
        .wait_until(cx, Duration::from_secs(12), |a| a.storage_error.is_none())
        .await;
    run.shot(cx, "09-offline-banner-gone").await;
    run.check("Offline: message hides by itself", hidden);

    if !handshake(run, cx, "start-engine", "engine-started").await {
        run.check("runner restarted Engine", false);
        return;
    }
    run.quick_add(cx, "E2E после движка", None).await;
    let saved = run
        .wait_until(cx, Duration::from_secs(20), |a| {
            Run::count(a, "E2E после движка") == 1 && a.storage_error.is_none() && !a.storage_busy
        })
        .await;
    run.shot(cx, "10-after-engine-back").await;
    run.check("Offline: saving works again once Engine is back", saved);
}
