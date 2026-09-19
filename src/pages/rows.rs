use super::*;

impl Agenda {
    // ==========================================================================
    // Shared: task row (.task-row) — h36 px8 gap8 border-b, fs13
    // ==========================================================================

    pub(crate) fn task_row(
        &mut self,
        t: &Todo,
        editable: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let id = t.id.to_string();
        let status = task_status(t);
        let done = status == Status::Done || status == Status::Canceled;
        let hid = format!("row-{}", t.id);
        let t_h = self.hover_t(window, &hid);
        let today = today_key();
        let (date_val, _) = task_date(t);
        let overdue = date_val.as_deref().is_some_and(|d| d < today.as_str()) && !done;

        let mut row = div()
            .id(SharedString::from(format!("tr-{}", t.id)))
            .min_h(px(36.))
            .w_full()
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .border_b_1()
            .border_color(border_mix(0.55))
            .text_size(px(13.))
            .line_height(px(15.))
            .text_color(c(FG()))
            .bg(fg_mix(0.04 * t_h))
            // status button (20px grid)
            .child(
                div()
                    .id(SharedString::from(format!("trs-{}", t.id)))
                    .w_5()
                    .h_5()
                    .flex_none()
                    .grid()
                    .items_center()
                    .justify_center()
                    .child(status_ring(status))
                    .on_click({
                        let weak = cx.weak_entity();
                        let id2 = id.clone();
                        move |_: &ClickEvent, _, cx| {
                            let _ = weak.update(cx, |this, _| {
                                this.run_menu_action(MenuAction::CompleteTodo(id2.clone()));
                            });
                        }
                    }),
            )
            .child(
                div()
                    .w_4()
                    .h_4()
                    .flex_none()
                    .grid()
                    .items_center()
                    .justify_center()
                    .child(priority_bars(t.priority)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_color(if done { c(MUTED_FG()) } else { c(FG()) })
                    .child(t.title.clone()),
            );

        // hover actions: "На сегодня" (overdue) + status chips (not done)
        let mut actions = div()
            .flex_none()
            .flex()
            .items_center()
            .gap_0p5()
            .opacity(t_h);
        if editable && !t.is_trashed {
            if overdue {
                actions = actions.child(self.row_action(
                    &format!("ra-today-{}", t.id),
                    "На сегодня",
                    MenuAction::MoveToToday(id.clone()),
                    t_h,
                    window,
                    cx,
                ));
            }
            if !done {
                actions = actions.child(self.row_status_chip(t, status, window, cx));
            }
        }
        row = row.child(actions);

        // meta: checklist chip, tag pills, project, date, price
        let mut meta = div()
            .flex_none()
            .flex()
            .items_center()
            .gap_2()
            .text_size(px(12.))
            .text_color(c(MUTED_FG()));
        let total = t.checklist.len();
        if total > 0 {
            let d = t.checklist.iter().filter(|i| i.is_completed).count();
            meta = meta.child(format!("{}/{}", d, total));
        }
        for tag_id in &t.tag_ids {
            if let Some(tag) = self.tag(tag_id) {
                let dot = tag_color(tag.color);
                meta = meta.child(
                    div()
                        .h_5()
                        .px_2()
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .rounded_full()
                        .bg(fg_mix(0.06))
                        .text_size(px(11.))
                        .text_color(c(FG()))
                        .child(div().w(px(6.)).h(px(6.)).rounded_full().bg(c(dot)))
                        .child(tag.title.clone()),
                );
            }
        }
        if let Some(pid) = t.project_id {
            if let Some(p) = self.project(pid) {
                meta = meta.child(
                    div()
                        .max_w(px(140.))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .child(p.title.clone()),
                );
            }
        }
        if let Some(d) = &date_val {
            meta = meta.child(
                div()
                    .text_color(if overdue {
                        c(DESTRUCTIVE())
                    } else {
                        c(MUTED_FG())
                    })
                    .child(fmt_day_month(d)),
            );
        }
        if t.billable {
            meta = meta.child(format!("${}", t.price.unwrap_or(0)));
        }
        row = row.child(meta);

        let weak = cx.weak_entity();
        row.on_hover({
            let weak = weak.clone();
            move |hovered, _, cx| {
                let _ = weak.update(cx, |this, _| {
                    this.set_hover(&format!("row-{}", id), *hovered)
                });
            }
        })
        .on_click({
            let weak = weak.clone();
            let id = t.id.to_string();
            move |_: &ClickEvent, _, cx| {
                let _ = weak.update(cx, |this, _| this.navigate(Route::Task(id.clone())));
            }
        })
        .on_mouse_down(MouseButton::Right, {
            let tid = t.id.to_string();
            let trashed = t.is_trashed;
            move |ev: &MouseDownEvent, _, cx| {
                let pos = ev.position;
                let tid = tid.clone();
                let _ = weak.update(cx, |this, _| {
                    this.menu = Some(CtxMenu {
                        x: pos.x.into(),
                        y: pos.y.into(),
                        items: if trashed {
                            vec![
                                (
                                    "Восстановить".into(),
                                    false,
                                    MenuAction::RestoreTodo(tid.clone()),
                                ),
                                ("Удалить".into(), true, MenuAction::Noop),
                            ]
                        } else {
                            vec![("Удалить".into(), true, MenuAction::TrashTodo(tid.clone()))]
                        },
                    });
                });
            }
        })
    }

    pub(crate) fn row_action(
        &mut self,
        hid: &str,
        label: &str,
        action: MenuAction,
        row_t: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let t = self.hover_t(window, hid);
        let weak = cx.weak_entity();
        let key = SharedString::from(hid.to_string());
        div()
            .id(SharedString::from(format!("el-{hid}")))
            .h(px(20.))
            .px_2()
            .flex()
            .items_center()
            .rounded(px(5.))
            .text_size(px(12.))
            .text_color(mix(FG(), 0.45 + 0.55 * t, BG()))
            .bg(fg_mix(0.07 * t))
            .opacity(row_t.max(0.001))
            .child(label.to_string())
            .on_hover({
                let weak = weak.clone();
                move |hovered, _, cx| {
                    let _ = weak.update(cx, |this, _| this.set_hover(&key, *hovered));
                }
            })
            .on_click({
                let weak = cx.weak_entity();
                move |_: &ClickEvent, _, cx| {
                    let _ = weak.update(cx, |this, _| {
                        this.run_menu_action(action.clone());
                    });
                }
            })
    }
}
