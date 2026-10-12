//! Cortex's product hero, parameterized for any Mundus application.
//! Diagnostics, update actions and support IO remain owned by the caller.
mod diagnostics;
pub use diagnostics::{DiagnosticCheck, DiagnosticState, Diagnostics};

use crate::{settings::UiStyle, theme::c};
use ::gpui::{prelude::*, *};

#[derive(IntoElement)]
pub struct AboutHero {
    id: ElementId,
    name: SharedString,
    version: SharedString,
    copyright: Option<SharedString>,
    icon: Option<SharedString>,
    style: UiStyle,
}

impl AboutHero {
    pub fn new(
        id: impl Into<ElementId>,
        name: impl Into<SharedString>,
        version: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: version.into(),
            copyright: None,
            icon: None,
            style: UiStyle::default(),
        }
    }
    pub fn icon(mut self, path: impl Into<SharedString>) -> Self {
        self.icon = Some(path.into());
        self
    }
    pub fn copyright(mut self, text: impl Into<SharedString>) -> Self {
        self.copyright = Some(text.into());
        self
    }
    pub fn style(mut self, style: UiStyle) -> Self {
        self.style = style;
        self
    }
}

impl RenderOnce for AboutHero {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let selector = self.id.to_string();
        let style = self.style;
        let name = self.name;
        div()
            .id(self.id)
            .debug_selector(move || selector.clone())
            .flex()
            .flex_col()
            .items_center()
            .pt_8()
            .pb_6()
            .when_some(self.icon, |hero, path| {
                hero.child(
                    gpui_component::Icon::default()
                        .path(path)
                        .size(px(96.))
                        .text_color(c(style.accent)),
                )
            })
            .child(
                div()
                    .id("about-product-name")
                    .role(Role::Heading)
                    .aria_level(1)
                    .aria_label(name.clone())
                    .pt_3()
                    .text_size(style.text_px(22.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(c(style.foreground))
                    .child(name),
            )
            .child(
                div()
                    .pt_1()
                    .text_size(style.text_px(13.))
                    .text_color(c(style.muted))
                    .child(self.version),
            )
            .when_some(self.copyright, |hero, text| {
                hero.child(
                    div()
                        .pt_2()
                        .text_size(style.text_px(12.))
                        .text_color(c(style.muted))
                        .child(text),
                )
            })
    }
}
