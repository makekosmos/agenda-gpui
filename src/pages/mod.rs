//! Page renderers + shared task widgets (TaskRow/TaskBoard parity),
//! quick search / quick entry / property dropdown overlays.

mod board;
mod calendar;
mod calendar_day;
mod chips;
mod dropdown;
mod fab;
mod logbook;
mod options;
mod project;
mod quick_entry;
mod quick_search;
mod recurrence;
mod recurring;
mod rows;
mod settings;
mod statistics;
mod task;
mod task_props;
mod trash;

pub(crate) use chrono::{Datelike, Duration, Local};
pub(crate) use gpui::{
    deferred, div, prelude::*, px, AnyElement, ClickEvent, Context, MouseButton, MouseDownEvent,
    SharedString, Window,
};
pub(crate) use gpui_component::input::{Input, Textarea};

pub(crate) use crate::app::{
    Agenda, BoardView, CalMode, CtxMenu, DropKind, DropState, MenuAction, Route,
};
pub(crate) use crate::chrome::CAPTION_W;
pub(crate) use crate::model::*;
pub(crate) use crate::theme::*;
pub(crate) use crate::widgets::*;

fn panel_mix(a: f32) -> HslaAlias {
    mix(0xffffff, a, POPOVER())
}

// Alias so we can write Hsla without importing gpui::Hsla in every signature.
pub type HslaAlias = gpui::Hsla;

impl Agenda {
    // ==========================================================================
    // Page dispatch
    // ==========================================================================

    pub(crate) fn render_page(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let route = self.route.clone();
        let content: AnyElement = match route {
            Route::Inbox => {
                self.board_page(SmartList::Inbox, "agenda.inbox.view", true, window, cx)
            }
            Route::Today => {
                self.board_page(SmartList::Today, "agenda.today.view", true, window, cx)
            }
            Route::Plans => {
                self.board_page(SmartList::Plans, "agenda.plans.view", false, window, cx)
            }
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

        // Primary FAB on list pages.
        if matches!(
            route,
            Route::Inbox | Route::Today | Route::Plans | Route::Someday | Route::Project(_)
        ) {
            page = page.child(self.render_fab(window, cx));
        }
        // Display-options popover is anchored under the titlebar options
        // button (top-right, left of the native caption controls).
        if self.options_for.is_some() {
            page = page.child(self.render_options_popover(window, cx).into_any_element());
        }
        if let Some(drop) = self.dropdown.clone() {
            page = page.child(self.render_dropdown(&drop, window, cx).into_any_element());
        }
        page.into_any_element()
    }

    /// Status select chip on rows (task-row__actions select parity).
    pub(crate) fn row_status_chip(
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
            .bg(c(SECONDARY()))
            .text_size(px(12.))
            .text_color(mix(FG(), 0.7 + 0.3 * ht, BG()))
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
}

// ---------------------------------------------------------------------------
// Small shared bits
// ---------------------------------------------------------------------------

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
                .text_color(c(MUTED_FG()))
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
