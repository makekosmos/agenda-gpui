use super::*;
use gpui::Div;
use gpui_component::kbd::Kbd;

/// Static shortcut reference — mirrors `Agenda::on_key` / input handlers.
const SHORTCUTS: &[(&str, &str)] = &[
    ("Боковая панель — открыть/скрыть", "cmd-b"),
    ("Новая задача — панель справа", "cmd-n"),
    ("Быстрый поиск", "cmd-k"),
    ("Обновить данные из Engine", "cmd-r"),
    ("Сохранить задачу / название", "enter"),
    ("Новая строка в поле", "shift-enter"),
    ("Закрыть панель / меню / поиск", "escape"),
];

fn shortcut_row(action: &'static str, keys: &'static str) -> Div {
    div()
        .flex()
        .items_center()
        .py_2()
        .gap_3()
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(text_px(13.))
                .text_color(c(FG()))
                .child(action),
        )
        .children(
            gpui::Keystroke::parse(keys)
                .ok()
                .map(|k| Kbd::new(k).into_any_element()),
        )
}
/// Settings headings live in the page, not in the window drag region.
pub(super) fn settings_heading(title: impl Into<SharedString>) -> gpui::Stateful<Div> {
    crate::theme::ui_style().heading("settings-heading", title)
}

fn settings_card() -> Div {
    crate::theme::ui_style().settings_card()
}

fn card_row(title: &'static str, description: &'static str) -> Div {
    let style = crate::theme::ui_style();
    style.card_row(true).child(
        div()
            .flex_1()
            .min_w(px(160.))
            .child(style.row_title(title))
            .when(!description.is_empty(), |row| {
                row.child(style.row_meta(description))
            }),
    )
}

impl Agenda {
    pub(crate) fn settings_page(
        &mut self,
        fuel: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut col = div()
            .id("settings")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.scroll("settings"))
            .flex()
            .flex_col()
            .px_7()
            .pt_4()
            .pb_8()
            .w_full()
            .max_w(px(720.))
            .mx_auto()
            .child(settings_heading(self.page_title().0));
        if fuel {
            return col.child(div().text_size(text_px(13.)).text_color(c(MUTED_FG()))
                .child("Мыслетопливо оценивает стоимость задач в процентах. Оценивайте задачи от 1 до 10 на странице задачи."))
                .into_any_element();
        }
        // The page itself is the shared `imago_gpui` editor, identical to
        // Manager's; Agenda only wires the scoped Engine transport and hides
        // the follow-apps row (Manager owns that toggle).
        let inherited = self.inherited_appearance();
        if inherited {
            col = col.child(settings_card().mb_4().child(card_row("Единый стиль приложений",
                "Оформление задаётся в Mundus Manager. Отключите «Единый стиль приложений» там, чтобы снова менять оформление Agenda.")
                .id("appearance-inherited")));
        }
        let editable = self.appearance.is_some() && self.appearance_editable();
        let status = if self.appearance.is_some() {
            ""
        } else {
            "Загрузка настроек внешнего вида…"
        };
        self.appearance_editor.update(cx, |editor, _| {
            editor.configure(crate::theme::ui_style(), false, editable);
            editor.status(status);
        });
        let shortcuts = settings_card().mt_4().child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .pb_1()
                        .text_size(text_px(13.))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(c(MUTED_FG()))
                        .child("Горячие клавиши"),
                )
                .children(
                    SHORTCUTS
                        .iter()
                        .map(|(action, keys)| shortcut_row(action, keys).into_any_element()),
                ),
        );
        col.child(self.appearance_editor.clone())
            .child(shortcuts)
            .into_any_element()
    }

    pub(crate) fn about_page(&mut self, cx: &mut Context<Self>) -> AnyElement {
        use imago_gpui::about::{AboutHero, DiagnosticCheck, DiagnosticState, Diagnostics};
        let style = crate::theme::ui_style();
        let weak = cx.weak_entity();
        let state = if self.storage_error.is_some() {
            DiagnosticState::Fail
        } else if self.storage_ready || self.demo {
            DiagnosticState::Ok
        } else {
            DiagnosticState::Pending
        };
        let label = if self.demo {
            "Демонстрационные данные (без Engine)"
        } else {
            "Доступ к данным Agenda через Engine"
        };
        div()
            .id("about")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.scroll("about"))
            .px_7()
            .pt_4()
            .pb_8()
            .child(
                imago_gpui::settings::page_stack()
                    .max_w(px(imago_gpui::settings::PAGE_WIDTH))
                    .mx_auto()
                    .child(
                        AboutHero::new(
                            "about-hero",
                            "Agenda",
                            concat!("Версия ", env!("CARGO_PKG_VERSION")),
                        )
                        .icon("icons/calendar-01.svg")
                        .copyright("© Yoso Industries 2025–2026")
                        .style(style),
                    )
                    .child(
                        Diagnostics::new(
                            "about-diagnostics-card",
                            vec![DiagnosticCheck::new(label, state)],
                        )
                        .expanded(self.about_diag_open)
                        .style(style)
                        .on_toggle(move |_, cx| {
                            let _ = weak.update(cx, |this, cx| {
                                this.about_diag_open = !this.about_diag_open;
                                cx.notify();
                            });
                        }),
                    )
                    .when(self.about_diag_open, |page| {
                        page.when_some(self.storage_error.as_ref(), |page, error| {
                            page.child(
                                style
                                    .card()
                                    .child(style.row("Доступ к данным", error.text.clone())),
                            )
                        })
                    }),
            )
            .into_any_element()
    }
}
