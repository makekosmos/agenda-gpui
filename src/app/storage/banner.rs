use super::*;
use crate::widgets::{icon, A11y};
use gpui::px;

impl Agenda {
    /// Bottom-left notice strip. Anchored left and capped at the FAB's strip
    /// on the right, so it can never cover the «Новая задача» button.
    /// Write failures show only × and auto-hide; load failures also get
    /// «Обновить» and stay until data arrives.
    pub(crate) fn storage_banner(&self, cx: &mut Context<Self>) -> AnyElement {
        let (text, is_write) = if self.storage_busy {
            ("Сохранение / загрузка…".to_string(), true)
        } else if let Some(error) = &self.storage_error {
            (error.text.clone(), error.fault == StorageFault::Write)
        } else {
            return div().into_any_element();
        };
        div()
            .id("storage-toast")
            .debug_selector(|| "storage-toast".to_string())
            .absolute()
            .bottom_3()
            .left_3()
            // FAB lives in the bottom-right ~190px strip; reserve it.
            .right(px(190.))
            .p_3()
            .rounded_md()
            .bg(c(BG()))
            .text_color(c(FG()))
            .text_sm()
            .flex()
            .items_center()
            .gap_3()
            .child(div().flex_1().child(text))
            .when(!self.storage_busy && !is_write, |d| {
                d.child(
                    div()
                        .id("reload-engine")
                        .debug_selector(|| "reload-engine".to_string())
                        .cursor_pointer()
                        .child("Обновить")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.reload_storage();
                            cx.notify();
                        }))
                        .a11y_button("Обновить"),
                )
            })
            .when(!self.storage_busy, |d| {
                d.child(
                    div()
                        .id("storage-toast-close")
                        .debug_selector(|| "storage-toast-close".to_string())
                        .cursor_pointer()
                        .child(icon("icons/status-x.svg", 12., c(MUTED_FG())))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.dismiss_storage_error();
                            cx.notify();
                        }))
                        .a11y_button("Закрыть"),
                )
            })
            .into_any_element()
    }
}
