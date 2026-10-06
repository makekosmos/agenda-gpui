//! Page renderers + shared task widgets (TaskRow/TaskBoard parity),
//! quick search / quick entry / property dropdown overlays.

mod board;
mod dev;
mod dropdown;
mod fab;
mod logbook;
mod options;
mod project;
mod quick_entry;
mod quick_search;
mod rows;
mod settings;
mod statistics;
mod trash;

pub(crate) use chrono::{Datelike, Duration, Local, NaiveDate};
pub(crate) use gpui::{
    deferred, div, prelude::*, px, AnyElement, ClickEvent, Context, MouseButton, MouseDownEvent,
    SharedString, Window,
};
pub(crate) use gpui_component::input::{Input, Textarea};

pub(crate) use crate::app::{
    storage::{StorageError, StorageFault},
    Agenda, BoardView, CtxMenu, DropKind, DropState, MenuAction, Route,
};
#[cfg(not(target_os = "macos"))]
pub(crate) use crate::chrome::CAPTION_W;
pub(crate) use crate::model::*;
pub(crate) use crate::theme::*;
pub(crate) use crate::widgets::*;
pub(crate) use rows::{FlatRow, RowCache};

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
            Route::Week => self.board_page(SmartList::Week, "agenda.week.view", true, window, cx),
            Route::NextWeek => self.board_page(
                SmartList::NextWeek,
                "agenda.next-week.view",
                true,
                window,
                cx,
            ),
            Route::Someday => {
                self.board_page(SmartList::Someday, "agenda.someday.view", false, window, cx)
            }
            Route::Logbook => self.logbook_page(window, cx),
            Route::Trash => self.trash_page(window, cx),
            Route::Statistics => self.statistics_page(window, cx),
            Route::Settings => self.settings_page(false, window, cx),
            Route::SettingsFuel => self.settings_page(true, window, cx),
            Route::SettingsEnergy => self.energy_page(window, cx),
            Route::Dev => self.dev_page(window, cx),
            Route::About => self.about_page(cx),
            Route::Project(ref id) => self.project_page(id, window, cx),
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
            Route::Inbox | Route::Week | Route::NextWeek | Route::Someday | Route::Project(_)
        ) {
            page = page.child(self.render_fab(window, cx));
        }
        // Display-options popover is anchored under the titlebar options
        // button (top-right, left of the native caption controls).
        if self.options_for.is_some() {
            page = page.child(self.render_options_popover(window, cx).into_any_element());
        }
        page.into_any_element()
    }
}

// ---------------------------------------------------------------------------
// Small shared bits
// ---------------------------------------------------------------------------

/// Linear-style empty state: thin hugeicons glyph, short title, one-line
/// hint. Every page supplies its own copy via [`empty_state_for`].
fn empty_state(icon_path: &'static str, title: &'static str, hint: &'static str) -> gpui::Div {
    div()
        .flex_1()
        .min_h(px(160.))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_1()
        .child(div().pb_1().child(icon(icon_path, 40., muted_fg_mix(0.45))))
        .child(
            div()
                .text_size(crate::theme::text_px(14.))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(c(MUTED_FG()))
                .child(title),
        )
        .child(
            div()
                .text_size(crate::theme::text_px(12.))
                .text_color(muted_fg_mix(0.8))
                .child(hint),
        )
}

/// Per-page empty-state copy — title/hint in Linear's laconic style.
fn empty_state_for(list: SmartList) -> gpui::Div {
    match list {
        SmartList::Inbox => empty_state(
            "icons/inbox.svg",
            "Входящие пустые",
            "Создай любую задачу и она появится здесь",
        ),
        SmartList::Week => empty_state(
            "icons/calendar-01.svg",
            "На этой неделе пусто",
            "Назначь задачу на эту неделю",
        ),
        SmartList::NextWeek => empty_state(
            "icons/calendar-02.svg",
            "На следующей неделе пусто",
            "Поставь задаче дату следующей недели",
        ),
        SmartList::Someday => empty_state(
            "icons/clock-01.svg",
            "Потом пуст",
            "Отложенные задачи появятся здесь",
        ),
        SmartList::Logbook => empty_state(
            "icons/book-open.svg",
            "Архив пуст",
            "Завершённые задачи хранятся здесь",
        ),
        SmartList::Trash => empty_state(
            "icons/delete.svg",
            "Корзина пуста",
            "Удалённые задачи попадают сюда",
        ),
    }
}

fn date_group_label(value: Option<&str>) -> String {
    match value {
        None => "Без даты".to_string(),
        Some(v) if v == today_key() => "Сегодня".to_string(),
        Some(v) => fmt_day_month(v),
    }
}
