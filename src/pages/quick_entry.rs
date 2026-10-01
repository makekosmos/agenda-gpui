use super::*;

impl Agenda {
    // ==========================================================================
    // Quick entry overlay (Ctrl+N) — dark panel top-center.
    // ==========================================================================

    pub(crate) fn render_quick_entry(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // contextual defaults — once the user picks/clears the chip
        // (`qe_project_touched`), a cleared state must not be re-seeded.
        if self.qe_project.is_none() && !self.qe_project_touched {
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
            // The panel sits above the inset_0 click-away backdrop: keep
            // inside clicks from reaching it and dismissing the overlay.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
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
                            .debug_selector(|| "qe-close".to_string())
                            .w_7()
                            .h_7()
                            .grid()
                            .items_center()
                            .justify_center()
                            .rounded_md()
                            .child(icon("icons/status-x.svg", 14., c(MUTED_FG())))
                            .on_click({
                                let weak = weak.clone();
                                move |_: &ClickEvent, window, cx| {
                                    let _ = weak.update(cx, |this, cx| {
                                        this.quick_entry_open = false;
                                        this.qe_focused = false;
                                        this.root_focus.focus(window, cx);
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

        div()
            .absolute()
            .inset_0()
            // Modal: nothing below the overlay may see this press (e.g. a
            // task row under the dim area), so occlude everything beneath.
            .occlude()
            .bg(rgba(0x000000, 0.30))
            .flex()
            .justify_center()
            .child(div().id("qe-backdrop").absolute().inset_0().on_mouse_down(
                MouseButton::Left,
                move |_: &MouseDownEvent, window, cx| {
                    let _ = weak.update(cx, |this, cx| {
                        this.quick_entry_open = false;
                        this.qe_focused = false;
                        this.root_focus.focus(window, cx);
                    });
                },
            ))
            .child(div().mt(px(136.)).child(deferred(panel)))
            .into_any_element()
    }
}
