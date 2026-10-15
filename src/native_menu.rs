//! Native macOS app menu, same contract as Manager. Quit only exits GPUI.

use gpui::{actions, App, KeyBinding, Menu, MenuItem, SystemMenuType};
#[cfg(target_os = "macos")]
use objc::{class, msg_send, sel};

actions!(
    agenda_menu,
    [Quit, Hide, HideOthers, ShowAll, CloseWindow, Minimize]
);

pub fn install(cx: &mut App) {
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.on_action(|_: &Hide, cx| cx.hide());
    cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
    cx.on_action(|_: &CloseWindow, _| perform_close());
    cx.on_action(|_: &Minimize, _| miniaturize());
    cx.bind_keys([
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-h", Hide, None),
        KeyBinding::new("cmd-р", Hide, None),
        KeyBinding::new("cmd-w", CloseWindow, None),
        KeyBinding::new("cmd-ц", CloseWindow, None),
        KeyBinding::new("cmd-m", Minimize, None),
        KeyBinding::new("cmd-ь", Minimize, None),
        KeyBinding::new("cmd-alt-h", HideOthers, None),
    ]);
    cx.set_menus([
        Menu::new("Agenda").items([
            MenuItem::os_submenu("Службы", SystemMenuType::Services),
            MenuItem::separator(),
            MenuItem::action("Скрыть Agenda", Hide),
            MenuItem::action("Скрыть остальные", HideOthers),
            MenuItem::action("Показать все", ShowAll),
            MenuItem::separator(),
            MenuItem::action("Завершить Agenda", Quit),
        ]),
        Menu::new("Окно").items([
            MenuItem::action("Свернуть", Minimize),
            MenuItem::action("Закрыть", CloseWindow),
        ]),
    ]);
}

/// Same path as the red button. Calling it inside the menu action re-enters
/// GPUI while `App` is borrowed, so it runs on the next main-queue turn.
fn perform_close() {
    #[cfg(target_os = "macos")]
    schedule(WindowCommand::Close);
}

fn miniaturize() {
    #[cfg(target_os = "macos")]
    schedule(WindowCommand::Minimize);
}

#[cfg(target_os = "macos")]
enum WindowCommand {
    Close,
    Minimize,
}

#[cfg(target_os = "macos")]
fn schedule(command: WindowCommand) {
    let context = Box::into_raw(Box::new(command)).cast();
    unsafe {
        // `dispatch_get_main_queue` is a macro over this symbol.
        dispatch_async_f(
            &raw const _dispatch_main_q as *mut _,
            context,
            run_window_command,
        );
    }
}

#[cfg(target_os = "macos")]
extern "C" fn run_window_command(context: *mut std::ffi::c_void) {
    let command = unsafe { Box::from_raw(context.cast::<WindowCommand>()) };
    unsafe {
        let window = key_window();
        if window.is_null() {
            return;
        }
        match *command {
            WindowCommand::Close => {
                let _: () =
                    msg_send![window, performClose: std::ptr::null::<objc::runtime::Object>()];
            }
            WindowCommand::Minimize => {
                let _: () =
                    msg_send![window, miniaturize: std::ptr::null::<objc::runtime::Object>()];
            }
        }
    }
}

#[cfg(target_os = "macos")]
unsafe fn key_window() -> *mut objc::runtime::Object {
    let app: *mut objc::runtime::Object = msg_send![class!(NSApplication), sharedApplication];
    if app.is_null() {
        return std::ptr::null_mut();
    }
    let window: *mut objc::runtime::Object = msg_send![app, keyWindow];
    if !window.is_null() {
        return window;
    }
    msg_send![app, mainWindow]
}

#[cfg(target_os = "macos")]
extern "C" {
    fn dispatch_async_f(
        queue: *mut std::ffi::c_void,
        context: *mut std::ffi::c_void,
        work: extern "C" fn(*mut std::ffi::c_void),
    );
    static _dispatch_main_q: u8;
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
