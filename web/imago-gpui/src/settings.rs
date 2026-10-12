//! Reusable settings geometry extracted from Cortex. No Engine or app state.
//! Applications supply their resolved accent and text scale through `UiStyle`.
use crate::theme::{c, rgba, ACCENT, BORDER, CARD, FG, MUTED_FG};
use ::gpui::{prelude::*, *};

pub const PAGE_WIDTH: f32 = 760.;
pub const INSET: f32 = 16.;
pub const GAP: f32 = 12.;
pub const SECTION_GAP: f32 = 32.;
pub const LABEL_WIDTH: f32 = 180.;

#[derive(Clone, Copy, Debug)]
pub struct UiStyle {
    pub foreground: u32,
    pub muted: u32,
    pub accent: u32,
    pub border: u32,
    pub surface: u32,
    pub font_scale: f32,
}

impl Default for UiStyle {
    fn default() -> Self {
        Self {
            foreground: FG(),
            muted: MUTED_FG(),
            accent: ACCENT(),
            border: BORDER(),
            surface: CARD(),
            font_scale: 1.,
        }
    }
}

pub fn page_stack() -> Div {
    div().w_full().min_w_0().flex().flex_col().gap(px(24.))
}
pub fn page_sections() -> Div {
    div()
        .w_full()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(SECTION_GAP))
}
pub fn section_group() -> Div {
    div().w_full().min_w_0().flex().flex_col().gap(px(GAP))
}

impl UiStyle {
    pub fn text_px(self, size: f32) -> Pixels {
        let scale = if self.font_scale.is_finite() && self.font_scale > 0. {
            self.font_scale
        } else {
            1.
        };
        px(size * scale)
    }
    pub fn heading(
        self,
        id: impl Into<ElementId>,
        title: impl Into<SharedString>,
    ) -> Stateful<Div> {
        let title = title.into();
        div()
            .id(id)
            .role(Role::Heading)
            .aria_level(1)
            .aria_label(title.clone())
            .text_size(self.text_px(24.))
            .line_height(self.text_px(30.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(c(self.foreground))
            .mb_4()
            .child(title)
    }
    pub fn input_field(
        self,
        state: &Entity<gpui_component::input::InputState>,
    ) -> gpui_component::input::Input {
        gpui_component::input::Input::new(state)
            .h(px(32.))
            .text_size(self.text_px(13.))
            .line_height(self.text_px(18.))
    }
    pub fn empty(self, text: &str) -> Div {
        div()
            .w_full()
            .min_w_0()
            .py(px(12.))
            .text_size(self.text_px(13.))
            .line_height(self.text_px(18.))
            .text_color(c(self.muted))
            .child(text.to_owned())
    }
    pub fn section(self, title: &str, hint: &str) -> Div {
        div()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(4.))
            .child(
                div()
                    .text_size(self.text_px(15.))
                    .line_height(self.text_px(20.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(title.to_owned()),
            )
            .when(!hint.is_empty(), |d| {
                d.child(
                    div()
                        .text_size(self.text_px(12.))
                        .line_height(self.text_px(16.))
                        .text_color(c(self.muted))
                        .child(hint.to_owned()),
                )
            })
    }
    /// Cortex's soft ink plaque: 12px corners, no border; physical geometry.
    pub fn card(self) -> Div {
        div()
            .w_full()
            .min_w_0()
            .rounded(px(12.))
            .overflow_hidden()
            .bg(rgba(self.foreground, 0.045))
            .px(px(INSET))
            .py(px(12.))
            .flex()
            .flex_col()
            .gap(px(GAP))
    }
    pub fn row_copy(self, label: impl Into<String>, sub: impl Into<String>) -> Div {
        let sub = sub.into();
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(
                div()
                    .text_size(self.text_px(13.))
                    .line_height(self.text_px(18.))
                    .font_weight(FontWeight::MEDIUM)
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(label.into()),
            )
            .when(!sub.is_empty(), |d| {
                d.child(
                    div()
                        .text_size(self.text_px(12.))
                        .line_height(self.text_px(16.))
                        .text_color(c(self.muted))
                        .child(sub),
                )
            })
    }
    pub fn row(self, label: impl Into<String>, sub: impl Into<String>) -> Div {
        div()
            .w_full()
            .min_w_0()
            .min_h(px(36.))
            .flex()
            .items_center()
            .gap(px(GAP))
            .child(self.row_copy(label, sub))
    }
    pub fn key_column(self, key: &str) -> Div {
        div()
            .w(px(LABEL_WIDTH))
            .flex_none()
            .text_size(self.text_px(13.))
            .line_height(self.text_px(18.))
            .text_color(c(self.muted))
            .child(key.to_owned())
    }
    pub fn kv(self, key: &str, value: impl Into<String>) -> Div {
        div()
            .w_full()
            .min_w_0()
            .flex()
            .items_start()
            .gap(px(GAP))
            .child(self.key_column(key))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_size(self.text_px(13.))
                    .line_height(self.text_px(18.))
                    .child(value.into()),
            )
    }
    pub fn section_label(self, label: impl Into<SharedString>) -> Div {
        div()
            .px(px(8.))
            .text_size(self.text_px(13.))
            .line_height(self.text_px(17.))
            .text_color(c(self.muted))
            .child(label.into())
    }
    pub fn section_block(self, label: impl Into<SharedString>, block: impl IntoElement) -> Div {
        div()
            .mt(px(SECTION_GAP))
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(self.section_label(label))
            .child(block)
    }
    pub fn settings_card(self) -> Div {
        self.card().px(px(0.)).py(px(0.)).gap(px(0.))
    }
    pub fn card_row(self, first: bool) -> Div {
        div()
            .mx(px(INSET))
            .py(px(12.))
            .min_h(px(60.))
            .when(!first, |row| {
                row.border_t_1().border_color(rgba(self.border, 0.6))
            })
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(16.))
    }
    pub fn row_title(self, title: impl Into<SharedString>) -> Div {
        div()
            .min_w_0()
            .text_size(self.text_px(13.))
            .line_height(self.text_px(17.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(c(self.foreground))
            .child(title.into())
    }
    pub fn row_meta(self, text: &str) -> Div {
        div()
            .mt(px(2.))
            .min_w_0()
            .text_size(self.text_px(12.))
            .line_height(self.text_px(16.))
            .text_color(rgba(self.muted, 0.65))
            .child(text.to_owned())
    }
}
