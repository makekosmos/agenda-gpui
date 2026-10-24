use gpui::AsyncApp;

use crate::appearance::{AccentSource, Mode};

use super::Run;

pub(super) async fn run(run: &Run, cx: &mut AsyncApp) {
    let demo_counts = run.read(cx, |agenda| (agenda.todos.len(), agenda.projects.len()));
    run.check(
        &format!(
            "demo data loaded: {} tasks, {} projects",
            demo_counts.0, demo_counts.1
        ),
        demo_counts.0 > 0 && demo_counts.1 == 2,
    );
    for name in [
        "inbox",
        "week",
        "next-week",
        "someday",
        "logbook",
        "project",
        "statistics",
        "about",
        "trash",
    ] {
        if name == "inbox" {
            run.sleep(cx, 500).await;
            run.click_at(cx, 80.0, 95.0).await;
            run.click_at(cx, 80.0, 60.0).await;
            let ready = run
                .wait_until(cx, std::time::Duration::from_secs(3), |agenda| {
                    agenda.route == crate::app::Route::Inbox
                })
                .await;
            run.check("nsh inbox route restored after initial state sync", ready);
        } else {
            run.navigate_nsh(cx, name).await;
        }
        run.shot(cx, name).await;
        run.check(
            &format!("captured nsh {name}"),
            run.out.join(format!("{name}.png")).exists(),
        );
    }
    run.navigate_nsh(cx, "settings").await;
    run.shot(cx, "settings").await;
    run.check(
        "captured nsh settings",
        run.out.join("settings.png").exists(),
    );
    run.click_at(cx, 670.0, 160.0).await;
    let light = run.read(cx, |agenda| {
        agenda
            .appearance
            .as_ref()
            .is_some_and(|appearance| appearance.settings.mode == Mode::Light)
    });
    run.check("Settings changed appearance mode", light);
    run.shot(cx, "settings-theme").await;
    run.click_at(cx, 755.0, 160.0).await;
    run.click_at(cx, 578.0, 470.0).await;
    let accent = run.read(cx, |agenda| {
        agenda.appearance.as_ref().is_some_and(|appearance| {
            appearance.settings.accent_source == AccentSource::Custom
                && appearance.settings.accent_color.as_deref() == Some("#d79921")
        })
    });
    run.check("Settings changed custom accent", accent);
    run.shot(cx, "settings-accent").await;
    run.click_at(cx, 578.0, 647.0).await;
    let density = run.read(cx, |agenda| {
        agenda
            .appearance
            .as_ref()
            .is_some_and(|appearance| appearance.settings.font_size == 12.0)
    });
    run.check("Settings changed density", density);
    run.shot(cx, "settings-density").await;
    run.click_at(cx, 1200.0, 740.0).await;
    run.shot(cx, "settings-hotkeys").await;
}
