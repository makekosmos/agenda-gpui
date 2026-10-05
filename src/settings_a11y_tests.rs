use crate::a11y_tests::{a11y_tree, launch, nodes};
use gpui::TestAppContext;

#[gpui::test]
async fn settings_pages_have_named_h1(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    for (route, title) in [
        (crate::app::Route::Settings, "Отображение"),
        (crate::app::Route::SettingsFuel, "Мыслетопливо"),
        (crate::app::Route::SettingsEnergy, "Энергосбережение"),
    ] {
        agenda.update(cx, |app, cx| {
            app.route = route;
            cx.notify();
        });
        let tree = a11y_tree(cx);
        assert!(
            nodes(&tree)
                .iter()
                .any(|(role, node)| { role == "Heading" && node["aria"]["label"] == title }),
            "missing H1 for {title}: {tree}"
        );
    }
}

#[gpui::test]
async fn appearance_mode_tiles_are_accessible(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    agenda.update(cx, |app, cx| {
        app.route = crate::app::Route::Settings;
        cx.notify();
    });
    let tree = a11y_tree(cx);
    for label in ["Системная", "Светлая", "Тёмная"] {
        let name = format!("Режим темы: {label}");
        assert!(
            nodes(&tree)
                .iter()
                .any(|(role, node)| { role == "Button" && node["aria"]["label"] == name }),
            "missing mode button {name}"
        );
    }
}

#[gpui::test]
async fn about_page_uses_shared_product_hero_and_diagnostics(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    agenda.update(cx, |app, cx| {
        app.route = crate::app::Route::About;
        cx.notify();
    });
    let tree = a11y_tree(cx);
    assert!(nodes(&tree)
        .iter()
        .any(|(role, node)| role == "Heading" && node["aria"]["label"] == "Agenda"));
    assert!(cx.debug_bounds("about-hero").is_some());
    assert!(cx.debug_bounds("about-diagnostics-card").is_some());
}

#[gpui::test]
async fn energy_page_uses_shared_named_switch(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    agenda.update(cx, |app, cx| {
        app.route = crate::app::Route::SettingsEnergy;
        cx.notify();
    });
    let tree = a11y_tree(cx);
    assert!(
        nodes(&tree).iter().any(|(role, node)| {
            role == "Switch" && node["aria"]["label"] == "Вертикальная синхронизация (VSync)"
        }),
        "missing shared VSync switch: {tree}"
    );
}
