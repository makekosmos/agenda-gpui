// Shared icon/ring/bars/button widgets matching the Vue components pixel-for-pixel.
use gpui::prelude::*;
use gpui::{
    div, px, svg, Div, ElementId, Hsla, Role, SharedString, Stateful, Styled, Toggled, Window,
};

use crate::model::Status;
use crate::theme::*;

// ---------------------------------------------------------------------------
// Accessibility helpers for the app's custom clickable `div`s. Every
// interactive element gets an AccessKit role and a Russian accessible name;
// string (`"sb-inbox"`) and name+int (`("tr", 3)`) element ids are also
// exported as the platform AutomationId / AXIdentifier, so automation
// locators stay stable across label changes.
// ---------------------------------------------------------------------------

/// `Role::Button` / `Role::MenuItem` / toggle roles + an accessible name
/// for `Stateful<Div>` controls. Names must be Russian, matching the
/// user-facing label (or describing the control when it's icon-only).
pub trait A11y: Sized {
    /// `Role::Button` + accessible name.
    fn a11y_button(self, name: impl Into<SharedString>) -> Self;
    /// `Role::MenuItem` + accessible name — rows inside popup menus and
    /// dropdowns.
    fn a11y_menu_item(self, name: impl Into<SharedString>) -> Self;
    /// `Role::Switch` + accessible name + on/off state.
    fn a11y_switch(self, name: impl Into<SharedString>, on: bool) -> Self;
    /// `Role::CheckBox` + accessible name + checked state — the task
    /// completion ring.
    fn a11y_checkbox(self, name: impl Into<SharedString>, on: bool) -> Self;
}

impl A11y for Stateful<Div> {
    fn a11y_button(self, name: impl Into<SharedString>) -> Self {
        a11y_id(self.role(Role::Button).aria_label(name))
    }
    fn a11y_menu_item(self, name: impl Into<SharedString>) -> Self {
        a11y_id(self.role(Role::MenuItem).aria_label(name))
    }
    fn a11y_switch(self, name: impl Into<SharedString>, on: bool) -> Self {
        a11y_id(
            self.role(Role::Switch)
                .aria_label(name)
                .aria_toggled(toggled(on)),
        )
    }
    fn a11y_checkbox(self, name: impl Into<SharedString>, on: bool) -> Self {
        a11y_id(
            self.role(Role::CheckBox)
                .aria_label(name)
                .aria_toggled(toggled(on)),
        )
    }
}

fn toggled(on: bool) -> Toggled {
    if on {
        Toggled::True
    } else {
        Toggled::False
    }
}

/// Export the element id as the platform AutomationId where it carries a
/// name; non-name ids (focus handles, paths) have no stable string form.
fn a11y_id(el: Stateful<Div>) -> Stateful<Div> {
    match gpui::Element::id(&el) {
        Some(ElementId::Name(id)) => el.accessibility_id(id),
        Some(ElementId::NamedInteger(id, i)) => {
            el.accessibility_id(SharedString::from(format!("{id}-{i}")))
        }
        _ => el,
    }
}

/// Render a bundled monochrome SVG icon at `size` px tinted `color`.
pub fn icon(path: &'static str, size: f32, color: Hsla) -> impl IntoElement {
    svg()
        .path(path)
        .w(px(size))
        .h(px(size))
        .flex_none()
        .text_color(color)
}

/// Task checkbox — 14px ring; done = accent fill + check, otherwise empty.
pub fn status_ring(status: Status) -> impl IntoElement {
    let ring = div()
        .w(px(14.))
        .h(px(14.))
        .rounded_full()
        .border(px(1.5))
        .flex_none()
        .grid()
        .items_center()
        .justify_center();
    if status == Status::Done {
        ring.border_color(c(ACCENT())).bg(c(ACCENT())).child(icon(
            "icons/status-check.svg",
            9.,
            c(ACCENT_FG()),
        ))
    } else {
        ring.border_color(c(MUTED_FG()))
    }
}

/// One-line hover note for small controls, ported from zeron's TextTooltip:
/// same frosted chip metrics (px 9 / py 6, r6, 11px muted on the popover
/// surface). No backdrop blur here — this gpui pin lacks it, so the chip uses
/// the opaque popover color like every other overlay in the app.
pub struct TextTooltip {
    text: SharedString,
}

impl gpui::Render for TextTooltip {
    fn render(&mut self, _window: &mut Window, _cx: &mut gpui::Context<Self>) -> impl IntoElement {
        div()
            .max_w(px(320.))
            .px(px(9.))
            .py(px(6.))
            .rounded(px(6.))
            .border_1()
            .border_color(c(BORDER()))
            .bg(c(POPOVER()))
            .text_size(crate::theme::text_px(11.))
            .text_color(c(MUTED_FG()))
            .child(self.text.clone())
    }
}

/// `.tooltip(...)` builder for a [`TextTooltip`].
pub fn text_tooltip(
    text: impl Into<SharedString>,
) -> impl Fn(&mut Window, &mut gpui::App) -> gpui::AnyView + 'static {
    let text: SharedString = text.into();
    move |_, cx| cx.new(|_| TextTooltip { text: text.clone() }).into()
}
