//! Native macOS application menu. Command-Q and the Quit item exit Agenda.
//! Closing the last window also quits (`QuitMode::LastWindowClosed`).

use gpui::{actions, App, KeyBinding, Menu, MenuItem, SystemMenuType};

actions!(agenda_menu, [Quit, Hide, HideOthers, ShowAll]);

pub fn install(cx: &mut App) {
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.on_action(|_: &Hide, cx| cx.hide());
    cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
    cx.bind_keys([
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-h", Hide, None),
        KeyBinding::new("cmd-alt-h", HideOthers, None),
    ]);
    cx.set_menus([Menu::new("Agenda").items([
        MenuItem::os_submenu("Службы", SystemMenuType::Services),
        MenuItem::separator(),
        MenuItem::action("Скрыть Agenda", Hide),
        MenuItem::action("Скрыть остальные", HideOthers),
        MenuItem::action("Показать все", ShowAll),
        MenuItem::separator(),
        MenuItem::action("Завершить Agenda", Quit),
    ])]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::Keystroke;

    #[gpui::test]
    fn command_q_quits(cx: &mut gpui::TestAppContext) {
        cx.update(install);
        let (_, cx) = crate::a11y_tests::launch(cx);
        cx.update(|window, _| {
            let key = Keystroke::parse("cmd-q").unwrap();
            assert!(window.bindings_for_action(&Quit).iter().any(|binding| {
                binding.match_keystrokes(std::slice::from_ref(&key)) == Some(false)
            }));
        });
    }
}
