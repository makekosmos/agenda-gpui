// Page renderers + shared task widgets (TaskRow/TaskBoard parity),
// quick search / quick entry / property dropdown overlays.
use gpui::{
    AnyElement, ClickEvent, Context, MouseButton, MouseDownEvent, SharedString, Window, deferred,
    div, prelude::*, px,
};
use gpui_component::input::Input;

use chrono::{Datelike, Duration, Local};

use crate::app::{Agenda, BoardView, CalMode, CtxMenu, DropKind, DropState, MenuAction, Route};
use crate::model::*;
use crate::theme::*;
use crate::widgets::*;

const PANEL: u32 = 0x121212; // dark overlay panels (quick search / quick entry)
const PANEL_FG: u32 = 0xe5e5e5;
const PANEL_MUTED: u32 = 0x8a8a8a;
const PANEL_BORDER: u32 = 0x2a2a2a;
const QE_CHIP_BG: u32 = 0x22346b; // quick-entry selected date chip
const QE_CHIP_FG: u32 = 0x86a5ff;

fn panel_mix(a: f32) -> HslaAlias {
    mix(0xffffff, a, PANEL)
}

// Alias so we can write Hsla without importing gpui::Hsla in every signature.
pub type HslaAlias = gpui::Hsla;

impl Agenda {
    // ==========================================================================
    // Page dispatch
    // ==========================================================================

