// Agenda GPUI clone — experimental, KOS-124. Seeds local state only.
#![windows_subsystem = "windows"]
mod app;
mod assets;
mod chrome;
mod model;
mod pages;
mod palettes;
mod theme;
mod widgets;

use gpui::{
    px, size, App, AppContext, Bounds, Context, SharedString, Styled, Window, WindowBounds,
    WindowOptions,
};

/// `AGENDA_OFFSCREEN=1` parks the window far outside the desktop so automated
/// soak runs (FPS/scroll benchmarks) don't pop a window on the user's screen.
/// DWM still composes it, so frame pacing stays representative.
fn window_bounds(cx: &mut App) -> Bounds<gpui::Pixels> {
    if std::env::var("AGENDA_OFFSCREEN").is_ok() {
        gpui::bounds(
            gpui::point(px(-20000.), px(-20000.)),
            size(px(1440.), px(900.)),
        )
    } else {
        Bounds::centered(None, size(px(1440.), px(900.)), cx)
    }
}

use app::Agenda;
use assets::{font_bytes, Assets};

fn main() {
    gpui::application().with_assets(Assets).run(|cx: &mut App| {
        gpui_component::init(cx);
        cx.text_system()
            .add_fonts(font_bytes())
            .expect("failed to load embedded fonts");

        let bounds = window_bounds(cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                // Windows adaptive pacing owns the background cap and input wakeup.
                // A second focus-only cap would throttle unfocused interaction.
                inactive_frame_interval: if cfg!(target_os = "windows")
                    || std::env::var("AGENDA_FPS").is_ok()
                {
                    None
                } else {
                    Some(std::time::Duration::from_micros(33_333))
                },
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some(SharedString::from("Agenda")),
                    appears_transparent: true,
                    // macOS traffic-light buttons sit inside the sidebar top strip.
                    traffic_light_position: Some(gpui::point(px(12.), px(14.))),
                }),
                ..Default::default()
            },
            |window, cx| {
                let agenda = cx.new(Agenda::new);
                // Shell keeps the FPS meter outside Agenda's subtree; the
                // sidebar and content own their independent caches.
                let view = cx.new(|_| app::AgendaShell { agenda });
                // gpui-component widgets (Input, menus) require a ui::Root window layer.
                let root = cx.new(|cx| gpui_component::Root::new(view, window, cx));
                // Root paints an opaque theme background over the whole window;
                // Agenda's own root owns the window background so the sidebar
                // acrylic/mica material can show through.
                root.update(cx, |root, _| {
                    root.style().background = Some(gpui::transparent_black().into());
                });
                root
            },
        )
        .unwrap();
        if std::env::var("AGENDA_OFFSCREEN").is_err() {
            cx.activate(true);
        }
    });
}

// Silence unused warnings for Context/Window in docs-only signature helpers.
#[allow(dead_code)]
fn _sig(_: &mut Window, _: &mut Context<Agenda>) {}
