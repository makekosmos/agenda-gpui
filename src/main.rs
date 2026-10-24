// Agenda GPUI — task data is owned by Mundus Engine.
#![windows_subsystem = "windows"]
#[cfg(target_os = "macos")]
#[macro_use]
extern crate objc;
mod app;
mod appearance;
mod assets;
mod brand;
mod chrome;
#[cfg(feature = "e2e")]
mod e2e;
#[cfg(target_os = "macos")]
mod macos_close;
mod model;
#[cfg(any(target_os = "macos", test))]
mod native_menu;
mod nsh_bridge;
mod pages;
mod palettes;
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
#[cfg(test)]
mod window_tests;

use gpui::{px, size, App, AppContext, Bounds, Styled, WindowBounds, WindowOptions};
#[cfg(target_os = "windows")]
use gpui_fast_backend as _;
use imago_nsh::{Bundle, NshApp};

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

const WINDOW_TITLE: &str = "Agenda";

fn agenda_window_options(cx: &mut App) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(window_bounds(cx))),
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
            // Same as Manager (KOS-368): a title at creation goes through
            // changeWindowsItem: before AppKit tracks the window, so the
            // Dock gets a phantom entry and the real window's close,
            // minimize and zoom stop being the system's buttons.
            title: None,
            appears_transparent: true,
            traffic_light_position: Some(gpui::point(px(14.), px(14.))),
        }),
        ..Default::default()
    }
}

fn open_agenda_window(cx: &mut App) -> (gpui::AnyWindowHandle, gpui::Entity<Agenda>) {
    let created = std::rc::Rc::new(std::cell::RefCell::new(None));
    let slot = created.clone();
    let options = agenda_window_options(cx);
    let window = cx
        .open_window(options, |window, cx| {
            // The platform window is already ordered in when this closure runs,
            // so set_title only renames the entry AppKit added itself.
            window.set_window_title(WINDOW_TITLE);
            let agenda = cx.new(Agenda::new);
            *slot.borrow_mut() = Some(agenda.clone());
            // Shell keeps the FPS meter outside Agenda's subtree; the
            // sidebar and content own their independent caches.
            let nsh = cx.new(|cx| {
                let initial = serde_json::to_string(&nsh_bridge::initial_snapshot(agenda.read(cx)))
                    .expect("serialize initial Agenda state");
                let source = format!(
                    "globalThis.__MUNDUS_AGENDA_INITIAL__={initial};\n{}",
                    include_str!("../ui/bundle.js")
                );
                imago_nsh::mount_app_view(window, cx, NshApp::Js(Bundle::source(source)), |view| {
                    view.register_component(
                        "agenda.bridge",
                        std::rc::Rc::new({
                            let agenda = agenda.clone();
                            move |context| {
                                Box::new(nsh_bridge::AgendaBridge::new(context, agenda.clone()))
                            }
                        }),
                    );
                    view.on_error(|message| eprintln!("Agenda nsh: {message}"));
                })
            });
            let view = cx.new(|_| app::AgendaShell {
                agenda,
                nsh: Some(nsh),
                close_handler_installed: false,
            });
            // gpui-component widgets (Input, menus) require a ui::Root window layer.
            let root = cx.new(|cx| gpui_component::Root::new(view, window, cx));
            // Root paints an opaque theme background over the whole window;
            // Agenda's own root owns the window background so the sidebar
            // acrylic/mica material can show through.
            root.update(cx, |root, _| {
                root.style().background = Some(gpui::transparent_black().into());
            });
            root
        })
        .unwrap();
    let agenda = created.borrow_mut().take().expect("window builder ran");
    (window.into(), agenda)
}

fn main() {
    #[cfg(target_os = "macos")]
    macos_close::install();
    gpui::application().with_assets(Assets).run(|cx: &mut App| {
        gpui_component::init(cx);
        cx.text_system()
            .add_fonts(font_bytes())
            .expect("failed to load embedded fonts");

        let (window, agenda) = open_agenda_window(cx);
        text_field::init_native_focus();
        #[cfg(target_os = "macos")]
        native_menu::install(cx);
        #[cfg(feature = "e2e")]
        if e2e::requested() {
            e2e::run(window, agenda, cx);
        }
        #[cfg(not(feature = "e2e"))]
        let _ = (window, agenda);
        if !offscreen() {
            cx.activate(true);
        }
    });
}