    pub(crate) fn render_page(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let route = self.route.clone();
        let content: AnyElement = match route {
            Route::Inbox => self.board_page(SmartList::Inbox, "agenda.inbox.view", true, window, cx),
            Route::Today => self.board_page(SmartList::Today, "agenda.today.view", true, window, cx),
            Route::Plans => self.board_page(SmartList::Plans, "agenda.plans.view", false, window, cx),
            Route::Someday => {
                self.board_page(SmartList::Someday, "agenda.someday.view", false, window, cx)
            }
            Route::Logbook => self.logbook_page(window, cx),
            Route::Trash => self.trash_page(window, cx),
            Route::Calendar => self.calendar_page(window, cx),
            Route::Statistics => self.statistics_page(window, cx),
            Route::Recurring => self.recurring_page(window, cx),
            Route::Settings => self.settings_page(false, window, cx),
            Route::SettingsFuel => self.settings_page(true, window, cx),
            Route::About => self.about_page(),
            Route::Project(ref id) => self.project_page(id, window, cx),
            Route::Task(ref id) => self.task_page(id, window, cx),
        };

        let mut page = div()
            .flex_1()
            .min_h_0()
            .relative()
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(content);

        // Primary FAB (+ dev vars stub) on list pages.
        if matches!(
            route,
            Route::Inbox | Route::Today | Route::Plans | Route::Someday | Route::Project(_)
        ) {
            page = page.child(self.render_fab(window, cx));
        }
        // Display-options popover is anchored top-right of the content area.
        if self.options_for.is_some() {
            page = page.child(self.render_options_popover(window, cx).into_any_element());
        }
        if let Some(drop) = self.dropdown.clone() {
            page = page.child(self.render_dropdown(&drop, window, cx).into_any_element());
        }
        page.into_any_element()
    }

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
        let overdue = date_val.as_deref().map_or(false, |d| d < today.as_str()) && !done;

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
            .text_color(c(FG))
            .bg(fg_mix(0.04 * t_h))
            // status button (20px grid)
            .child(
                div()
                    .id(SharedString::from(format!("trs-{}", t.id)))
                    .w_5()
                    .h_5()
                    .flex_none()
                    .grid()
                    .items_center().justify_center()
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
                    .items_center().justify_center()
                    .child(priority_bars(t.priority)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_color(if done { c(MUTED_FG) } else { c(FG) })
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
            .text_color(c(MUTED_FG));
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
                        .text_color(c(FG))
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
                    .text_color(if overdue { c(DESTRUCTIVE) } else { c(MUTED_FG) })
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
                                ("Восстановить".into(), false, MenuAction::RestoreTodo(tid.clone())),
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

    fn row_action(
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
            .text_color(mix(FG, 0.45 + 0.55 * t, BG))
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

    /// Status select chip on rows (task-row__actions select parity).
    fn row_status_chip(
        &mut self,
        t: &Todo,
        status: Status,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let label = match status {
            Status::Started => "В работе",
            Status::Deferred => "Потом",
            _ => "К выполнению",
        };
        let hid = format!("ra-status-{}", t.id);
        let ht = self.hover_t(window, &hid);
        let weak = cx.weak_entity();
        let key = SharedString::from(hid.clone());
        let tid = t.id.to_string();
        div()
            .id(SharedString::from(format!("el-{hid}")))
            .h(px(20.))
            .px_2()
            .flex()
            .items_center()
            .rounded(px(5.))
            .bg(c(SECONDARY))
            .text_size(px(12.))
            .text_color(mix(FG, 0.7 + 0.3 * ht, BG))
            .child(label)
            .on_hover({
                let weak = weak.clone();
                move |hovered, _, cx| {
                    let _ = weak.update(cx, |this, _| this.set_hover(&key, *hovered));
                }
            })
            .on_click(move |ev: &ClickEvent, _, cx| {
                let pos = ev.position();
                let tid = tid.clone();
                let _ = weak.update(cx, |this, _| {
                    this.dropdown = Some(DropState {
                        kind: DropKind::Status,
                        x: pos.x.into(),
                        y: pos.y.into(),
                        todo_id: Some(tid),
                    });
                });
            })
    }

    // ==========================================================================
    // TaskBoard parity: options row + list groups + kanban
    // ==========================================================================

    fn board_page(
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
        body = body.child(self.options_row(storage, window, cx));

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
                        let bucket: Vec<Todo> =
                            items.iter().filter(|t| t.project_id == Some(p.id)).cloned().collect();
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
                                .map_or(false, |pid| !self.projects.iter().any(|p| p.id == pid))
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
                        a.clone().unwrap_or_else(|| "9999".into())
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
                            .text_color(c(MUTED_FG))
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

    fn options_row(
        &mut self,
        storage: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let t = self.hover_t(window, "opt-btn");
        let weak = cx.weak_entity();
        let key = SharedString::from(format!("opt-key-{storage}"));
        div()
            .flex_none()
            .flex()
            .justify_end()
            .px_7()
            .pb_2()
            .pt_1()
            .child(
                div()
                    .id("opt-btn")
                    .h_7()
                    .px_2()
                    .flex()
                    .items_center()
                    .rounded_md()
                    .bg(fg_mix(0.06 * t))
                    .child(icon("icons/sliders.svg", 14., c(MUTED_FG)))
                    .on_hover(move |hovered, _, cx| {
                        let _ = weak.update(cx, |this, _| this.set_hover("opt-btn", *hovered));
                    })
                    .on_click({
                        let weak = cx.weak_entity();
                        move |_: &ClickEvent, _, cx| {
                            let _ = weak.update(cx, |this, _| {
                                this.options_for = if this.options_for.is_some() {
                                    None
                                } else {
                                    Some(key.to_string())
                                };
                            });
                        }
                    }),
            )
    }

    fn kanban(
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
                .border_color(c(BORDER))
                .bg(rgba(SECONDARY, 0.3))
                .p_2()
                .flex()
                .flex_col()
                .child(
                    div()
                        .mb_2()
                        .px_2()
                        .text_size(px(12.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(c(MUTED_FG))
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
                    .border_color(lerp(BORDER, ACCENT, ht))
                    .bg(c(BG))
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
                            .text_color(c(MUTED_FG))
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

    /// .task-options popover (top-right, below the options button).
    fn render_options_popover(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                .text_color(c(MUTED_FG))
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
                .text_color(c(FG))
                .bg(fg_mix(0.06 * t))
                .child(label.to_string())
                .when(checked, |el| el.child(icon("icons/check.svg", 14., c(MUTED_FG))))
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

        let mut panel = div()
            .absolute()
            .right_7()
            .top(px(44.))
            .min_w(px(200.))
            .p_1()
            .flex()
            .flex_col()
            .rounded_lg()
            .border_1()
            .border_color(c(BORDER))
            .bg(c(BG))
            .shadow(vec![gpui::BoxShadow {
                color: rgba(0x000000, 0.12),
                offset: gpui::point(px(0.), px(8.)),
                blur_radius: px(24.),
                spread_radius: px(0.),
            }]);

        let mut idx = 0usize;
        let mut sec = div().flex().flex_col().child(section("Вид"));
        for (v, l) in [(BoardView::List, "Список"), (BoardView::Kanban, "Доска")] {
            sec = sec.child(item(self, window, cx, idx, l, opts.view == v, OptAct::View(v)));
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
            sec = sec.child(item(self, window, cx, idx, l, opts.sort == v, OptAct::Sort(v)));
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
                sec = sec.child(item(self, window, cx, idx, l, opts.group == v, OptAct::Group(v)));
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

    // ==========================================================================
    // Property dropdown (task-prop chips) — white panel at click point.
    // ==========================================================================

    fn render_dropdown(
        &mut self,
        drop: &DropState,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let weak = cx.weak_entity();
        let mut rows: Vec<gpui::Stateful<gpui::Div>> = vec![];
        let tid = drop.todo_id.clone().unwrap_or_default();

        let item = |app: &mut Self,
                    window: &mut Window,
                    cx: &mut Context<Self>,
                    i: usize,
                    label: &str,
                    checked: bool,
                    act: MenuAction| {
            let hid = format!("dd-{i}-{label}");
            let t = app.hover_t(window, &hid);
            let key = SharedString::from(hid.clone());
            let weak = cx.weak_entity();
            div()
                .id(SharedString::from(format!("el-{i}-{label}")))
                .h(px(28.))
                .w_full()
                .px_2()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .rounded(px(5.))
                .text_size(px(13.))
                .text_color(c(FG))
                .bg(fg_mix(0.06 * t))
                .child(label.to_string())
                .when(checked, |el| el.child(icon("icons/check.svg", 14., c(MUTED_FG))))
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
        };

        let todo = self.todos.iter().find(|t| t.id == tid).cloned();
        match drop.kind {
            DropKind::Status => {
                let cur = todo.as_ref().map(|t| task_status(t));
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
                    rows.push(item(
                        self,
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
                    rows.push(item(
                        self,
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
                rows.push(item(
                    self,
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
                    rows.push(item(
                        self,
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
                    rows.push(item(
                        self,
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
                rows.push(item(
                    self,
                    window,
                    cx,
                    0,
                    "Не оценено",
                    cur.is_none(),
                    MenuAction::SetSignificance(tid.clone(), None),
                ));
                for v in 1..=10u8 {
                    rows.push(item(
                        self,
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
                rows.push(item(
                    self,
                    window,
                    cx,
                    0,
                    "Без оплаты",
                    !cur,
                    MenuAction::SetBillable(tid.clone(), false),
                ));
                rows.push(item(
                    self,
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
                    rows.push(item(self, window, cx, i, l, cur == *d, act));
                }
            }
            DropKind::QeProject => {
                rows.push(item(
                    self,
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
                    rows.push(item(
                        self,
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
                for (i, (v, l)) in
                    [(0u8, "День"), (1, "Неделя"), (2, "Месяц"), (3, "Год")].iter().enumerate()
                {
                    rows.push(item(
                        self,
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
                for (i, (v, l)) in
                    [(0u8, "по расписанию"), (1u8, "после выполнения")].iter().enumerate()
                {
                    rows.push(item(
                        self,
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
            .border_color(c(BORDER))
            .bg(c(BG))
            .shadow(vec![gpui::BoxShadow {
                color: rgba(0x000000, 0.12),
                offset: gpui::point(px(0.), px(8.)),
                blur_radius: px(24.),
                spread_radius: px(0.),
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

    // ==========================================================================
    // FAB (.btn-primary md = h40 px16) + dev-vars stub circle.
    // ==========================================================================

    fn render_fab(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let t = self.hover_t(window, "fab");
        let weak = cx.weak_entity();
        let fab = div()
            .id("fab-new")
            .absolute()
            .right_6()
            .bottom_6()
            .h_10()
            .px_4()
            .flex()
            .items_center()
            .gap_2()
            .rounded_lg()
            .bg(lerp(ACCENT, 0x1e4fd8, t * 0.5))
            .text_color(c(ACCENT_FG))
            .text_size(px(14.))
            .font_weight(gpui::FontWeight::MEDIUM)
            .shadow(vec![gpui::BoxShadow {
                color: rgba(0x000000, 0.15),
                offset: gpui::point(px(0.), px(4.)),
                blur_radius: px(12.),
                spread_radius: px(0.),
            }])
            .child(icon("icons/plus.svg", 16., c(ACCENT_FG)))
            .child("Новая задача")
            .on_hover(move |hovered, _, cx| {
                let _ = weak.update(cx, |this, _| this.set_hover("fab", *hovered));
            })
            .on_click({
                let weak = cx.weak_entity();
                move |_: &ClickEvent, _, cx| {
                    let _ = weak.update(cx, |this, _| {
                        this.quick_entry_open = true;
                    });
                }
            });
        // dev DesignVarsPanel stub: 32px black circle w/ asterisk.
        let dev = div()
            .absolute()
            .right_7()
            .bottom(px(84.))
            .w_8()
            .h_8()
            .rounded_full()
            .bg(c(0x000000))
            .grid()
            .items_center().justify_center()
            .text_color(c(0xffffff))
            .text_size(px(15.))
            .shadow(vec![gpui::BoxShadow {
                color: rgba(0x000000, 0.2),
                offset: gpui::point(px(0.), px(4.)),
                blur_radius: px(10.),
                spread_radius: px(0.),
            }])
            .child("✳");
        div().size_full().absolute().inset_0().child(dev).child(fab).into_any_element()
    }

    // ==========================================================================
    // Pages
    // ==========================================================================

    fn logbook_page(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let items = filter_todos(SmartList::Logbook, &self.todos);
        let archived: Vec<Project> = self
            .projects
            .iter()
            .filter(|p| p.status == 2)
            .cloned()
            .collect();
        let mut list = div()
            .id("list-logbook")
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .track_scroll(&self.scroll("logbook"));
        if items.is_empty() && archived.is_empty() {
            list = list.child(empty_state());
        }
        if !items.is_empty() {
            list = list.child(
                div()
                    .pt(px(14.))
                    .pb_1()
                    .px_7()
                    .text_size(px(12.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(c(MUTED_FG))
                    .child(format!("Задачи · {}", items.len())),
            );
            for t in &items {
                list = list.child(self.task_row(t, true, window, cx));
            }
        }
        if !archived.is_empty() {
            list = list.child(
                div()
                    .pt(px(14.))
                    .pb_1()
                    .px_7()
                    .text_size(px(12.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(c(MUTED_FG))
                    .child(format!("Проекты · {}", archived.len())),
            );
            for p in &archived {
                let hid = format!("lb-{}", p.id);
                let t = self.hover_t(window, &hid);
                let weak = cx.weak_entity();
                let weak2 = weak.clone();
                let pid = p.id.to_string();
                let key = SharedString::from(hid.clone());
                list = list.child(
                    div()
                        .id(SharedString::from(format!("el-{hid}")))
                        .min_h(px(36.))
                        .w_full()
                        .flex()
                        .items_center()
                        .gap_2()
                        .px_2()
                        .border_b_1()
                        .border_color(border_mix(0.55))
                        .text_size(px(13.))
                        .bg(fg_mix(0.04 * t))
                        .child(icon("icons/folder.svg", 16., c(MUTED_FG)))
                        .child(div().flex_1().text_color(c(FG)).child(p.title.clone()))
                        .on_hover(move |hovered, _, cx| {
                            let _ = weak.update(cx, |this, _| this.set_hover(&key, *hovered));
                        })
                        .on_mouse_down(MouseButton::Right, {
                            let weak = weak2.clone();
                            let pid = pid.clone();
                            move |ev: &MouseDownEvent, _, cx| {
                                let pos = ev.position;
                                let pid = pid.clone();
                                let _ = weak.update(cx, |this, _| {
                                    this.menu = Some(CtxMenu {
                                        x: pos.x.into(),
                                        y: pos.y.into(),
                                        items: vec![
                                            (
                                                "Восстановить".into(),
                                                false,
                                                MenuAction::RestoreProject(pid.clone()),
                                            ),
                                            (
                                                "Удалить".into(),
                                                true,
                                                MenuAction::DeleteProject(pid.clone()),
                                            ),
                                        ],
                                    });
                                });
                            }
                        }),
                );
            }
        }
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(self.options_row("agenda.logbook.view", window, cx))
            .child(list)
            .into_any_element()
    }

    fn trash_page(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let items = filter_todos(SmartList::Trash, &self.todos);
        let mut list = div()
            .id("list-trash")
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .track_scroll(&self.scroll("trash"));
        if items.is_empty() {
            list = list.child(empty_state());
        }
        for t in &items {
            list = list.child(self.task_row(t, false, window, cx));
        }
        div().flex_1().min_h_0().flex().flex_col().child(list).into_any_element()
    }

    fn project_page(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let pid: Option<&'static str> =
            self.projects.iter().find(|p| p.id == id).map(|p| p.id);
        let items: Vec<Todo> = self
            .todos
            .iter()
            .filter(|t| {
                t.project_id == pid && !t.is_trashed && !is_archived(t, &today_key())
            })
            .cloned()
            .collect();
        let storage = format!("agenda.project.{id}.view");
        let opts = self.board_opts(&storage);
        let items = sorted(&items, opts.sort);
        let mut list = div()
            .id("list-project")
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .track_scroll(&self.scroll("project"));
        if items.is_empty() {
            list = list.child(empty_state());
        }
        for t in &items {
            list = list.child(self.task_row(t, true, window, cx));
        }
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(self.options_row(&storage, window, cx))
            .child(list)
            .into_any_element()
    }

    // ==========================================================================
    // Task page (Linear-like): bar, title input, prop chips, notes.
    // ==========================================================================

    fn task_page(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        // Reset per-task inputs when navigating between tasks.
        if self.input_task.as_deref() != Some(id) {
            self.input_task = Some(id.to_string());
            for k in ["task-title", "task-notes"] {
                self.inputs.remove(k);
            }
        }
        let todo = self.todos.iter().find(|t| t.id == id).cloned();
        let Some(t) = todo else {
            return div().flex_1().child(empty_state()).into_any_element();
        };

        let title_state = self.input_state(window, cx, "task-title", "Название задачи", false);
        if title_state.read(cx).value().is_empty() && !t.title.is_empty() {
            let tv = t.title.clone();
            title_state.update(cx, |s, cx| s.set_value(tv, window, cx));
        }
        let notes_state =
            self.input_state(window, cx, "task-notes", "Добавить описание...", true);
        if notes_state.read(cx).value().is_empty() {
            if let Some(n) = &t.notes {
                let nv = n.clone();
                notes_state.update(cx, |s, cx| s.set_value(nv, window, cx));
            }
        }

        let status = task_status(&t);
        let (date_val, _) = task_date(&t);
        let today = today_key();
        let overdue = date_val.as_deref().map_or(false, |d| d < today.as_str())
            && status != Status::Done
            && status != Status::Canceled;

        // ---- bar: more button
        let bar = div()
            .flex()
            .items_center()
            .justify_end()
            .gap_1()
            .min_h_7()
            .child({
                let hid = "tp-more";
                let t2 = self.hover_t(window, hid);
                let weak = cx.weak_entity();
                let id2 = id.to_string();
                div()
                    .id("tp-more-btn")
                    .w_7()
                    .h_7()
                    .grid()
                    .items_center().justify_center()
                    .rounded_md()
                    .bg(fg_mix(0.06 * t2))
                    .child(icon("icons/more-h.svg", 16., mix(MUTED_FG, 0.6 + 0.4 * t2, BG)))
                    .on_hover({
                        let weak = weak.clone();
                        move |hovered, _, cx| {
                            let _ = weak.update(cx, |this, _| this.set_hover(hid, *hovered));
                        }
                    })
                    .on_click(move |ev: &ClickEvent, _, cx| {
                        let pos = ev.position();
                        let id2 = id2.clone();
                        let _ = weak.update(cx, |this, _| {
                            this.menu = Some(CtxMenu {
                                x: pos.x.into(),
                                y: pos.y.into(),
                                items: vec![
                                    (
                                        "Вернуть в работу".into(),
                                        false,
                                        MenuAction::CompleteTodo(id2.clone()),
                                    ),
                                    ("Удалить".into(), true, MenuAction::TrashTodo(id2)),
                                ],
                            });
                        });
                    })
            });

        // ---- prop chips
        let status_label = match status {
            Status::Inbox => "Входящие",
            Status::Todo => "Сделать",
            Status::Started => "В работе",
            Status::Deferred => "Потом",
            Status::Done => "Готово",
            Status::Canceled => "Отменено",
        };
        let priority_label = ["Без приоритета", "Низкий", "Средний", "Высокий"][t.priority as usize];

        let mut props = div().flex().flex_wrap().items_center().gap_1();
        props = props.child(
            self.prop_chip("tp-status", DropKind::Status, id, status_label, window, cx)
                .child(status_ring(status)),
        );
        props = props.child(
            self.prop_chip("tp-prio", DropKind::Priority, id, priority_label, window, cx)
                .child(priority_bars(t.priority)),
        );

        // date chip
        let date_label = date_val
            .as_ref()
            .map(|d| fmt_day_month(d))
            .unwrap_or_else(|| "Срок".to_string());
        let mut date_chip = self
            .prop_chip("tp-date", DropKind::Date, id, "", window, cx)
            .child(icon("icons/calendar-02.svg", 14., c(MUTED_FG)))
            .child(
                div()
                    .text_color(if overdue { c(DESTRUCTIVE) } else { c(FG) })
                    .child(date_label),
            );
        if date_val.is_some() {
            date_chip = date_chip.child(
                div()
                    .id("tp-date-clear")
                    .w_3p5()
                    .h_3p5()
                    .grid()
                    .items_center().justify_center()
                    .rounded_full()
                    .child(icon("icons/status-x.svg", 9., c(MUTED_FG)))
                    .on_click({
                        let weak = cx.weak_entity();
                        let id2 = id.to_string();
                        move |_: &ClickEvent, _, cx| {
                            let _ = weak.update(cx, |this, _| {
                                this.run_menu_action(MenuAction::SetDate(id2.clone(), None));
                            });
                        }
                    }),
            );
        }
        props = props.child(date_chip);

        // project chip
        let project_label = t
            .project_id
            .and_then(|pid| self.projects.iter().find(|p| p.id == pid))
            .map(|p| p.title.clone())
            .unwrap_or_else(|| "Без проекта".to_string());
        props = props.child(
            self.prop_chip("tp-proj", DropKind::Project, id, &project_label, window, cx)
                .child(icon("icons/folder.svg", 14., c(MUTED_FG))),
        );

        // tag pills + "Метки" add chip
        for tag_id in &t.tag_ids {
            if let Some(tag) = self.tag(tag_id) {
                let dot = tag_color(tag.color);
                props = props.child(
                    div()
                        .h(px(22.))
                        .pl_2()
                        .pr_1()
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .rounded_full()
                        .bg(fg_mix(0.06))
                        .text_size(px(12.))
                        .text_color(c(FG))
                        .child(div().w(px(7.)).h(px(7.)).rounded_full().bg(c(dot)))
                        .child(tag.title.clone())
                        .child(
                            div()
                                .id(SharedString::from(format!("tagx-{}", tag.id)))
                                .w_3p5()
                                .h_3p5()
                                .grid()
                                .items_center().justify_center()
                                .rounded_full()
                                .child(icon("icons/status-x.svg", 9., c(MUTED_FG)))
                                .on_click({
                                    let weak = cx.weak_entity();
                                    let id2 = id.to_string();
                                    let tagid = tag.id.to_string();
                                    move |_: &ClickEvent, _, cx| {
                                        let _ = weak.update(cx, |this, _| {
                                            this.run_menu_action(MenuAction::RemoveTag(
                                                id2.clone(),
                                                tagid.clone(),
                                            ));
                                        });
                                    }
                                }),
                        ),
                );
            }
        }
        props = props.child(
            self.prop_chip("tp-tags", DropKind::Tags, id, "Метки", window, cx)
                .child(icon("icons/tag.svg", 14., c(MUTED_FG))),
        );

        // recurrence chip
        let recur_label = if t.recurrence.is_some() {
            describe_recurrence(&t.recurrence)
        } else {
            "Повторение".to_string()
        };
        props = props.child(
            self.recur_chip("tp-recur", id, &recur_label, window, cx)
                .child(icon("icons/repeat.svg", 14., c(MUTED_FG))),
        );

        // second row: significance + billable + fuel
        let mut props2 = div().flex().flex_wrap().items_center().gap_1();
        let sig_label = t
            .significance
            .map(|s| format!("{}/10", s))
            .unwrap_or_else(|| "Не оценено".to_string());
        props2 = props2.child(
            self.prop_chip("tp-sig", DropKind::Significance, id, &sig_label, window, cx)
                .child(icon("icons/star.svg", 14., c(MUTED_FG))),
        );
        let bill_label = if t.billable {
            format!("Оплачиваемая ${}", t.price.unwrap_or(0))
        } else {
            "Без оплаты".to_string()
        };
        props2 = props2.child(
            self.prop_chip("tp-bill", DropKind::Billable, id, &bill_label, window, cx)
                .child(icon("icons/dollar.svg", 14., c(MUTED_FG))),
        );
        if let Some(f) = t.fuel_cost {
            props2 = props2.child(
                div()
                    .h_7()
                    .px_2()
                    .flex()
                    .items_center()
                    .text_size(px(13.))
                    .text_color(c(MUTED_FG))
                    .child(format!("~{}%", f)),
            );
        }

        // recurrence editor card
        let recur_editor = if self.recur_open {
            self.render_recur_editor(&t, window, cx).into_any_element()
        } else {
            div().into_any_element()
        };

        let column = div()
            .w_full()
            .max_w(px(720.))
            .mx_auto()
            .pt_5()
            .pb_16()
            .px_8()
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(bar)
            .child(
                Input::new(&title_state)
                    .text_size(px(22.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(c(FG))
                    .p_0()
                    .appearance(false)
                    .bordered(false)
                    .h(px(30.)),
            )
            .child(props)
            .child(props2)
            .child(recur_editor)
            .child(
                Input::new(&notes_state)
                    .text_size(px(13.))
                    .text_color(c(FG))
                    .p_0()
                    .appearance(false)
                    .bordered(false)
                    .h(px(120.)),
            );

        div()
            .id("task-page")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.scroll("task"))
            .flex()
            .flex_col()
            .child(column)
            .into_any_element()
    }

    /// `.task-prop` chip button: h28 px8 r6 gap6 fs13, hover fg6%.
    fn prop_chip(
        &mut self,
        hid: &str,
        kind: DropKind,
        todo_id: &str,
        label: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let t = self.hover_t(window, hid);
        let weak = cx.weak_entity();
        let key = SharedString::from(hid.to_string());
        let tid = todo_id.to_string();
        div()
            .id(SharedString::from(format!("chip-{hid}")))
            .h_7()
            .px_2()
            .flex()
            .items_center()
            .gap_1p5()
            .rounded_md()
            .text_size(px(13.))
            .text_color(c(FG))
            .bg(fg_mix(0.06 * t))
            .when(!label.is_empty(), |el| el.child(label.to_string()))
            .on_hover({
                let weak = weak.clone();
                move |hovered, _, cx| {
                    let _ = weak.update(cx, |this, _| this.set_hover(&key, *hovered));
                }
            })
            .on_click(move |ev: &ClickEvent, _, cx| {
                let pos = ev.position();
                let tid = tid.clone();
                let _ = weak.update(cx, |this, _| {
                    this.dropdown = Some(DropState {
                        kind,
                        x: pos.x.into(),
                        y: pos.y.into(),
                        todo_id: Some(tid),
                    });
                });
            })
    }

    fn recur_chip(
        &mut self,
        hid: &str,
        todo_id: &str,
        label: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let t = self.hover_t(window, hid);
        let weak = cx.weak_entity();
        let key = SharedString::from(hid.to_string());
        let tid = todo_id.to_string();
        div()
            .id(SharedString::from(format!("chip-{hid}")))
            .h_7()
            .px_2()
            .flex()
            .items_center()
            .gap_1p5()
            .rounded_md()
            .text_size(px(13.))
            .text_color(c(FG))
            .bg(fg_mix(0.06 * t))
            .when(!label.is_empty(), |el| el.child(label.to_string()))
            .on_hover({
                let weak = weak.clone();
                move |hovered, _, cx| {
                    let _ = weak.update(cx, |this, _| this.set_hover(&key, *hovered));
                }
            })
            .on_click(move |_: &ClickEvent, _, cx| {
                let tid = tid.clone();
                let _ = weak.update(cx, |this, _| {
                    this.recur_open = !this.recur_open;
                    if this.recur_open {
                        if let Some(t) = this.todos.iter().find(|t| t.id == tid) {
                            if let Some(r) = &t.recurrence {
                                this.recur_freq = r.frequency;
                                this.recur_interval = r.interval;
                                this.recur_type = r.recurrence_type;
                                this.recur_days = r.days_of_week.clone();
                            } else {
                                this.recur_freq = 1;
                                this.recur_interval = 1;
                                this.recur_type = 0;
                                this.recur_days = vec![];
                            }
                        }
                    }
                });
            })
    }

    /// Inline recurrence editor (dropdown + weekday toggles + apply/clear).
    fn render_recur_editor(
        &mut self,
        t: &Todo,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let freq_label = ["День", "Неделя", "Месяц", "Год"][self.recur_freq as usize];
        let type_label = if self.recur_type == 1 { "после выполнения" } else { "по расписанию" };
        let day_names = ["пн", "вт", "ср", "чт", "пт", "сб", "вс"];
        let tid = t.id.to_string();

        let chip_btn = |app: &mut Self,
                        window: &mut Window,
                        cx: &mut Context<Self>,
                        hid: &str,
                        label: String,
                        kind: DropKind| {
            let t = app.hover_t(window, hid);
            let weak = cx.weak_entity();
            let key = SharedString::from(hid.to_string());
            div()
                .id(SharedString::from(format!("rc-{hid}")))
                .h_7()
                .px_2()
                .flex()
                .items_center()
                .rounded_md()
                .text_size(px(12.))
                .text_color(c(FG))
                .bg(fg_mix(0.05 + 0.02 * t))
                .border_1()
                .border_color(c(BORDER))
                .child(label)
                .on_hover({
                    let weak = weak.clone();
                    move |hovered, _, cx| {
                        let _ = weak.update(cx, |this, _| this.set_hover(&key, *hovered));
                    }
                })
                .on_click(move |ev: &ClickEvent, _, cx| {
                    let pos = ev.position();
                    let _ = weak.update(cx, |this, _| {
                        this.dropdown = Some(DropState {
                            kind,
                            x: pos.x.into(),
                            y: pos.y.into(),
                            todo_id: None,
                        });
                    });
                })
        };

        let mut row = div().flex().items_center().gap_2().flex_wrap();
        row = row.child(chip_btn(self, window, cx, "rc-freq", freq_label.to_string(), DropKind::RecurFreq));
        row = row.child(chip_btn(self, window, cx, "rc-type", type_label.to_string(), DropKind::RecurType));
        if self.recur_freq == 1 {
            for (i, name) in day_names.iter().enumerate() {
                let d = (i + 1) as u8;
                let sel = self.recur_days.contains(&d);
                let hid = format!("rc-day-{i}");
                let ht = self.hover_t(window, &hid);
                let weak = cx.weak_entity();
                let key = SharedString::from(hid.clone());
                row = row.child(
                    div()
                        .id(SharedString::from(format!("el-{hid}")))
                        .h_7()
                        .w_8()
                        .grid()
                        .items_center().justify_center()
                        .rounded_md()
                        .text_size(px(12.))
                        .text_color(if sel { c(ACCENT_FG) } else { c(FG) })
                        .bg(if sel { c(ACCENT) } else { fg_mix(0.04 + 0.03 * ht) })
                        .child(*name)
                        .on_hover(move |hovered, _, cx| {
                            let _ = weak.update(cx, |this, _| this.set_hover(&key, *hovered));
                        })
                        .on_click({
                            let weak = cx.weak_entity();
                            move |_: &ClickEvent, _, cx| {
                                let _ = weak.update(cx, |this, _| {
                                    if this.recur_days.contains(&d) {
                                        this.recur_days.retain(|x| *x != d);
                                    } else {
                                        this.recur_days.push(d);
                                        this.recur_days.sort();
                                    }
                                });
                            }
                        }),
                );
            }
        }
        // apply + clear
        let apply = {
            let hid = "rc-apply";
            let t2 = self.hover_t(window, hid);
            let weak = cx.weak_entity();
            let tid2 = tid.clone();
            div()
                .id("rc-apply-btn")
                .h_7()
                .px_3()
                .flex()
                .items_center()
                .rounded_md()
                .text_size(px(12.))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(c(ACCENT_FG))
                .bg(lerp(ACCENT, 0x1e4fd8, t2 * 0.5))
                .child("Применить")
                .on_hover({
                    let weak = weak.clone();
                    move |hovered, _, cx| {
                        let _ = weak.update(cx, |this, _| this.set_hover(hid, *hovered));
                    }
                })
                .on_click(move |_: &ClickEvent, _, cx| {
                    let tid2 = tid2.clone();
                    let _ = weak.update(cx, |this, _| {
                        let rule = RecurrenceRule {
                            frequency: this.recur_freq,
                            interval: this.recur_interval.max(1),
                            recurrence_type: this.recur_type,
                            days_of_week: this.recur_days.clone(),
                        };
                        this.update_todo(&tid2, |t| t.recurrence = Some(rule));
                        this.recur_open = false;
                    });
                })
        };
        let clear = {
            let hid = "rc-clear";
            let t2 = self.hover_t(window, hid);
            let weak = cx.weak_entity();
            let tid2 = tid.clone();
            div()
                .id("rc-clear-btn")
                .h_7()
                .px_3()
                .flex()
                .items_center()
                .rounded_md()
                .text_size(px(12.))
                .text_color(mix(FG, 0.6 + 0.4 * t2, BG))
                .bg(fg_mix(0.06 * t2))
                .child("Убрать")
                .on_hover({
                    let weak = weak.clone();
                    move |hovered, _, cx| {
                        let _ = weak.update(cx, |this, _| this.set_hover(hid, *hovered));
                    }
                })
                .on_click(move |_: &ClickEvent, _, cx| {
                    let tid2 = tid2.clone();
                    let _ = weak.update(cx, |this, _| {
                        this.update_todo(&tid2, |t| t.recurrence = None);
                        this.recur_open = false;
                    });
                })
        };
        row = row.child(apply).child(clear);

        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .rounded_lg()
            .border_1()
            .border_color(c(BORDER))
            .bg(c(SECONDARY))
            .child(row)
    }

    // ==========================================================================
    // Calendar
    // ==========================================================================

    fn calendar_page(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let anchor = parse_key(&self.cal_anchor).unwrap_or_else(|| Local::now().date_naive());
        let (days, period): (Vec<String>, String) = match self.cal_mode {
            CalMode::Day => {
                let k = key_of(anchor);
                (vec![k.clone()], fmt_day_month_year(&k))
            }
            CalMode::Week => {
                let monday = anchor
                    - Duration::days(anchor.weekday().num_days_from_monday() as i64);
                let days: Vec<String> =
                    (0..7).map(|i| key_of(monday + Duration::days(i))).collect();
                (days.clone(), fmt_week_period(&days[0], &days[6]))
            }
        };
        let today = today_key();

        // header controls
        let seg = |app: &mut Self, window: &mut Window, cx: &mut Context<Self>, mode: CalMode, label: &str| {
            let hid = format!("cal-seg-{label}");
            let t = app.hover_t(window, &hid);
            let active = app.cal_mode == mode;
            let weak = cx.weak_entity();
            let key = SharedString::from(hid.clone());
            div()
                .id(SharedString::from(format!("el-{hid}")))
                .h_7()
                .px_3()
                .flex()
                .items_center()
                .rounded_md()
                .text_size(px(12.))
                .text_color(if active { c(FG) } else { c(MUTED_FG) })
                .bg(if active { fg_mix(0.10) } else { fg_mix(0.05 * t) })
                .child(label.to_string())
                .on_hover({
                    let weak = weak.clone();
                    move |hovered, _, cx| {
                        let _ = weak.update(cx, |this, _| this.set_hover(&key, *hovered));
                    }
                })
                .on_click(move |_: &ClickEvent, _, cx| {
                    let _ = weak.update(cx, |this, _| this.cal_mode = mode);
                })
        };
        let nav = |app: &mut Self,
                   window: &mut Window,
                   cx: &mut Context<Self>,
                   hid: &str,
                   label: &str,
                   delta: i64| {
            let t = app.hover_t(window, hid);
            let weak = cx.weak_entity();
            let key = SharedString::from(hid.to_string());
            div()
                .id(SharedString::from(format!("el-{hid}")))
                .h_7()
                .px_3()
                .flex()
                .items_center()
                .rounded_md()
                .text_size(px(12.))
                .text_color(mix(FG, 0.65 + 0.35 * t, BG))
                .bg(fg_mix(0.05 + 0.03 * t))
                .child(label.to_string())
                .on_hover({
                    let weak = weak.clone();
                    move |hovered, _, cx| {
                        let _ = weak.update(cx, |this, _| this.set_hover(&key, *hovered));
                    }
                })
                .on_click(move |_: &ClickEvent, _, cx| {
                    let _ = weak.update(cx, |this, _| {
                        if delta == 0 {
                            this.cal_anchor = today_key();
                        } else {
                            let a = parse_key(&this.cal_anchor)
                                .unwrap_or_else(|| Local::now().date_naive());
                            let step = if this.cal_mode == CalMode::Week { delta * 7 } else { delta };
                            this.cal_anchor = key_of(a + Duration::days(step));
                        }
                    });
                })
        };

        let header = div()
            .flex_none()
            .flex()
            .items_center()
            .px_7()
            .pt_4()
            .pb_3()
            .child(
                div()
                    .flex_1()
                    .text_size(px(13.))
                    .text_color(c(MUTED_FG))
                    .child(period),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(seg(self, window, cx, CalMode::Day, "День"))
                    .child(seg(self, window, cx, CalMode::Week, "Неделя")),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .ml_2()
                    .child(nav(self, window, cx, "cal-prev", "←", -1))
                    .child(nav(self, window, cx, "cal-today", "Сегодня", 0))
                    .child(nav(self, window, cx, "cal-next", "→", 1)),
            );

        // columns
        let mut grid = div()
            .flex_1()
            .min_h_0()
            .flex()
            .gap_3()
            .px_7()
            .pb_4()
            .overflow_hidden();
        for d in &days {
            let day_events: Vec<CalEvent> = self
                .events
                .iter()
                .filter(|e| date_only(&Some(e.starts_at.clone())).as_deref() == Some(d.as_str()))
                .cloned()
                .collect();
            let day_todos: Vec<Todo> = self
                .todos
                .iter()
                .filter(|t| {
                    !t.is_trashed && task_date(t).0.as_deref() == Some(d.as_str())
                })
                .cloned()
                .collect();
            let is_today = *d == today;

            // .calendar-day: min-h-40 rounded-lg border bg-secondary/20 p-3; today → border-accent
            let mut head = div()
                .text_size(px(13.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(c(FG))
                .flex()
                .gap_1()
                .child(fmt_cal_day_label(d));
            if is_today {
                head = head.child(
                    div().text_size(px(11.)).text_color(c(ACCENT)).child("Сегодня"),
                );
            }
            let mut col = div()
                .flex_1()
                .min_w_0()
                .min_h(px(160.))
                .rounded_lg()
                .border_1()
                .border_color(if is_today { c(ACCENT) } else { c(BORDER) })
                .bg(rgba(SECONDARY, 0.2))
                .p_3()
                .flex()
                .flex_col()
                .gap_2()
                .child(head);
            if day_events.is_empty() && day_todos.is_empty() {
                col = col.child(
                    div()
                        .text_size(px(12.))
                        .text_color(c(MUTED_FG))
                        .child("Нет задач"),
                );
            }
            for e in &day_events {
                let mut card = div()
                    .rounded_md()
                    .border_1()
                    .border_dashed()
                    .border_color(c(BORDER))
                    .bg(c(BG))
                    .p_2()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(c(ACCENT))
                            .child(format!(
                                "{}–{}",
                                fmt_time(&e.starts_at),
                                e.ends_at.as_deref().map(fmt_time).unwrap_or_default()
                            )),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(c(FG))
                            .child(e.title),
                    );
                let loc = e.location.unwrap_or("PseudoCalendar");
                card = card.child(
                    div()
                        .text_size(px(12.))
                        .text_color(c(MUTED_FG))
                        .child(format!("{} · PseudoCalendar", loc)),
                );
                col = col.child(card);
            }
            for t in &day_todos {
                let st = task_status(t);
                let st_label = match st {
                    Status::Done => "Готово",
                    Status::Started => "В работе",
                    Status::Canceled => "Отменено",
                    Status::Deferred => "Потом",
                    _ => "К выполнению",
                };
                let weak = cx.weak_entity();
                let tid = t.id.to_string();
                let hid = format!("cal-t-{}", t.id);
                let ht = self.hover_t(window, &hid);
                let key = SharedString::from(hid.clone());
                col = col.child(
                    div()
                        .id(SharedString::from(format!("el-{hid}")))
                        .rounded_md()
                        .border_1()
                        .border_color(lerp(BORDER, ACCENT, ht * 0.5))
                        .bg(c(BG))
                        .p_2()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(c(FG))
                                .child(t.title.clone()),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(c(MUTED_FG))
                                .child(format!("{} · Agenda", st_label)),
                        )
                        .on_hover(move |hovered, _, cx| {
                            let _ = weak.update(cx, |this, _| this.set_hover(&key, *hovered));
                        })
                        .on_click({
                            let weak = cx.weak_entity();
                            let tid = tid.clone();
                            move |_: &ClickEvent, _, cx| {
                                let _ = weak
                                    .update(cx, |this, _| this.navigate(Route::Task(tid.clone())));
                            }
                        }),
                );
            }
            grid = grid.child(col);
        }

        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(header)
            .child(grid)
            .into_any_element()
    }

    // ==========================================================================
    // Statistics heatmap (84 days, 12×7)
    // ==========================================================================

    fn statistics_page(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let metric_labels = ["Выполнено", "Значимость", "Мыслетопливо"];
        let mut tabs = div().flex().gap_2();
        for (i, l) in metric_labels.iter().enumerate() {
            let hid = format!("stat-tab-{i}");
            let t = self.hover_t(window, &hid);
            let active = self.stat_metric == i as u8;
            let weak = cx.weak_entity();
            let key = SharedString::from(hid.clone());
            tabs = tabs.child(
                div()
                    .id(SharedString::from(format!("el-{hid}")))
                    .h_8()
                    .px_3()
                    .flex()
                    .items_center()
                    .rounded_md()
                    .text_size(px(13.))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(if active { c(ACCENT_FG) } else { c(FG) })
                    .bg(if active { c(ACCENT) } else { fg_mix(0.05 + 0.03 * t) })
                    .child(*l)
                    .on_hover(move |hovered, _, cx| {
                        let _ = weak.update(cx, |this, _| this.set_hover(&key, *hovered));
                    })
                    .on_click({
                        let weak = cx.weak_entity();
                        move |_: &ClickEvent, _, cx| {
                            let _ = weak.update(cx, |this, _| this.stat_metric = i as u8);
                        }
                    }),
            );
        }

        // values: last 84 days ending today (12 weeks × 7)
        let today = Local::now().date_naive();
        let start = today - Duration::days(83);
        let mut vals = [0f64; 84];
        let mut max_v = 0f64;
        for t in &self.todos {
            if let Some(d) = date_only(&t.completed_at).and_then(|k| parse_key(&k)) {
                let idx = (d - start).num_days();
                if (0..84).contains(&idx) {
                    let v = match self.stat_metric {
                        1 => t.significance.unwrap_or(0) as f64,
                        2 => t.fuel_cost.unwrap_or(0) as f64,
                        _ => 1.0,
                    };
                    vals[idx as usize] += v;
                    if vals[idx as usize] > max_v {
                        max_v = vals[idx as usize];
                    }
                }
            }
        }

        // grid-flow-col grid-rows-7: 12 week columns, 7 cells each, stretch to fill.
        // Emit column-major (week = 7 consecutive days) so ordering matches Vue.
        // Available content width = viewport - sidebar - px_8 padding, capped at max-w-4xl
        // inner (832px). w_full collapses inside the scroll container, so size explicitly.
        let grid_w = (window.viewport_size().width - px(240.0 + 64.0)).min(px(832.));
        let mut grid = div().flex().gap_1().w(grid_w).mt_4();
        for col_i in 0..12usize {
            let mut week = div().flex_1().min_w_0().flex().flex_col().gap_1();
            for row in 0..7usize {
                let v = vals[col_i * 7 + row];
                let cell_bg = if v <= 0.0 {
                    c(0xe5e5e5)
                } else {
                    let t = (v / max_v.max(1.0)).min(1.0);
                    lerp(0xbdbdbd, 0x262626, (0.35 + 0.65 * t) as f32)
                };
                week = week.child(
                    div()
                        .w_full()
                        .h_8()
                        .rounded_md()
                        .border_1()
                        .border_color(c(BORDER))
                        .bg(cell_bg)
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(10.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(if v > 0.0 { c(0xf5f5f5) } else { rgba(0, 0.0) })
                        .child(if v > 0.0 {
                            format!("{}", v as i64)
                        } else {
                            String::new()
                        }),
                );
            }
            grid = grid.child(week);
        }

        let desc = div()
            .mt_2()
            .text_size(px(12.))
            .text_color(c(MUTED_FG))
            .child(format!(
                "{}: светлее — меньше, насыщеннее — больше. Ячейка показывает точное значение; серый цвет означает неполные или недоступные данные.",
                metric_labels[self.stat_metric as usize]
            ));

        let legend = div()
            .flex()
            .items_center()
            .gap_1()
            .mt_2()
            .text_size(px(12.))
            .text_color(c(MUTED_FG))
            .child("Меньше")
            .children((0..5).map(|i| {
                div()
                    .w_3()
                    .h_3()
                    .rounded(px(2.))
                    .bg(lerp(0xe5e5e5, 0x262626, i as f32 / 4.0))
            }))
            .child("Больше");

        div()
            .flex_1()
            .min_h_0()
            .id("stats-scroll")
            .overflow_y_scroll()
            .track_scroll(&self.scroll("stats"))
            .child(
                div()
                    .w_full()
                    .max_w(px(896.))
                    .mx_auto()
                    .pt_4()
                    .px_8()
                    .flex()
                    .flex_col()
                    .child(tabs)
                    .child(grid)
                    .child(desc)
                    .child(legend),
            )
            .into_any_element()
    }

    // ==========================================================================
    // Recurring / Settings / About
    // ==========================================================================

    fn recurring_page(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let items: Vec<Todo> = self
            .todos
            .iter()
            .filter(|t| t.recurrence.is_some() && !t.is_trashed && !is_archived(t, &today_key()))
            .cloned()
            .collect();
        let mut list = div()
            .id("list-recurring")
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .gap_3()
            .px_7()
            .pt_4()
            .overflow_y_scroll()
            .track_scroll(&self.scroll("recurring"));
        if items.is_empty() {
            list = list.child(empty_state());
        }
        for t in &items {
            let hid = format!("rec-{}", t.id);
            let ht = self.hover_t(window, &hid);
            let weak = cx.weak_entity();
            let tid = t.id.to_string();
            let key = SharedString::from(hid.clone());
            list = list.child(
                div()
                    .id(SharedString::from(format!("el-{hid}")))
                    .rounded_lg()
                    .border_1()
                    .border_color(lerp(BORDER, ACCENT, ht * 0.4))
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .bg(fg_mix(0.02 * ht))
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(c(FG))
                            .child(t.title.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(c(MUTED_FG))
                            .child(describe_recurrence(&t.recurrence)),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(c(MUTED_FG))
                            .child(format!(
                                "Следующий срок: {}",
                                task_date(t).0.unwrap_or_default()
                            )),
                    )
                    .on_hover(move |hovered, _, cx| {
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
        list.into_any_element()
    }

    fn settings_page(&mut self, fuel: bool, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let mut col = div()
            .id("settings")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.scroll("settings"))
            .flex()
            .flex_col()
            .px_7()
            .pt_4()
            .gap_3()
            .max_w(px(720.));
        if fuel {
            col = col.child(
                div()
                    .text_size(px(15.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(c(FG))
                    .child("Мыслетопливо"),
            );
            col = col.child(
                div()
                    .text_size(px(13.))
                    .text_color(c(MUTED_FG))
                    .child(
                        "Мыслетопливо оценивает стоимость задач в процентах. Оценивайте задачи от 1 до 10 на странице задачи.",
                    ),
            );
        } else {
            col = col.child(
                div()
                    .text_size(px(13.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(c(FG))
                    .child("Тема"),
            );
            let mut row = div().flex().gap_2();
            for (i, l) in ["Светлая", "Тёмная", "Системная"].iter().enumerate() {
                let active = self.theme_sel == i as u8;
                let hid = format!("theme-{i}");
                let t = self.hover_t(window, &hid);
                let weak = cx.weak_entity();
                let key = SharedString::from(hid.clone());
                row = row.child(
                    div()
                        .id(SharedString::from(format!("el-{hid}")))
                        .h_8()
                        .px_3()
                        .flex()
                        .items_center()
                        .rounded_md()
                        .text_size(px(13.))
                        .text_color(if active { c(ACCENT_FG) } else { c(FG) })
                        .bg(if active { c(ACCENT) } else { fg_mix(0.05 + 0.03 * t) })
                        .child(*l)
                        .on_hover(move |hovered, _, cx| {
                            let _ = weak.update(cx, |this, _| this.set_hover(&key, *hovered));
                        })
                        .on_click({
                            let weak = cx.weak_entity();
                            move |_: &ClickEvent, _, cx| {
                                let _ = weak.update(cx, |this, _| this.theme_sel = i as u8);
                            }
                        }),
                );
            }
            col = col.child(row);
            // fuel nav row
            let hid = "set-fuel";
            let t = self.hover_t(window, hid);
            let weak = cx.weak_entity();
            col = col.child(
                div()
                    .id("set-fuel-row")
                    .h_10()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .rounded_md()
                    .bg(fg_mix(0.04 * t))
                    .text_size(px(13.))
                    .text_color(c(FG))
                    .child("Мыслетопливо")
                    .child(icon("icons/arrow-left.svg", 14., c(MUTED_FG)))
                    .on_hover({
                        let weak = weak.clone();
                        move |hovered, _, cx| {
                            let _ = weak.update(cx, |this, _| this.set_hover(hid, *hovered));
                        }
                    })
                    .on_click(move |_: &ClickEvent, _, cx| {
                        let _ = weak.update(cx, |this, _| this.navigate(Route::SettingsFuel));
                    }),
            );
        }
        col.into_any_element()
    }

    fn about_page(&mut self) -> AnyElement {
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .items_center()
            .pt_16()
            .gap_2()
            .child(
                div()
                    .text_size(px(20.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(c(FG))
                    .child("Agenda"),
            )
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(c(MUTED_FG))
                    .child("Версия 0.1.0 (GPUI clone)"),
            )
            .into_any_element()
    }

    // ==========================================================================
    // Quick search overlay (Ctrl+K) — dark panel top-center.
    // ==========================================================================

    pub(crate) fn render_quick_search(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let qs = self.input_state(window, cx, "qs", "Поиск задач и проектов...", false);
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
                    .text_color(c(PANEL_FG))
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
                    }),
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
                    .text_color(c(PANEL_FG))
                    .bg(panel_mix(if sel { 0.10 } else { 0.06 * ht }))
                    .child(icon("icons/folder.svg", 14., c(PANEL_MUTED)))
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
                    }),
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
                    .text_color(c(PANEL_MUTED))
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
                            .text_color(c(PANEL_FG))
                            .child("Ничего не найдено"),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(c(PANEL_MUTED))
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
            .bg(c(PANEL))
            .border_1()
            .border_color(panel_mix(0.10))
            .shadow(vec![gpui::BoxShadow {
                color: rgba(0x000000, 0.35),
                offset: gpui::point(px(0.), px(16.)),
                blur_radius: px(48.),
                spread_radius: px(0.),
            }])
            .child(
                div()
                    .h(px(56.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap_2p5()
                    .px_3p5()
                    .child(icon("icons/search.svg", 16., c(PANEL_MUTED)))
                    .child(
                        Input::new(&qs)
                            .flex_1()
                            .text_size(px(14.))
                            .text_color(c(PANEL_FG))
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
                            .text_color(c(PANEL_MUTED))
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
            .child(
                div()
                    .id("qs-backdrop")
                    .absolute()
                    .inset_0()
                    .on_mouse_down(MouseButton::Left, move |_: &MouseDownEvent, _, cx| {
                        let _ = weak.update(cx, |this, _| {
                            this.quick_open = false;
                            this.qs_focused = false;
                        });
                    }),
            )
            .child(div().mt(px(128.)).child(deferred(panel)))
            .into_any_element()
    }

    // ==========================================================================
    // Quick entry overlay (Ctrl+N) — dark panel top-center + significance card.
    // ==========================================================================

    pub(crate) fn render_quick_entry(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // contextual defaults
        if self.qe_project.is_none() {
            if let Route::Project(pid) = &self.route {
                self.qe_project = Some(pid.clone());
            }
        }
        if self.qe_date.is_none() && !self.qe_date_touched {
            if self.route == Route::Today {
                self.qe_date = Some(today_key());
            }
        }
        let title_state = self.input_state(window, cx, "qe-title", "Новая задача", false);
        let notes_state = self.input_state(window, cx, "qe-notes", "Заметки", false);
        if !self.qe_focused {
            self.qe_focused = true;
            title_state.update(cx, |s, cx| s.focus(window, cx));
        }

        let weak = cx.weak_entity();

        // chips row
        let mut chips = div()
            .h(px(52.))
            .flex_none()
            .flex()
            .items_center()
            .gap_2()
            .px_3();

        // date chip
        let date_label = self
            .qe_date
            .as_ref()
            .map(|d| fmt_day_month(d))
            .unwrap_or_else(|| "Срок".to_string());
        let has_date = self.qe_date.is_some();
        let mut date_chip = div()
            .id("qe-date")
            .h_7()
            .pl_2()
            .pr_2()
            .flex()
            .items_center()
            .gap_1p5()
            .rounded_full()
            .text_size(px(12.))
            .when(has_date, |el| {
                el.bg(c(QE_CHIP_BG)).text_color(c(QE_CHIP_FG))
            })
            .when(!has_date, |el| {
                el.bg(panel_mix(0.10)).text_color(c(PANEL_MUTED))
            })
            .child(icon("icons/calendar-02.svg", 13., if has_date { c(QE_CHIP_FG) } else { c(PANEL_MUTED) }))
            .child(date_label);
        if has_date {
            date_chip = date_chip.child(
                div()
                    .id("qe-date-clear")
                    .w_3p5()
                    .h_3p5()
                    .grid()
                    .items_center().justify_center()
                    .child(icon("icons/status-x.svg", 9., c(QE_CHIP_FG)))
                    .on_click({
                        let weak = weak.clone();
                        move |_: &ClickEvent, _, cx| {
                            let _ = weak.update(cx, |this, _| {
                                this.qe_date = None;
                                this.qe_date_touched = true;
                            });
                        }
                    }),
            );
        }
        let qe_date_t = self.hover_t(window, "qe-date");
        let _ = qe_date_t;
        chips = chips.child(date_chip.on_click({
            let weak = weak.clone();
            move |ev: &ClickEvent, _, cx| {
                let pos = ev.position();
                let _ = weak.update(cx, |this, _| {
                    this.dropdown = Some(DropState {
                        kind: DropKind::QeDate,
                        x: pos.x.into(),
                        y: pos.y.into(),
                        todo_id: None,
                    });
                });
            }
        }));

        // billable pill (white when on)
        let bill_t = self.hover_t(window, "qe-bill");
        let _ = bill_t;
        chips = chips.child(
            div()
                .id("qe-bill")
                .h_7()
                .px_3()
                .flex()
                .items_center()
                .gap_1p5()
                .rounded_full()
                .text_size(px(12.))
                .when(self.qe_billable, |el| {
                    el.bg(c(0xf5f5f5)).text_color(c(0x171717))
                })
                .when(!self.qe_billable, |el| {
                    el.bg(panel_mix(0.10)).text_color(c(PANEL_MUTED))
                })
                .child(icon("icons/dollar.svg", 13., if self.qe_billable { c(0x171717) } else { c(PANEL_MUTED) }))
                .child(if self.qe_billable { "Оплачиваемая" } else { "Без оплаты" })
                .on_click({
                    let weak = weak.clone();
                    move |_: &ClickEvent, _, cx| {
                        let _ = weak.update(cx, |this, _| {
                            this.qe_billable = !this.qe_billable;
                        });
                    }
                }),
        );

        // project pill (right-aligned, white)
        let proj_label = self
            .qe_project
            .as_ref()
            .and_then(|pid| self.projects.iter().find(|p| p.id == pid.as_str()))
            .map(|p| p.title.clone())
            .unwrap_or_else(|| "Входящие".to_string());
        chips = chips.child(div().flex_1()).child(
            div()
                .id("qe-proj")
                .h_7()
                .px_3()
                .flex()
                .items_center()
                .gap_1p5()
                .rounded_full()
                .bg(c(0xf5f5f5))
                .text_color(c(0x171717))
                .text_size(px(12.))
                .child(icon("icons/folder.svg", 13., c(0x404040)))
                .child(proj_label)
                .on_click({
                    let weak = weak.clone();
                    move |ev: &ClickEvent, _, cx| {
                        let pos = ev.position();
                        let _ = weak.update(cx, |this, _| {
                            this.dropdown = Some(DropState {
                                kind: DropKind::QeProject,
                                x: pos.x.into(),
                                y: pos.y.into(),
                                todo_id: None,
                            });
                        });
                    }
                }),
        );

        let panel = div()
            .w(px(640.))
            .flex()
            .flex_col()
            .rounded(px(10.))
            .overflow_hidden()
            .bg(c(PANEL))
            .border_1()
            .border_color(panel_mix(0.10))
            .shadow(vec![gpui::BoxShadow {
                color: rgba(0x000000, 0.35),
                offset: gpui::point(px(0.), px(16.)),
                blur_radius: px(48.),
                spread_radius: px(0.),
            }])
            .child(
                div()
                    .flex()
                    .items_center()
                    .pl_4()
                    .pr_2()
                    .child(
                        Input::new(&title_state)
                            .flex_1()
                            .h(px(48.))
                            .text_size(px(15.))
                            .text_color(c(PANEL_FG))
                            .appearance(false)
                            .bordered(false),
                    )
                    .child(
                        div()
                            .id("qe-close")
                            .w_7()
                            .h_7()
                            .grid()
                            .items_center().justify_center()
                            .rounded_md()
                            .child(icon("icons/status-x.svg", 14., c(PANEL_MUTED)))
                            .on_click({
                                let weak = weak.clone();
                                move |_: &ClickEvent, _, cx| {
                                    let _ = weak.update(cx, |this, _| {
                                        this.quick_entry_open = false;
                                        this.qe_focused = false;
                                    });
                                }
                            }),
                    ),
            )
            .child(
                div().pl_4().pr_2().child(
                    Input::new(&notes_state)
                        .w_full()
                        .h(px(36.))
                        .text_size(px(13.))
                        .text_color(c(PANEL_FG))
                        .appearance(false)
                        .bordered(false),
                ),
            )
            .child(div().h_px().w_full().bg(panel_mix(0.10)))
            .child(chips);

        // significance card (bottom-right white card)
        let sig_label = self
            .qe_sig
            .map(|s| format!("{}/10", s))
            .unwrap_or_else(|| "Не оценено".to_string());
        let fill = self.qe_sig.unwrap_or(5) as f32 / 10.0;
        let sig_card = div()
            .absolute()
            .right_6()
            .bottom_6()
            .px_4()
            .py_3()
            .rounded_lg()
            .border_1()
            .border_color(c(BORDER))
            .bg(c(BG))
            .shadow(vec![gpui::BoxShadow {
                color: rgba(0x000000, 0.15),
                offset: gpui::point(px(0.), px(12.)),
                blur_radius: px(32.),
                spread_radius: px(0.),
            }])
            .flex()
            .items_center()
            .gap(px(10.))
            .text_size(px(12.))
            .text_color(c(FG))
            .child("Значимость")
            .child(
                div()
                    .id("qe-sig-slider")
                    .w(px(120.))
                    .h(px(20.))
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .w_full()
                            .h(px(4.))
                            .rounded_full()
                            .bg(c(0xd4d4d4))
                            .child(
                                div()
                                    .h_full()
                                    .w(gpui::DefiniteLength::Fraction(fill))
                                    .rounded_full()
                                    .bg(c(ACCENT)),
                            ),
                    )
                    .child(
                        div()
                            .ml(px(-8. - 104. * (1.0 - fill)))
                            .w(px(16.))
                            .h(px(16.))
                            .rounded_full()
                            .bg(c(ACCENT)),
                    )
                    .on_click({
                        let weak = weak.clone();
                        move |ev: &ClickEvent, _, cx| {
                            let x: f32 = ev.position().x.into();
                            let _ = weak.update(cx, |this, _| {
                                // slider is 120px wide; anchor from window right edge
                                let v = ((x - (1440. - 24. - 16. - 96. - 120.)) / 120. * 10.)
                                    .round()
                                    .clamp(1., 10.) as u8;
                                this.qe_sig = Some(v);
                            });
                        }
                    }),
            )
            .child(
                div()
                    .text_color(c(MUTED_FG))
                    .child(sig_label),
            );

        div()
            .absolute()
            .inset_0()
            .bg(rgba(0x000000, 0.30))
            .flex()
            .justify_center()
            .child(
                div()
                    .id("qe-backdrop")
                    .absolute()
                    .inset_0()
                    .on_mouse_down(MouseButton::Left, move |_: &MouseDownEvent, _, cx| {
                        let _ = weak.update(cx, |this, _| {
                            this.quick_entry_open = false;
                            this.qe_focused = false;
                        });
                    }),
            )
            .child(div().mt(px(136.)).child(deferred(panel)))
            .child(deferred(sig_card))
            .into_any_element()
    }
}

// ---------------------------------------------------------------------------
// Small shared bits
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum OptAct {
    View(BoardView),
    Sort(SortKey),
    Group(GroupKey),
}

fn empty_state() -> gpui::Div {
    div()
        .flex_1()
        .min_h(px(160.))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_1()
        .child(
            div()
                .text_size(px(14.))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(c(MUTED_FG))
                .child("Ничего не найдено"),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(muted_fg_mix(0.8))
                .child("Здесь появятся задачи"),
        )
}

fn date_group_label(value: Option<&str>) -> String {
    match value {
        None => "Без даты".to_string(),
        Some(v) if v == today_key() => "Сегодня".to_string(),
        Some(v) => fmt_day_month(v),
    }
}
