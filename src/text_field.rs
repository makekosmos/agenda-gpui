//! Bind the system text-field node to the editor's actual keyboard focus.
//! gpui-component's frame has a separate focus handle from its editor, so
//! exposing that frame alone leaves AXFocusedUIElement pointing at the window.
use gpui::{div, prelude::*, App, Div, Entity, Focusable, Role, Stateful};
use gpui_component::input::{InputState, TextareaState};

/// NSWindow must forward AXFocusedUIElement to AccessKit's content view.
/// Call after GPUI has created its first native window (and registered classes).
pub(crate) fn init_native_focus() {
    #[cfg(target_os = "macos")]
    {
        static INIT: std::sync::Once = std::sync::Once::new();
        INIT.call_once(|| unsafe {
            // GPUI owns this NSWindow subclass; AccessKit is statically linked
            // into the executable and remains loaded for the process lifetime.
            accesskit_macos::add_focus_forwarder_to_window_class("GPUIWindow");
        });
    }
}

pub(crate) enum TextFieldState {
    Input(Entity<InputState>),
    Textarea(Entity<TextareaState>),
}

impl From<Entity<InputState>> for TextFieldState {
    fn from(state: Entity<InputState>) -> Self {
        Self::Input(state)
    }
}

impl From<Entity<TextareaState>> for TextFieldState {
    fn from(state: Entity<TextareaState>) -> Self {
        Self::Textarea(state)
    }
}

/// The child component must use `.role(None)` to avoid duplicate text fields.
pub(crate) fn text_field(
    id: &'static str,
    name: &'static str,
    state: impl Into<TextFieldState>,
    cx: &App,
) -> Stateful<Div> {
    let state = state.into();
    let (focus, value, role) = match &state {
        TextFieldState::Input(s) => (s.focus_handle(cx), s.read(cx).value(), Role::TextInput),
        TextFieldState::Textarea(s) => (
            s.focus_handle(cx),
            s.read(cx).value(),
            Role::MultilineTextInput,
        ),
    };
    div()
        .id(id)
        .accessibility_id(id)
        .role(role)
        .aria_label(name)
        .aria_value(value)
        .track_focus(&focus)
        .on_a11y_action(
            gpui::accesskit::Action::SetValue,
            move |data, window, cx| {
                let Some(gpui::accesskit::ActionData::Value(value)) = data else {
                    return;
                };
                match &state {
                    TextFieldState::Input(s) => s.update(cx, |s, cx| {
                        s.replace_all(value.to_string(), window, cx);
                    }),
                    TextFieldState::Textarea(s) => s.update(cx, |s, cx| {
                        s.replace_all(value.to_string(), window, cx);
                    }),
                }
            },
        )
}
