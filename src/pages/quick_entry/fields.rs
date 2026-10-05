use super::*;
use gpui::{App, Div, Entity};
use gpui_component::input::TextareaState;

pub(super) fn title(title_state: &Entity<TextareaState>, cx: &App) -> Div {
    div().pl(px(14.)).pr_4().pt_3().flex_none().child(
        crate::text_field::text_field("qe-title", "Название задачи", title_state.clone(), cx)
            .w_full()
            .flex()
            .flex_col()
            .child(
                Textarea::new(title_state)
                    .role(None)
                    .w_full()
                    .flex_none()
                    .text_size(crate::theme::text_px(24.))
                    .line_height(crate::theme::text_px(32.))
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(c(FG()))
                    .p_0()
                    .appearance(false)
                    .bordered(false),
            ),
    )
}

pub(super) fn notes(notes_state: &Entity<TextareaState>, cx: &App) -> Div {
    div()
        .pl(px(14.))
        .pr_4()
        .pt_2()
        .pb_2()
        .flex_1()
        .min_h_0()
        .child(
            crate::text_field::text_field("qe-notes", "Заметки", notes_state.clone(), cx)
                .size_full()
                .child(
                    Textarea::new(notes_state)
                        .role(None)
                        .w_full()
                        .h_full()
                        .text_size(crate::theme::text_px(13.))
                        .text_color(c(FG()))
                        .p_0()
                        .appearance(false)
                        .bordered(false),
                ),
        )
}
