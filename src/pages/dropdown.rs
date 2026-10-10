use super::*;

impl Agenda {
    // ==========================================================================
    // Property dropdown (task-prop chips) — white panel at click point.
    // ==========================================================================

    pub(crate) fn render_dropdown(
        &mut self,
        drop: &DropState,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let weak = cx.weak_entity();
        let mut rows: Vec<gpui::Stateful<gpui::Div>> = vec![];
        let tid = drop.todo_id.clone().unwrap_or_default();

        let todo = self.todos.iter().find(|t| t.id == tid).cloned();
        match drop.kind {
            DropKind::Project => {
                let cur = todo.as_ref().and_then(|t| t.project_id.as_deref());
                rows.push(self.dd_row(
                    window,
                    cx,
                    0,
                    "Без проекта",
                    cur.is_none(),
                    MenuAction::SetProject(tid.clone(), None),
                ));
                for (i, p) in self.projects.clone().iter().enumerate() {
                    if p.status == 2 {
                        continue;
                    }
                    rows.push(self.dd_row(
                        window,
                        cx,
                        i + 1,
                        &p.title,
                        cur == Some(p.id.as_str()),
                        MenuAction::SetProject(tid.clone(), Some(p.id.to_string())),
                    ));
                }
            }
            DropKind::Date => {
                // Dorofeev weeks — «эта неделя» is a flag, «следующая» is a
                // real Monday so the task can surface in «Планы».
                let day = LocalDay::now();
                let next_mon = day.next_monday_key();
                let cur = todo.as_ref().and_then(|t| task_date(t, &day).0);
                let this_week = todo.as_ref().is_some_and(|t| t.is_today) && cur.is_none();
                rows.push(self.dd_row(
                    window,
                    cx,
                    0,
                    "Эта неделя",
                    this_week,
                    MenuAction::MoveToWeek(tid.clone()),
                ));
                rows.push(self.dd_row(
                    window,
                    cx,
                    1,
                    "Следующая неделя",
                    cur == next_mon,
                    MenuAction::SetDate(tid.clone(), next_mon),
                ));
                rows.push(self.dd_row(
                    window,
                    cx,
                    2,
                    "Без даты",
                    cur.is_none() && !this_week,
                    MenuAction::SetDate(tid.clone(), None),
                ));
            }
            DropKind::QeDate => {
                // Quick entry has no owning todo — the draft date lives on
                // `qe_date`/`qe_someday`. «Потом» defers to the someday list,
                // not a calendar date.
                let day = LocalDay::now();
                rows.push(self.dd_row(
                    window,
                    cx,
                    0,
                    "Эта неделя",
                    self.qe_week,
                    MenuAction::QeSetWeek,
                ));
                rows.push(self.dd_row(
                    window,
                    cx,
                    1,
                    "Следующая неделя",
                    self.qe_date.is_some() && self.qe_date == day.next_monday_key(),
                    MenuAction::QeSetDate(day.next_monday_key()),
                ));
                rows.push(self.dd_row(
                    window,
                    cx,
                    2,
                    "Потом",
                    self.qe_someday,
                    MenuAction::QeSetSomeday,
                ));
                rows.push(self.dd_row(
                    window,
                    cx,
                    3,
                    "Без даты",
                    !self.qe_someday && !self.qe_week && self.qe_date.is_none(),
                    MenuAction::QeSetDate(None),
                ));
            }
            DropKind::QeProject => {
                rows.push(self.dd_row(
                    window,
                    cx,
                    0,
                    "Без проекта",
                    self.qe_project.is_none(),
                    MenuAction::QeSetProject(None),
                ));
                for (i, p) in self.projects.clone().iter().enumerate() {
                    if p.status == 2 {
                        continue;
                    }
                    rows.push(self.dd_row(
                        window,
                        cx,
                        i + 1,
                        &p.title,
                        self.qe_project.as_deref() == Some(p.id.as_str()),
                        MenuAction::QeSetProject(Some(p.id.to_string())),
                    ));
                }
            }
        }

        // `drop.x/y` are window coordinates captured from the chip's click
        // event; this overlay is mounted at the window root (app.rs), so the
        // panel anchors at the press point — unless that would spill it off
        // the window, in which case it clamps back inside (the quick-entry
        // chips now live in the right-docked panel, so a naive anchor would
        // push the dropdown past the right edge).
        let vw = f32::from(window.viewport_size().width);
        let vh = f32::from(window.viewport_size().height);
        let x = drop.x.min((vw - 168.0).max(8.0));
        let y = (drop.y + 4.0).min((vh - 320.0 - 8.0).max(8.0));
        let panel = div()
            .absolute()
            .left(px(x))
            .top(px(y))
            .min_w(px(160.))
            .max_h(px(320.))
            .id("dropdown-panel")
            .debug_selector(|| "dd-panel".to_string())
            // Keep inside presses off the backdrop (and any overlay below):
            // row clicks close the dropdown via their own `on_click`.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .overflow_y_scroll()
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
            }])
            .children(rows);

        div()
            .absolute()
            .inset_0()
            // Modal: swallow presses that miss the panel instead of letting
            // them reach overlays or page content underneath.
            .occlude()
            .child(
                div()
                    .id("dd-backdrop")
                    .size_full()
                    .on_mouse_down(MouseButton::Left, {
                        let weak = weak.clone();
                        move |_: &MouseDownEvent, _, cx| {
                            let _ = weak.update(cx, |this, _| this.dropdown = None);
                        }
                    })
                    .on_mouse_down(MouseButton::Right, move |_: &MouseDownEvent, _, cx| {
                        let _ = weak.update(cx, |this, _| this.dropdown = None);
                    }),
            )
            .child(deferred(panel))
            .into_any_element()
    }
}
