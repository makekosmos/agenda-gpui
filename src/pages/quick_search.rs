use super::*;

impl Agenda {
    // ==========================================================================
    // Quick search overlay (Ctrl+K) — dark panel top-center.
    // ==========================================================================

    pub(crate) fn render_quick_search(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let qs = self.input_state(window, cx, "qs", "Поиск задач и проектов...");
        if !self.qs_focused {
            self.qs_focused = true;
            qs.update(cx, |s, cx| s.focus(window, cx));
        }
        let query = qs.read(cx).value().to_string();
        self.quick_query = query.clone();

        let (todos, projects) = self.quick_matches();
        let mut results: Vec<gpui::Stateful<gpui::Div>> = vec![];
        let mut idx = 0usize;
        for t in &todos {
            let hid = format!("qs-t-{}", t.id);
            let ht = self.hover_t(window, &hid);
            let sel = self.quick_sel == idx;
            let weak = cx.weak_entity();
            let tid = t.id.to_string();
            let key = SharedString::from(hid.clone());
            results.push(
                div()
                    .id(SharedString::from(format!("el-{hid}")))
                    .h_10()
                    .w_full()
                    .px_3()
                    .flex()
                    .items_center()
                    .gap_2p5()
                    .text_size(px(13.))
                    .text_color(c(FG()))
                    .bg(panel_mix(if sel { 0.10 } else { 0.06 * ht }))
                    .child(status_ring(task_status(t)))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .child(t.title.clone()),
                    )
                    .on_hover(move |hovered, _, cx| {
                        let _ = weak.update(cx, |this, _| {
                            this.set_hover(&key, *hovered);
                        });
                    })
                    .on_click({
                        let weak = cx.weak_entity();
                        let tid = tid.clone();
                        move |_: &ClickEvent, _, cx| {
                            let _ = weak.update(cx, |this, _| {
                                this.quick_open = false;
                                this.navigate(Route::Task(tid.clone()));
                            });
                        }
                    })
                    .a11y_button(t.title.clone()),
            );
            idx += 1;
        }
        for p in &projects {
            let hid = format!("qs-p-{}", p.id);
            let ht = self.hover_t(window, &hid);
            let sel = self.quick_sel == idx;
            let weak = cx.weak_entity();
            let pid = p.id.to_string();
            let key = SharedString::from(hid.clone());
            results.push(
                div()
                    .id(SharedString::from(format!("el-{hid}")))
                    .h_10()
                    .w_full()
                    .px_3()
                    .flex()
                    .items_center()
                    .gap_2p5()
                    .text_size(px(13.))
                    .text_color(c(FG()))
                    .bg(panel_mix(if sel { 0.10 } else { 0.06 * ht }))
                    .child(icon("icons/folder.svg", 14., c(MUTED_FG())))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .child(p.title.clone()),
                    )
                    .on_hover(move |hovered, _, cx| {
                        let _ = weak.update(cx, |this, _| {
                            this.set_hover(&key, *hovered);
                        });
                    })
                    .on_click({
                        let weak = cx.weak_entity();
                        let pid = pid.clone();
                        move |_: &ClickEvent, _, cx| {
                            let _ = weak.update(cx, |this, _| {
                                this.quick_open = false;
                                this.navigate(Route::Project(pid.clone()));
                            });
                        }
                    })
                    .a11y_button(p.title.clone()),
            );
            idx += 1;
        }

        let mut body = div().flex().flex_col();
        if query.trim().is_empty() {
            body = body.child(
                div()
                    .h(px(68.))
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(13.))
                    .text_color(c(MUTED_FG()))
                    .child("Начните вводить для поиска"),
            );
        } else if results.is_empty() {
            body = body.child(
                div()
                    .min_h(px(92.))
                    .w_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(c(FG()))
                            .child("Ничего не найдено"),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(c(MUTED_FG()))
                            .child(format!("По запросу «{}» совпадений нет", query.trim())),
                    ),
            );
        } else {
            body = body.children(results);
        }

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
                    .h(px(56.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap_2p5()
                    .px_3p5()
                    .child(icon("icons/search.svg", 16., c(MUTED_FG())))
                    .child(
                        Input::new(&qs)
                            .accessibility_id("qs")
                            .flex_1()
                            .text_size(px(14.))
                            .text_color(c(FG()))
                            .appearance(false)
                            .bordered(false)
                            .h(px(24.)),
                    )
                    .child(
                        div()
                            .h_5()
                            .px_1p5()
                            .flex()
                            .items_center()
                            .rounded(px(4.))
                            .border_1()
                            .border_color(panel_mix(0.22))
                            .text_size(px(10.))
                            .text_color(c(MUTED_FG()))
                            .child("esc"),
                    ),
            )
            .child(div().h_px().w_full().bg(panel_mix(0.10)))
            .child(body);

        let weak = cx.weak_entity();
        div()
            .absolute()
            .inset_0()
            .bg(rgba(0x000000, 0.30))
            .flex()
            .justify_center()
            .child(div().id("qs-backdrop").absolute().inset_0().on_mouse_down(
                MouseButton::Left,
                move |_: &MouseDownEvent, _, cx| {
                    let _ = weak.update(cx, |this, _| {
                        this.quick_open = false;
                        this.qs_focused = false;
                    });
                },
            ))
            .child(div().mt(px(128.)).child(deferred(panel)))
            .into_any_element()
    }
}
