//! Window contract shared with Manager (KOS-368). The GPUI test platform has
//! no AppKit, so this cannot click the traffic lights. It does check the
//! startup path those buttons depend on: no title until the window exists,
//! exactly one window, and a platform close that is allowed to proceed.

use gpui::{TestAppContext, VisualTestContext};

#[gpui::test]
fn startup_defers_title_and_platform_close_is_allowed(cx: &mut TestAppContext) {
    std::env::set_var("AGENDA_DEMO", "1");
    std::env::remove_var("AGENDA_DEVTASKS");
    cx.update(gpui_component::init);
    let window = cx.update(|cx| {
        let options = super::agenda_window_options(cx);
        assert!(
            options
                .titlebar
                .as_ref()
                .and_then(|titlebar| titlebar.title.as_ref())
                .is_none(),
            "a creation-time title reintroduces the phantom Dock entry"
        );
        let (window, _) = super::open_agenda_window(cx);
        assert_eq!(cx.windows().len(), 1, "expected one window");
        window
    });
    let mut window_cx = VisualTestContext::from_window(window, cx);
    assert_eq!(window_cx.window_title().as_deref(), Some("Agenda"));
    window_cx.update(|_, cx| cx.refresh_windows());
    window_cx.run_until_parked();
    assert!(
        window_cx.simulate_close(),
        "platform close must be allowed when nothing is mid-write"
    );
}
