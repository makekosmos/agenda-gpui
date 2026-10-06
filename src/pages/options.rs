use super::*;

impl Agenda {
    /// .task-options popover (top-right, below the options button).
    pub(crate) fn render_options_popover(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let Some(key) = self.options_for.clone() else {
            return div().into_any_element();
        };
        let opts = self.board_opts(&key);

        // Section label: zeron's chip (`px-1.5 py-0.5 rounded bg-ink/5
        // text-[10px] text-muted`), not a plain line of text.
        let section = |title: &str| {
            div()
                .px(px(5.))
                .py(px(1.))
                .rounded(px(5.))
                .bg(rgba(FG(), 0.05))
                .text_size(crate::theme::text_px(10.))
                .text_color(c(MUTED_FG()))
                .child(title.to_string())
        };
        let key2 = key.clone();
        let item = move |app: &mut Self,
                         window: &mut Window,
                         cx: &mut Context<Self>,
                         i: usize,
                         label: &str,
                         checked: bool,
                         act: OptAct| {
            let hid = format!("opt-item-{i}-{label}");
            let t = app.hover_t(window, &hid);
            let weak = cx.weak_entity();
            let keyh = SharedString::from(hid.clone());
            let key3 = key2.clone();
            div()
                .id(SharedString::from(format!("el-{i}-{label}")))
                .w_full()
                .px_2()
                .py(px(6.))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(10.))
                // Concentric with the card: 12 - 1 border - 4 inset.
                .rounded(px(7.))
                .text_size(crate::theme::text_px(13.))
                .text_color(c(FG()))
                .cursor_pointer()
                // zeron menu_row: selected keeps the 11% wash, others fade it
                // in on hover.
                .bg(rgba(FG(), 0.11 * t.max(if checked { 1.0 } else { 0.0 })))
                .child(label.to_string())
                .when(checked, |el| {
                    el.child(icon("icons/check.svg", 14., c(MUTED_FG())))
                })
                .on_hover({
                    let weak = weak.clone();
                    move |hovered, _, cx| {
                        let _ = weak.update(cx, |this, _| this.set_hover(&keyh, *hovered));
                    }
                })
                .on_click(move |_: &ClickEvent, _, cx| {
                    let key3 = key3.clone();
                    let _ = weak.update(cx, |this, _| {
                        let mut o = this.boards.get(&key3).copied().unwrap_or_default();
                        match act {
                            OptAct::Sort(s) => o.sort = s,
                            OptAct::Group(g) => o.group = g,
                        }
                        this.boards.insert(key3, o);
                        this.options_for = None;
                    });
                })
                .aria_selected(checked)
                .a11y_menu_item(label.to_string())
        };

        // Right edge of the options button in the titlebar: on Windows it
        // sits left of the 3 native caption buttons + pr_2 gap; on macOS the
        // buttons are on the left side, so only the gap applies.
        #[cfg(not(target_os = "macos"))]
        let pop_right = CAPTION_W * 3.0 + 8.0;
        #[cfg(target_os = "macos")]
        let pop_right = 8.0;

        // Card: zeron popover_card — r12, 4px inset, 2px row gap, hairline
        // border, clipped so row washes don't poke out of the corners.
        let mut panel = div()
            .absolute()
            .right(px(pop_right))
            .top(px(4.))
            .min_w(px(200.))
            .p_1()
            .flex()
            .flex_col()
            .gap(px(2.))
            .rounded(px(12.))
            .border_1()
            .border_color(c(BORDER()))
            .bg(c(POPOVER()))
            .overflow_hidden()
            .shadow_lg();

        let mut idx = 0usize;
        let mut sec = div()
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(section("Сортировка"));
        for (v, l) in [
            (SortKey::Default, "По умолчанию"),
            (SortKey::Date, "По дате"),
            (SortKey::Priority, "По приоритету"),
            (SortKey::Title, "По названию"),
        ] {
            sec = sec.child(item(
                self,
                window,
                cx,
                idx,
                l,
                opts.sort == v,
                OptAct::Sort(v),
            ));
            idx += 1;
        }
        panel = panel.child(sec);
        // zeron menu_section: hairline runs edge-to-edge of the card inset.
        let mut sec = div()
            .flex()
            .flex_col()
            .gap(px(2.))
            .mt_1()
            .pt_1()
            .border_t_1()
            .border_color(rgba(FG(), 0.06))
            .child(section("Группировка"));
        for (v, l) in [
            (GroupKey::None, "Нет"),
            (GroupKey::Project, "По проекту"),
            (GroupKey::Date, "По дате"),
        ] {
            sec = sec.child(item(
                self,
                window,
                cx,
                idx,
                l,
                opts.group == v,
                OptAct::Group(v),
            ));
            idx += 1;
        }
        panel = panel.child(sec);

        div()
            .absolute()
            .inset_0()
            // Modal: dismiss presses must not also activate page content
            // underneath the backdrop.
            .occlude()
            .child(
                div()
                    .id("opt-backdrop")
                    .size_full()
                    .on_mouse_down(MouseButton::Left, {
                        let weak = cx.weak_entity();
                        move |_: &MouseDownEvent, _, cx| {
                            let _ = weak.update(cx, |this, _| this.options_for = None);
                        }
                    }),
            )
            .child(deferred(
                panel.on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation()),
            ))
            .into_any_element()
    }

    /// One dropdown row: h28 px8 r5, check icon when selected; a click closes
    /// the panel and runs the menu action.
    pub(crate) fn dd_row(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        i: usize,
        label: &str,
        checked: bool,
        act: MenuAction,
    ) -> gpui::Stateful<gpui::Div> {
        let hid = format!("dd-{i}-{label}");
        let t = self.hover_t(window, &hid);
        let key = SharedString::from(hid.clone());
        let weak = cx.weak_entity();
        div()
            .id(SharedString::from(format!("el-{i}-{label}")))
            .debug_selector(|| format!("dd-{i}"))
            .h(px(28.))
            .w_full()
            .px_2()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .rounded(px(5.))
            .text_size(crate::theme::text_px(13.))
            .text_color(c(FG()))
            .bg(fg_mix(0.06 * t))
            .child(label.to_string())
            .when(checked, |el| {
                el.child(icon("icons/check.svg", 14., c(MUTED_FG())))
            })
            .on_hover({
                let weak = weak.clone();
                move |hovered, _, cx| {
                    let _ = weak.update(cx, |this, _| this.set_hover(&key, *hovered));
                }
            })
            .on_click(move |_: &ClickEvent, _, cx| {
                let _ = weak.update(cx, |this, _| {
                    this.dropdown = None;
                    this.run_menu_action(act.clone());
                });
            })
            .aria_selected(checked)
            .a11y_menu_item(label.to_string())
    }
}

#[derive(Clone, Copy)]
enum OptAct {
    Sort(SortKey),
    Group(GroupKey),
}
