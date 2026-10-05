use super::*;

/// One toggle row on the dev page.
struct DevRow {
    id: &'static str,
    title: &'static str,
    desc: &'static str,
    on: bool,
}

impl Agenda {
    pub(crate) fn energy_page(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .id("energy")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.scroll("energy"))
            .flex()
            .flex_col()
            .px_7()
            .pt_4()
            .gap_3()
            .w_full()
            .max_w(px(720.))
            .mx_auto()
            .child(settings::settings_heading("Энергосбережение"))
            .child(self.dev_row(
                DevRow {
                    id: "vsync",
                    title: "Вертикальная синхронизация (VSync)",
                    desc: "Частота экрана во время работы. При выключении — не более 60 FPS.",
                    on: self.vsync_enabled,
                },
                window,
                cx,
                |this| this.vsync_enabled = !this.vsync_enabled,
            ))
            .child(div().text_size(crate::theme::text_px(13.)).text_color(c(MUTED_FG()))
                .child("Без взаимодействия в простое и в фоне — 1 FPS. Ввод возвращает рабочую частоту даже без фокуса; через секунду после завершения взаимодействия частота снова снижается."))
            .into_any_element()
    }

    // ==========================================================================
    // Developers page (settings): design grid, FPS meter, task generator
    // ==========================================================================

    /// Row: label + description on the left, toggle switch on the right.
    /// Same visual language as the theme rows on the display page.
    fn dev_row(
        &mut self,
        row: DevRow,
        _window: &mut Window,
        cx: &mut Context<Self>,
        on_click: impl Fn(&mut Agenda) + 'static,
    ) -> gpui::Stateful<gpui::Div> {
        let style = crate::theme::ui_style();
        style
            .row(row.title, row.desc)
            .id(SharedString::from(format!("el-dev-{}", row.id)))
            .child(
                imago_gpui::toggle::toggle(row.id, row.on, cx, move |this, _, _| on_click(this))
                    .accessibility_label(row.title)
                    .glass(
                        self.sb_material != 0
                            && cfg!(any(target_os = "macos", target_os = "windows")),
                    )
                    .colors(imago_gpui::toggle::ToggleColors {
                        accent: style.accent,
                        foreground: style.foreground,
                        surface: style.surface,
                    }),
            )
    }

    pub(crate) fn dev_page(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let section = |title: &'static str| {
            div()
                .pt_3()
                .text_size(crate::theme::text_px(13.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(c(FG()))
                .child(title)
        };

        div()
            .id("dev")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.scroll("dev"))
            .flex()
            .flex_col()
            .px_7()
            .pt_4()
            .gap_3()
            .w_full()
            .max_w(px(720.))
            .mx_auto()
            .child(
                div()
                    .text_size(crate::theme::text_px(15.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(c(FG()))
                    .child("Для разработчиков"),
            )
            .child(section("Инструменты"))
            .child(self.dev_row(
                DevRow {
                    id: "grid",
                    title: "Сетка 8px",
                    desc: "Полупрозрачная сетка поверх интерфейса для проверки выравнивания.",
                    on: self.dev_grid,
                },
                window,
                cx,
                |this| this.dev_grid = !this.dev_grid,
            ))
            .child(self.dev_row(
                DevRow {
                    id: "fps",
                    title: "FPS-счётчик",
                    desc: "Бейдж в правом нижнем углу со сглаженной частотой кадров.",
                    on: self.dev_fps,
                },
                window,
                cx,
                |this| {
                    this.dev_fps = !this.dev_fps;
                    if this.dev_fps {
                        // Warmup: first seconds after enabling are polluted by
                        // layout/shaping of the freshly mounted overlay.
                        this.fps_warmup =
                            Some(std::time::Instant::now() + std::time::Duration::from_secs(4));
                    }
                },
            ))
            .child(section("Данные"))
            .child({
                let hid = "dev-generate";
                let t = self.hover_t(window, hid);
                let weak = cx.weak_entity();
                let key = SharedString::from(hid);
                div()
                    .id(SharedString::from(format!("el-{hid}")))
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .border_1()
                    .border_color(rgba(FG(), 0.08 + 0.05 * t))
                    .bg(fg_mix(0.02 + 0.02 * t))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .child(
                                div()
                                    .text_size(crate::theme::text_px(13.))
                                    .text_color(c(FG()))
                                    .child("Сгенерировать 1000 задач"),
                            )
                            .child(
                                div()
                                    .text_size(crate::theme::text_px(11.))
                                    .text_color(c(MUTED_FG()))
                                    .child(format!(
                                        "Случайные названия и поля; ~55% завершённых за последний год. Сейчас задач: {}",
                                        self.todos.len()
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .h_8()
                            .px_3()
                            .flex()
                            .items_center()
                            .rounded_md()
                            .text_size(crate::theme::text_px(13.))
                            .text_color(c(ACCENT_FG()))
                            .bg(c(ACCENT()))
                            .child("Создать"),
                    )
                    .on_hover(move |hovered, _, cx| {
                        let _ = weak.update(cx, |this, _| this.set_hover(&key, *hovered));
                    })
                    .on_click({
                        let weak = cx.weak_entity();
                        move |_: &ClickEvent, _, cx| {
                            let _ = weak.update(cx, |this, cx| {
                                if !this.demo {
                                    this.raise(StorageError::notice(
                                        StorageFault::Write,
                                        "Генератор доступен только с AGENDA_DEMO=1",
                                    ));
                                    cx.notify();
                                    return;
                                }
                                this.dev_gen_batch += 1;
                                let seed = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .map(|d| d.as_nanos() as u64)
                                    .unwrap_or(0x9E3779B9)
                                    ^ this.dev_gen_batch;
                                let first_sort =
                                    this.todos.iter().map(|t| t.sort_order).max().unwrap_or(0) + 1;
                                let mut gen =
                                    gen_random_todos(1000, seed, this.dev_gen_batch, first_sort);
                                this.todos.append(&mut gen);
                                this.model_rev += 1;
                                cx.notify();
                            });
                        }
                    })
                    .a11y_button("Сгенерировать 1000 задач")
            })
            .into_any_element()
    }
}
