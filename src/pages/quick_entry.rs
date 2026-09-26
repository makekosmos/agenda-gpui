use super::*;

impl Agenda {
    // ==========================================================================
    // Quick entry overlay (Ctrl+N) — dark panel top-center + significance card.
    // ==========================================================================

    pub(crate) fn render_quick_entry(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // contextual defaults
        if self.qe_project.is_none() {
            if let Route::Project(pid) = &self.route {
                self.qe_project = Some(pid.clone());
            }
        }
        if self.qe_date.is_none() && !self.qe_date_touched && self.route == Route::Today {
            self.qe_date = Some(today_key());
        }
        let title_state = self.input_state(window, cx, "qe-title", "Новая задача");
        let notes_state = self.input_state(window, cx, "qe-notes", "Заметки");
        if !self.qe_focused {
            self.qe_focused = true;
            title_state.update(cx, |s, cx| s.focus(window, cx));
        }

        let weak = cx.weak_entity();

        let chips = self.qe_chips(window, cx);

        let panel = div()
            .w(px(640.))
            .flex()
            .flex_col()
            .rounded(px(10.))
            .overflow_hidden()
            .bg(c(POPOVER()))
            .border_1()
            .border_color(panel_mix(0.10))
            .shadow(vec![gpui::BoxShadow {
                color: rgba(0x000000, 0.35),
                offset: gpui::point(px(0.), px(16.)),
                blur_radius: px(48.),
                spread_radius: px(0.),
                inset: false,
            }])
            .child(
                div()
                    .flex()
                    .items_center()
                    .pl_4()
                    .pr_2()
                    .child(
                        Input::new(&title_state)
                            .accessibility_id("qe-title")
                            .flex_1()
                            .h(px(48.))
                            .text_size(px(15.))
                            .text_color(c(FG()))
                            .appearance(false)
                            .bordered(false),
                    )
                    .child(
                        div()
                            .id("qe-close")
                            .w_7()
                            .h_7()
                            .grid()
                            .items_center()
                            .justify_center()
                            .rounded_md()
                            .child(icon("icons/status-x.svg", 14., c(MUTED_FG())))
                            .on_click({
                                let weak = weak.clone();
                                move |_: &ClickEvent, _, cx| {
                                    let _ = weak.update(cx, |this, _| {
                                        this.quick_entry_open = false;
                                        this.qe_focused = false;
                                    });
                                }
                            })
                            .a11y_button("Закрыть"),
                    ),
            )
            .child(
                div().pl_4().pr_2().child(
                    Input::new(&notes_state)
                        .accessibility_id("qe-notes")
                        .w_full()
                        .h(px(36.))
                        .text_size(px(13.))
                        .text_color(c(FG()))
                        .appearance(false)
                        .bordered(false),
                ),
            )
            .child(div().h_px().w_full().bg(panel_mix(0.10)))
            .child(chips);

        // significance card (bottom-right white card)
        let sig_label = self
            .qe_sig
            .map(|s| format!("{}/10", s))
            .unwrap_or_else(|| "Не оценено".to_string());
        let sig_value = self.qe_sig.unwrap_or(0);
        let fill = self.qe_sig.unwrap_or(5) as f32 / 10.0;
        let sig_card = div()
            .absolute()
            .right_6()
            .bottom_6()
            .px_4()
            .py_3()
            .rounded_lg()
            .border_1()
            .border_color(c(BORDER()))
            .bg(c(POPOVER()))
            .shadow(vec![gpui::BoxShadow {
                color: rgba(0x000000, 0.15),
                offset: gpui::point(px(0.), px(12.)),
                blur_radius: px(32.),
                spread_radius: px(0.),
                inset: false,
            }])
            .flex()
            .items_center()
            .gap(px(10.))
            .text_size(px(12.))
            .text_color(c(FG()))
            .child("Значимость")
            .child(
                div()
                    .id("qe-sig-slider")
                    .w(px(120.))
                    .h(px(20.))
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .w_full()
                            .h(px(4.))
                            .rounded_full()
                            .bg(c(SECONDARY()))
                            .child(
                                div()
                                    .h_full()
                                    .w(gpui::DefiniteLength::Fraction(fill))
                                    .rounded_full()
                                    .bg(c(ACCENT())),
                            ),
                    )
                    .child(
                        div()
                            .ml(px(-8. - 104. * (1.0 - fill)))
                            .w(px(16.))
                            .h(px(16.))
                            .rounded_full()
                            .bg(c(ACCENT())),
                    )
                    .on_click({
                        let weak = weak.clone();
                        move |ev: &ClickEvent, _, cx| {
                            let x: f32 = ev.position().x.into();
                            let _ = weak.update(cx, |this, _| {
                                // slider is 120px wide; anchor from window right edge
                                let v = ((x - (1440. - 24. - 16. - 96. - 120.)) / 120. * 10.)
                                    .round()
                                    .clamp(1., 10.) as u8;
                                this.qe_sig = Some(v);
                            });
                        }
                    })
                    .role(gpui::Role::Slider)
                    .aria_label("Значимость")
                    .aria_numeric_value(sig_value as f64)
                    .aria_min_numeric_value(1.0)
                    .aria_max_numeric_value(10.0)
                    .aria_value(sig_label.clone())
                    .accessibility_id("qe-sig-slider"),
            )
            .child(div().text_color(c(MUTED_FG())).child(sig_label));

        div()
            .absolute()
            .inset_0()
            .bg(rgba(0x000000, 0.30))
            .flex()
            .justify_center()
            .child(div().id("qe-backdrop").absolute().inset_0().on_mouse_down(
                MouseButton::Left,
                move |_: &MouseDownEvent, _, cx| {
                    let _ = weak.update(cx, |this, _| {
                        this.quick_entry_open = false;
                        this.qe_focused = false;
                    });
                },
            ))
            .child(div().mt(px(136.)).child(deferred(panel)))
            .child(deferred(sig_card))
            .into_any_element()
    }
}
