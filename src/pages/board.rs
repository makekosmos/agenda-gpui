use super::*;

impl Agenda {
    // ==========================================================================
    // TaskBoard parity: options row + list groups + kanban
    // ==========================================================================

    pub(crate) fn board_page(
        &mut self,
        list: SmartList,
        storage: &str,
        fab_hint: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let _ = fab_hint;
        let opts = self.board_opts(storage);
        let items = filter_todos(list, &self.todos);
        let items = sorted(&items, opts.sort);

        let mut body = div().flex_1().min_h_0().flex().flex_col().overflow_hidden();

        let scroll = self.scroll(storage);
        if opts.view == BoardView::Kanban {
            body = body.child(self.kanban(&items, window, cx));
        } else {
            // grouping
            let groups: Vec<(Option<String>, Vec<Todo>)> = match opts.group {
                GroupKey::None => vec![(None, items)],
                GroupKey::Project => {
                    let mut g: Vec<(Option<String>, Vec<Todo>)> = vec![];
                    let mut no_proj: Vec<Todo> = vec![];
                    for p in &self.projects {
                        let bucket: Vec<Todo> = items
                            .iter()
                            .filter(|t| t.project_id == Some(p.id))
                            .cloned()
                            .collect();
                        if !bucket.is_empty() {
                            g.push((Some(p.title.clone()), bucket));
                        }
                    }
                    for t in &items {
                        if t.project_id.is_none() {
                            no_proj.push(t.clone());
                        }
                    }
                    if !no_proj.is_empty() {
                        g.insert(0, (Some("Без проекта".to_string()), no_proj));
                    }
                    // unknown ids → "Проект"
                    let orphan: Vec<Todo> = items
                        .iter()
                        .filter(|t| {
                            t.project_id
                                .is_some_and(|pid| !self.projects.iter().any(|p| p.id == pid))
                        })
                        .cloned()
                        .collect();
                    if !orphan.is_empty() {
                        g.push((Some("Проект".into()), orphan));
                    }
                    g
                }
                GroupKey::Date => {
                    let mut keys: Vec<Option<String>> = vec![];
                    for t in &items {
                        let k = task_date(t).0;
                        if !keys.contains(&k) {
                            keys.push(k);
                        }
                    }
                    keys.sort_by(|a, b| {
                        a.clone()
                            .unwrap_or_else(|| "9999".into())
                            .cmp(&b.clone().unwrap_or_else(|| "9999".into()))
                    });
                    keys.into_iter()
                        .map(|k| {
                            (
                                Some(date_group_label(k.as_deref())),
                                items
                                    .iter()
                                    .filter(|t| task_date(t).0 == k)
                                    .cloned()
                                    .collect::<Vec<Todo>>(),
                            )
                        })
                        .collect()
                }
            };

            let mut list_el = div()
                .id(SharedString::from(format!("list-{storage}")))
                .flex_1()
                .min_h_0()
                .flex()
                .flex_col()
                .overflow_y_scroll()
                .track_scroll(&scroll);
            let mut any = false;
            for (label, gitems) in groups {
                if gitems.is_empty() {
                    continue;
                }
                any = true;
                if let Some(l) = label {
                    list_el = list_el.child(
                        div()
                            .pt(px(14.))
                            .pb_1()
                            .px_7()
                            .text_size(px(12.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(c(MUTED_FG()))
                            .child(format!("{} · {}", l, gitems.len())),
                    );
                }
                for t in &gitems {
                    list_el = list_el.child(self.task_row(t, true, window, cx));
                }
            }
            if !any {
                list_el = list_el.child(empty_state());
            }
            body = body.child(list_el);
        }
        body.into_any_element()
    }

    pub(crate) fn kanban(
        &mut self,
        items: &[Todo],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let cols: [(Status, &str); 3] = [
            (Status::Todo, "К выполнению"),
            (Status::Started, "В работе"),
            (Status::Done, "Готово"),
        ];
        let mut grid = div()
            .flex_1()
            .min_h(px(192.))
            .grid()
            .grid_cols(3)
            .gap_3()
            .px_7()
            .overflow_hidden();
        for (st, label) in cols {
            let col_items: Vec<Todo> = items
                .iter()
                .filter(|t| {
                    let s = task_status(t);
                    if st == Status::Todo {
                        s == Status::Todo || s == Status::Inbox
                    } else {
                        s == st
                    }
                })
                .cloned()
                .collect();
            let mut col = div()
                .rounded_lg()
                .border_1()
                .border_color(c(BORDER()))
                .bg(rgba(SECONDARY(), 0.3))
                .p_2()
                .flex()
                .flex_col()
                .child(
                    div()
                        .mb_2()
                        .px_2()
                        .text_size(px(12.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(c(MUTED_FG()))
                        .child(format!("{} · {}", label, col_items.len())),
                );
            let mut cards = div().flex().flex_col().gap_2();
            for t in &col_items {
                let hid = format!("kb-{}", t.id);
                let ht = self.hover_t(window, &hid);
                let weak = cx.weak_entity();
                let tid = t.id.to_string();
                let key = SharedString::from(hid.clone());
                let mut card = div()
                    .id(SharedString::from(format!("el-{hid}")))
                    .rounded_md()
                    .border_1()
                    .border_color(lerp(BORDER(), ACCENT(), ht))
                    .bg(c(CARD()))
                    .p_3()
                    .text_left()
                    .text_size(px(13.))
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .child(t.title.clone()),
                    );
                if let Some(n) = &t.notes {
                    card = card.child(
                        div()
                            .mt_1()
                            .text_size(px(12.))
                            .text_color(c(MUTED_FG()))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .child(n.clone()),
                    );
                }
                cards = cards.child(
                    card.on_hover(move |hovered, _, cx| {
                        let _ = weak.update(cx, |this, _| this.set_hover(&key, *hovered));
                    })
                    .on_click({
                        let weak = cx.weak_entity();
                        let tid = tid.clone();
                        move |_: &ClickEvent, _, cx| {
                            let _ =
                                weak.update(cx, |this, _| this.navigate(Route::Task(tid.clone())));
                        }
                    }),
                );
            }
            col = col.child(cards);
            grid = grid.child(col);
        }
        grid.into_any_element()
    }
}
