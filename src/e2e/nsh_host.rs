use gpui::{
    AppContext as _, AsyncApp, Keystroke, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent,
    PlatformInput,
};

use crate::app::Route;

use super::Run;

impl Run {
    /// Dispatches a key and draws the offscreen window immediately.
    pub(super) fn keystroke(&self, cx: &mut AsyncApp, keystroke: Keystroke) {
        cx.update_window(self.window, |_, window, cx| {
            window.dispatch_keystroke(keystroke, cx);
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .expect("window alive");
    }

    pub(super) async fn click_at(&self, cx: &mut AsyncApp, x: f32, y: f32) {
        cx.update_window(self.window, |_, window, cx| {
            let position = gpui::point(gpui::px(x), gpui::px(y));
            let modifiers = Modifiers::default();
            window.dispatch_event(
                PlatformInput::MouseDown(MouseDownEvent {
                    position,
                    modifiers,
                    button: MouseButton::Left,
                    click_count: 1,
                    first_mouse: false,
                }),
                cx,
            );
            window.dispatch_event(
                PlatformInput::MouseUp(MouseUpEvent {
                    position,
                    modifiers,
                    button: MouseButton::Left,
                    click_count: 1,
                }),
                cx,
            );
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .expect("window alive");
        self.sleep(cx, 250).await;
        cx.update_window(self.window, |_, window, cx| {
            for _ in 0..3 {
                window.refresh();
                window.draw(cx).clear(cx);
            }
        })
        .expect("window alive");
        self.sleep(cx, 100).await;
    }

    pub(super) async fn navigate_nsh(&self, cx: &mut AsyncApp, name: &str) {
        let position = match name {
            "inbox" => (80.0, 60.0),
            "week" => (80.0, 95.0),
            "next-week" => (100.0, 129.0),
            "someday" => (80.0, 163.0),
            "project" => (100.0, 258.0),
            "logbook" | "statistics" | "settings" | "about" | "trash" => {
                if name == "logbook" {
                    self.click_at(cx, 60.0, 1070.0).await;
                }
                let y = match name {
                    "logbook" => 899.0,
                    "statistics" => 931.0,
                    "settings" => 963.0,
                    "about" => 995.0,
                    _ => 1027.0,
                };
                (100.0, y)
            }
            _ => panic!("unknown nsh route {name}"),
        };
        self.click_at(cx, position.0, position.1).await;
        let expected = match name {
            "inbox" => Route::Inbox,
            "week" => Route::Week,
            "next-week" => Route::NextWeek,
            "someday" => Route::Someday,
            "project" => Route::Project("dev-proj-release".into()),
            "logbook" => Route::Logbook,
            "statistics" => Route::Statistics,
            "settings" => Route::Settings,
            "about" => Route::About,
            "trash" => Route::Trash,
            _ => unreachable!(),
        };
        let navigated = self
            .wait_until(cx, std::time::Duration::from_secs(3), |agenda| {
                agenda.route == expected
            })
            .await;
        self.check(&format!("nsh navigation reached {name}"), navigated);
    }
}
