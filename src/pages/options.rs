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

        let section = |title: &str| {
            div()
                .pt_1()
                .pb(px(2.))
                .px_2()
                .text_size(px(11.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
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
                .h(px(26.))
                .w_full()
                .px_2()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .rounded(px(5.))
                .text_size(px(13.))
                .text_color(c(FG()))
                .bg(fg_mix(0.06 * t))
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
                            OptAct::View(v) => o.view = v,
                            OptAct::Sort(s) => o.sort = s,
                            OptAct::Group(g) => o.group = g,
                        }
                        this.boards.insert(key3, o);
                        this.options_for = None;
                    });
                })
        };

        // Right edge of the options button in the titlebar: on Windows it
        // sits left of the 3 native caption buttons + pr_2 gap; on macOS the
        // buttons are on the left side, so only the gap applies.
        #[cfg(not(target_os = "macos"))]
        let pop_right = CAPTION_W * 3.0 + 8.0;
        #[cfg(target_os = "macos")]
        let pop_right = 8.0;

        let mut panel = div()
            .absolute()
            .right(px(pop_right))
            .top(px(4.))
            .min_w(px(200.))
            .p_1()
            .flex()
            .flex_col()
            .rounded_lg()
            .border_1()
            .border_color(c(BORDER()))
            .bg(c(POPOVER()))
            .shadow(vec![gpui::BoxShadow {
                color: rgba(0x000000, 0.12),
                offset: gpui::point(px(0.), px(8.)),
                blur_radius: px(24.),
                spread_radius: px(0.),
                inset: false,
            }]);

        let mut idx = 0usize;
        let mut sec = div().flex().flex_col().child(section("Вид"));
        for (v, l) in [(BoardView::List, "Список"), (BoardView::Kanban, "Доска")] {
            sec = sec.child(item(
                self,
                window,
                cx,
                idx,
                l,
                opts.view == v,
                OptAct::View(v),
            ));
            idx += 1;
        }
        panel = panel.child(sec);
        let mut sec = div()
            .flex()
            .flex_col()
            .mt_1()
            .pt_1()
            .border_t_1()
            .border_color(border_mix(0.6))
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
        if opts.view == BoardView::List {
            let mut sec = div()
                .flex()
                .flex_col()
                .mt_1()
                .pt_1()
                .border_t_1()
                .border_color(border_mix(0.6))
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
        }

        div()
            .absolute()
            .inset_0()
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
            .child(deferred(panel))
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
            .text_size(px(13.))
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
    }
}

#[derive(Clone, Copy)]
enum OptAct {
    View(BoardView),
    Sort(SortKey),
    Group(GroupKey),
}
