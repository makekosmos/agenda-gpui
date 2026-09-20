use super::*;

/// Flattened item of a virtualized task list. Every variant renders into the
/// same 37px slot (36px row + 1px border) so `uniform_list` heights match.
pub(crate) enum FlatRow {
    /// Group caption ("Проект · N") padded to the row slot.
    Header(String, usize),
    /// Index into `Agenda::todos` — no per-frame clones, the row resolves the
    /// todo only when it scrolls into the visible range.
    Task(usize),
    /// Logbook's archived-project row (same 37px chrome as task rows).
    ArchivedProject(Project),
}

/// Flat-row cache: the expensive filter/sort/group pipeline reruns only when
/// the list, view options or `model_rev` change — scrolling and idle frames
/// reuse the same `Rc` and allocate nothing.
pub(crate) struct RowCache {
    pub list: SmartList,
    pub group: GroupKey,
    pub sort: SortKey,
    pub rev: u64,
    pub rows: std::rc::Rc<Vec<FlatRow>>,
}

/// Group caption sharing the task-row slot: bottom-aligned so the text sits
/// where the old `pt-14 pb-1` block put it.
fn list_header(label: &str, count: usize) -> gpui::Div {
    div()
        .h(px(37.))
        .flex_none()
        .flex()
        .flex_col()
        .justify_end()
        .pb(px(6.))
        .px_7()
        .text_size(px(12.))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(c(MUTED_FG()))
        .child(format!("{label} · {count}"))
}

impl Agenda {
    /// Virtualized task list shared by board/project/logbook/trash pages.
    /// Only the visible range is rendered; scroll position is kept in the
    /// page's regular `ScrollHandle` (wrapped in a UniformListScrollHandle).
    pub(crate) fn task_list(
        &mut self,
        id: String,
        scroll_key: &str,
        rows: std::rc::Rc<Vec<FlatRow>>,
        editable: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        // Persistent handle: keeps last_item_size/deferred_scroll_to_item
        // across frames instead of dropping them every render.
        let handle = self.list_scroll(scroll_key);
        gpui::uniform_list(
            SharedString::from(id),
            rows.len(),
            cx.processor(move |this, range: std::ops::Range<usize>, window, cx| {
                // Move the Vec out for the duration of row building: task_row
                // needs &mut self (hover state) while also borrowing a Todo —
                // mem::take makes that legal without cloning ~25 rows/frame.
                // Nothing in the row path reads this.todos back.
                let rows_t0 = std::time::Instant::now();
                let todos = std::mem::take(&mut this.todos);
                let today = today_key();
                let out: Vec<_> = range
                    .map(|ix| match &rows[ix] {
                        FlatRow::Header(label, count) => {
                            list_header(label, *count).into_any_element()
                        }
                        FlatRow::Task(ix) => match todos.get(*ix) {
                            Some(t) => this
                                .task_row(t, *ix, &today, editable, window, cx)
                                .into_any_element(),
                            None => div().into_any_element(),
                        },
                        FlatRow::ArchivedProject(p) => {
                            this.archived_project_row(p, window, cx).into_any_element()
                        }
                    })
                    .collect();
                this.todos = todos;
                this.rows_ms = rows_t0.elapsed().as_secs_f32() * 1000.0;
                out
            }),
        )
        .flex_1()
        .track_scroll(&handle)
        .into_any_element()
    }

    // ==========================================================================
    // Shared: task row (.task-row) — h36 px8 gap8 border-b, fs13
    // ==========================================================================

    pub(crate) fn task_row(
        &mut self,
        t: &Todo,
        ix: usize,
        today: &str,
        editable: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let id = t.id.to_string();
        let status = task_status(t);
        let done = status == Status::Done || status == Status::Canceled;
        let hid = format!("row-{}", t.id);
        let t_h = self.hover_t(window, &hid);
        let (date_val, _) = task_date(t);
        let overdue = date_val.as_deref().is_some_and(|d| d < today) && !done;

        let mut row = div()
            .id(("tr", ix))
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
                    .id(("trs", ix))
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

        // hover actions: "На сегодня" (overdue) + status chips (not done);
        // the container itself is skipped when empty — one less element/row.
        if editable && !t.is_trashed && (overdue || !done) {
            let mut actions = div()
                .flex_none()
                .flex()
                .items_center()
                .gap_0p5()
                .opacity(t_h);
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
                actions = actions.child(self.row_status_chip(t, ix, status, window, cx));
            }
            row = row.child(actions);
        }

        // meta: checklist chip, tag pills, project, date, price — skipped
        // entirely when the task has none of them.
        let total = t.checklist.len();
        let has_meta = total > 0
            || !t.tag_ids.is_empty()
            || t.project_id.is_some()
            || date_val.is_some()
            || t.billable;
        if has_meta {
            let mut meta = div()
                .flex_none()
                .flex()
                .items_center()
                .gap_2()
                .text_size(px(12.))
                .text_color(c(MUTED_FG()));
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
        }

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
