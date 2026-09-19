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
            DropKind::Status => {
                let cur = todo.as_ref().map(task_status);
                for (i, (s, l)) in [
                    (Status::Inbox, "Входящие"),
                    (Status::Todo, "Сделать"),
                    (Status::Started, "В работе"),
                    (Status::Deferred, "Потом"),
                    (Status::Done, "Готово"),
                    (Status::Canceled, "Отменено"),
                ]
                .iter()
                .enumerate()
                {
                    rows.push(self.dd_row(
                        window,
                        cx,
                        i,
                        l,
                        cur == Some(*s),
                        MenuAction::SetStatus(tid.clone(), *s),
                    ));
                }
            }
            DropKind::Priority => {
                let cur = todo.as_ref().map(|t| t.priority).unwrap_or(0);
                for (i, (p, l)) in [
                    (0u8, "Без приоритета"),
                    (1, "Низкий"),
                    (2, "Средний"),
                    (3, "Высокий"),
                ]
                .iter()
                .enumerate()
                {
                    rows.push(self.dd_row(
                        window,
                        cx,
                        i,
                        l,
                        cur == *p,
                        MenuAction::SetPriority(tid.clone(), *p),
                    ));
                }
            }
            DropKind::Project => {
                let cur = todo.as_ref().and_then(|t| t.project_id);
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
                        cur == Some(p.id),
                        MenuAction::SetProject(tid.clone(), Some(p.id.to_string())),
                    ));
                }
            }
            DropKind::Tags => {
                let cur: Vec<&'static str> =
                    todo.as_ref().map(|t| t.tag_ids.clone()).unwrap_or_default();
                for (i, tag) in self.tags.clone().iter().enumerate() {
                    rows.push(self.dd_row(
                        window,
                        cx,
                        i,
                        &tag.title,
                        cur.contains(&tag.id),
                        MenuAction::AddTag(tid.clone(), tag.id.to_string()),
                    ));
                }
            }
            DropKind::Significance => {
                let cur = todo.as_ref().and_then(|t| t.significance);
                rows.push(self.dd_row(
                    window,
                    cx,
                    0,
                    "Не оценено",
                    cur.is_none(),
                    MenuAction::SetSignificance(tid.clone(), None),
                ));
                for v in 1..=10u8 {
                    rows.push(self.dd_row(
                        window,
                        cx,
                        v as usize,
                        &format!("{}/10", v),
                        cur == Some(v),
                        MenuAction::SetSignificance(tid.clone(), Some(v)),
                    ));
                }
            }
            DropKind::Billable => {
                let cur = todo.as_ref().map(|t| t.billable).unwrap_or(false);
                rows.push(self.dd_row(
                    window,
                    cx,
                    0,
                    "Без оплаты",
                    !cur,
                    MenuAction::SetBillable(tid.clone(), false),
                ));
                rows.push(self.dd_row(
                    window,
                    cx,
                    1,
                    "Оплачиваемая",
                    cur,
                    MenuAction::SetBillable(tid.clone(), true),
                ));
            }
            DropKind::Date | DropKind::QeDate => {
                let today = today_key();
                let tomorrow = day_key(1);
                let week = day_key(7);
                let cur = todo.as_ref().and_then(|t| task_date(t).0);
                let opts: [(Option<String>, &str); 4] = [
                    (Some(today), "Сегодня"),
                    (Some(tomorrow), "Завтра"),
                    (Some(week), "Через неделю"),
                    (None, "Без даты"),
                ];
                for (i, (d, l)) in opts.iter().enumerate() {
                    let act = if drop.kind == DropKind::QeDate {
                        MenuAction::QeSetDate(d.clone())
                    } else {
                        MenuAction::SetDate(tid.clone(), d.clone())
                    };
                    rows.push(self.dd_row(window, cx, i, l, cur == *d, act));
                }
            }
            DropKind::QeProject => {
                rows.push(self.dd_row(
                    window,
                    cx,
                    0,
                    "Входящие",
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
                        self.qe_project.as_deref() == Some(p.id),
                        MenuAction::QeSetProject(Some(p.id.to_string())),
                    ));
                }
            }
            DropKind::RecurFreq => {
                for (i, (v, l)) in [(0u8, "День"), (1, "Неделя"), (2, "Месяц"), (3, "Год")]
                    .iter()
                    .enumerate()
                {
                    rows.push(self.dd_row(
                        window,
                        cx,
                        i,
                        l,
                        self.recur_freq == *v,
                        MenuAction::RecurSetFreq(*v),
                    ));
                }
            }
            DropKind::RecurType => {
                for (i, (v, l)) in [(0u8, "по расписанию"), (1u8, "после выполнения")]
                    .iter()
                    .enumerate()
                {
                    rows.push(self.dd_row(
                        window,
                        cx,
                        i,
                        l,
                        self.recur_type == *v,
                        MenuAction::RecurSetType(*v),
                    ));
                }
            }
        }

        let x: f32 = drop.x;
        let y: f32 = drop.y;
        let panel = div()
            .absolute()
            .left(px(x))
            .top(px(y + 4.))
            .min_w(px(160.))
            .max_h(px(320.))
            .id("dropdown-panel")
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
