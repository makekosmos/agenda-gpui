// Agenda GPUI clone — experimental, KOS-124. Seeds local state only.
mod app;
mod assets;
mod chrome;
mod model;
mod pages;
mod theme;
mod widgets;

use gpui::{
    px, size, App, AppContext, Application, Bounds, Context, SharedString, Window, WindowBounds,
    WindowOptions,
};

use app::Agenda;
use assets::{font_bytes, Assets};

fn main() {
    Application::new().with_assets(Assets).run(|cx: &mut App| {
        gpui_component::init(cx);
        cx.text_system()
            .add_fonts(font_bytes())
            .expect("failed to load embedded fonts");

        let bounds = Bounds::centered(None, size(px(1440.), px(900.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some(SharedString::from("Agenda")),
                    appears_transparent: true,
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(|cx| Agenda::new(cx));
                // gpui-component widgets (Input, menus) require a ui::Root window layer.
                cx.new(|cx| gpui_component::Root::new(view, window, cx))
            },
        )
        .unwrap();
        cx.activate(true);
    });
}

// Silence unused warnings for Context/Window in docs-only signature helpers.
#[allow(dead_code)]
fn _sig(_: &mut Window, _: &mut Context<Agenda>) {}
