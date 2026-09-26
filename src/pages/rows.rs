use super::*;

mod task;

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
            .a11y_button(label.to_string())
    }
}
