// Agenda GPUI — task data is owned by Mundus Engine.
#![windows_subsystem = "windows"]
mod app;
mod appearance;
mod assets;
mod brand;
mod chrome;
#[cfg(feature = "e2e")]
mod e2e;
mod pages;
mod palettes;
mod seed;
mod store;
mod text_field;
mod theme;
#[cfg(test)]
mod ui_tests;
mod widgets;

#[cfg(test)]
mod a11y_tests;
#[cfg(test)]
mod capture_tests;
#[cfg(test)]
mod input_tests;
#[cfg(test)]
mod kos216_tests;
#[cfg(test)]
mod kos232_tests;
#[cfg(test)]
mod kos244_tests;
#[cfg(test)]
mod kos254_tests;
#[cfg(test)]
mod kos295_tests;
#[cfg(test)]
mod regression_tests;
#[cfg(test)]
mod settings_a11y_tests;

use gpui::{px, size, App, AppContext, Bounds, SharedString, Styled, WindowBounds, WindowOptions};

/// `AGENDA_OFFSCREEN=1` parks the window far outside the desktop so automated
/// soak runs (FPS/scroll benchmarks) don't pop a window on the user's screen.
/// DWM still composes it, so frame pacing stays representative.
fn window_bounds(cx: &mut App) -> Bounds<gpui::Pixels> {
    if offscreen() {
        gpui::bounds(
            gpui::point(px(-20000.), px(-20000.)),
            size(px(1440.), px(900.)),
        )
    } else {
        Bounds::centered(None, size(px(1440.), px(900.)), cx)
    }
}

fn offscreen() -> bool {
    #[cfg(feature = "e2e")]
    if e2e::requested() {
        return true;
    }
    std::env::var("AGENDA_OFFSCREEN").is_ok()
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
        let created = std::rc::Rc::new(std::cell::RefCell::new(None));
        let slot = created.clone();
        let window = cx
            .open_window(
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
                        traffic_light_position: Some(gpui::point(px(14.), px(14.))),
                    }),
                    ..Default::default()
                },
                |window, cx| {
                    let agenda = cx.new(Agenda::new);
                    *slot.borrow_mut() = Some(agenda.clone());
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
        text_field::init_native_focus();
        #[cfg(feature = "e2e")]
        if e2e::requested() {
            let agenda = created.borrow_mut().take().expect("window builder ran");
            e2e::run(window.into(), agenda, cx);
        }
        #[cfg(not(feature = "e2e"))]
        let _ = (window, created);
        if !offscreen() {
            cx.activate(true);
        }
    });
}
