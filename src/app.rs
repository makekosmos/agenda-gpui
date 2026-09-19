// Root view: chrome (sidebar + titlebar), routing, pages, overlays.
use std::collections::HashMap;
use std::time::Instant;

use gpui::{
    div, prelude::*, AnyElement, Context, Entity, FocusHandle, Hsla, KeyDownEvent, ScrollHandle,
    SharedString, Subscription, Window,
};
use gpui_component::input::{InputState, TextareaState};

use crate::model::*;
use crate::theme::*;

const HOVER_MS: f32 = 120.0;

#[derive(Clone, Debug, PartialEq)]
pub enum Route {
    Inbox,
    Today,
    Plans,
    Calendar,
    Someday,
    Statistics,
    Recurring,
    Logbook,
    Trash,
    Settings,
    SettingsFuel,
    About,
    Project(String),
    Task(String),
}

#[derive(Clone, Copy, PartialEq)]
pub enum BoardView {
    List,
    Kanban,
}

#[derive(Clone, Copy)]
pub struct BoardOpts {
    pub view: BoardView,
    pub sort: SortKey,
    pub group: GroupKey,
}

impl Default for BoardOpts {
    fn default() -> Self {
        Self {
            view: BoardView::List,
            sort: SortKey::Default,
            group: GroupKey::None,
        }
    }
}

#[derive(Clone)]
pub(crate) struct HoverAnim {
    value: f32,
    target: bool,
    stamp: Instant,
}

#[derive(Clone)]
pub enum MenuAction {
    TrashTodo(String),
    #[allow(dead_code)]
    OpenTask(String),
    RestoreProject(String),
    ArchiveProject(String),
    DeleteProject(String),
    RenameProject(String),
    SetStatus(String, Status),
    SetPriority(String, u8),
    SetProject(String, Option<String>),
    AddTag(String, String),
    SetSignificance(String, Option<u8>),
    SetBillable(String, bool),
    SetDate(String, Option<String>),
    RestoreTodo(String),
    MoveToToday(String),
    #[allow(dead_code)]
    DeferTodo(String),
    CompleteTodo(String),
    RemoveTag(String, String),
    QeSetProject(Option<String>),
    QeSetDate(Option<String>),
    RecurSetFreq(u8),
    RecurSetType(u8),
    Noop,
}

#[derive(Clone)]
pub struct SbHighlight {
    pub y: f32,
    pub hh: f32,
    pub ty: f32,
    pub th: f32,
    /// Slide origins anchored at the last retarget — the tick interpolates
    /// y0→ty / h0→th over a fixed 160ms window.
    pub y0: f32,
    pub h0: f32,
    pub opacity: f32,
    pub visible: bool,
    /// Retarget anchor for the slide progress.
    pub stamp: Instant,
    /// Per-frame stamp for the opacity fade rate.
    pub tick_stamp: Instant,
}

#[derive(Clone)]
pub struct CtxMenu {
    pub x: f32,
    pub y: f32,
    pub items: Vec<(String, bool, MenuAction)>, // label, destructive, action
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum DropKind {
    Status,
    Priority,
    Project,
    Tags,
    Significance,
    Billable,
    Date,
    QeProject,
    QeDate,
    RecurFreq,
    RecurType,
}

/// Open prop-dropdown panel state: kind + anchor point + owning todo.
#[derive(Clone)]
pub(crate) struct DropState {
    pub kind: DropKind,
    pub x: f32,
    pub y: f32,
    pub todo_id: Option<String>,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum CalMode {
    Day,
    Week,
}

pub struct Agenda {
    pub(crate) todos: Vec<Todo>,
    pub(crate) projects: Vec<Project>,
    // Seeded model state kept for Agenda parity; not rendered yet.
    #[allow(dead_code)]
    pub(crate) areas: Vec<Area>,
    pub(crate) tags: Vec<Tag>,
    #[allow(dead_code)]
    pub(crate) headings: Vec<Heading>,
    pub(crate) events: Vec<CalEvent>,

    pub(crate) route: Route,
    pub(crate) back_route: Route,

    pub(crate) sidebar_t: f32,
    pub(crate) sidebar_target: f32,
    pub(crate) sidebar_stamp: Instant,

    pub(crate) hovers: HashMap<SharedString, HoverAnim>,
    pub(crate) options_open: bool,
    pub(crate) more_open: bool,
    /// `more_open` as of the button's mouse_down. The click-away backdrop may
    /// already close the menu on press; the click must not reopen it.
    pub(crate) more_down_was_open: bool,
    pub(crate) menu: Option<CtxMenu>,
    pub(crate) dropdown: Option<DropState>,
    pub(crate) sb_hover_id: Option<SharedString>,
    pub(crate) sb_highlight: SbHighlight,
    pub(crate) quick_entry_open: bool,
    pub(crate) qe_billable: bool,
    pub(crate) qe_project: Option<String>,
    pub(crate) qe_menu_open: bool,
    pub(crate) qe_date: Option<String>,
    pub(crate) qe_date_touched: bool,
    pub(crate) qe_sig: Option<u8>,
    pub(crate) kanban_group_open: bool,
    pub(crate) group_open_t: f32,
    pub(crate) group_open_stamp: Instant,
    pub(crate) more_menu_t: f32,
    pub(crate) more_menu_stamp: Instant,

    pub(crate) recur_open: bool,
    pub(crate) recur_freq: u8,
    pub(crate) recur_interval: u32,
    pub(crate) recur_type: u8,
    pub(crate) recur_days: Vec<u8>,
    #[allow(dead_code)]
    pub(crate) recur_day_of_month: Option<u8>,

    pub(crate) theme_sel: u8, // 0 light 1 dark 2 system
    pub(crate) theme_idx: usize,
    pub(crate) sb_material: u8, // 0 solid 1 acrylic 2 mica
    pub(crate) applied_material: Option<u8>,
    pub(crate) applied_theme_sel: Option<u8>,
    pub(crate) delete_blocked: bool,

    pub(crate) scrolls: HashMap<String, ScrollHandle>,
    pub(crate) inputs: HashMap<String, Entity<InputState>>,
    /// Single-line keys live in `inputs`; the task notes field is multi-line,
    /// which in gpui-base 0.6 is a distinct entity type (`TextareaState`).
    pub(crate) notes_input: Option<Entity<TextareaState>>,
    pub(crate) input_task: Option<String>,
    pub(crate) _subs: Vec<Subscription>,

    pub(crate) boards: HashMap<String, BoardOpts>,
    pub(crate) options_for: Option<String>,

    pub(crate) quick_open: bool,
    pub(crate) quick_query: String,
    pub(crate) quick_sel: usize,

    pub(crate) cal_mode: CalMode,
    pub(crate) cal_anchor: String,

    pub(crate) stat_metric: u8, // 0 count 1 significance 2 fuel
    #[allow(dead_code)]
    pub(crate) stat_day: Option<String>,

    pub(crate) root_focus: FocusHandle,
    #[allow(dead_code)]
    pub(crate) quick_focus: FocusHandle,
    pub(crate) focused_once: bool,
    pub(crate) qs_focused: bool,
    pub(crate) qe_focused: bool,
}

/// Screenshot/QA hook: `AGENDA_ROUTE=<route>` selects the initial route.
fn initial_route() -> Route {
    match std::env::var("AGENDA_ROUTE").ok().as_deref() {
        Some("today") => Route::Today,
        Some("plans") => Route::Plans,
        Some("calendar") => Route::Calendar,
        Some("someday") => Route::Someday,
        Some("statistics") => Route::Statistics,
        Some("recurring") => Route::Recurring,
        Some("logbook") => Route::Logbook,
        Some("trash") => Route::Trash,
        Some("settings") => Route::Settings,
        Some("settings-fuel") => Route::SettingsFuel,
        Some("about") => Route::About,
        Some(r) if r.starts_with("task/") => Route::Task(r[5..].to_string()),
        Some(r) if r.starts_with("project/") => Route::Project(r[8..].to_string()),
        _ => Route::Inbox,
    }
}

impl Agenda {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            todos: seed_todos(),
            projects: seed_projects(),
            areas: seed_areas(),
            tags: seed_tags(),
            headings: seed_headings(),
            events: seed_events(),
            route: initial_route(),
            back_route: Route::Inbox,
            sidebar_t: 1.0,
            sidebar_target: 1.0,
            sidebar_stamp: Instant::now(),
            hovers: HashMap::new(),
            options_open: false,
            more_open: std::env::var("AGENDA_OVERLAY").ok().as_deref() == Some("more"),
            more_down_was_open: false,
            menu: None,
            dropdown: None,
            sb_hover_id: None,
            sb_highlight: SbHighlight {
                y: 0.0,
                hh: 32.0,
                ty: 0.0,
                th: 32.0,
                y0: 0.0,
                h0: 32.0,
                opacity: 0.0,
                visible: false,
                stamp: Instant::now(),
                tick_stamp: Instant::now(),
            },
            quick_entry_open: std::env::var("AGENDA_OVERLAY").ok().as_deref() == Some("quickentry"),
            qe_billable: true,
            qe_project: None,
            qe_menu_open: false,
            qe_date: None,
            qe_date_touched: false,
            qe_sig: None,
            kanban_group_open: true,
            group_open_t: 1.0,
            group_open_stamp: Instant::now(),
            more_menu_t: 0.0,
            more_menu_stamp: Instant::now(),
            recur_open: false,
            recur_freq: 0,
            recur_interval: 1,
            recur_type: 0,
            recur_days: vec![],
            recur_day_of_month: None,
            theme_sel: 1,
            theme_idx: 0,
            sb_material: 0,
            applied_material: None,
            applied_theme_sel: None,
            delete_blocked: false,
            scrolls: HashMap::new(),
            inputs: HashMap::new(),
            notes_input: None,
            input_task: None,
            _subs: vec![],
            boards: HashMap::new(),
            options_for: std::env::var("AGENDA_OVERLAY")
                .ok()
                .as_deref()
                .and_then(|o| {
                    if o == "options" {
                        Some("agenda.today.view".to_string())
                    } else {
                        None
                    }
                }),
            quick_open: std::env::var("AGENDA_OVERLAY").ok().as_deref() == Some("quicksearch"),
            quick_query: String::new(),
            quick_sel: 0,
            cal_mode: CalMode::Week,
            cal_anchor: today_key(),
            stat_metric: 0,
            stat_day: None,
            root_focus: cx.focus_handle(),
            quick_focus: cx.focus_handle(),
            focused_once: false,
            qs_focused: false,
            qe_focused: false,
        }
    }

    // ------------------------------------------------------------------
    // Hover transition machinery — replicates CSS `transition: X 120ms ease`.
    // `on_hover` events set the target; each render advances value and
    // requests another frame while mid-flight.
    // ------------------------------------------------------------------

    pub(crate) fn set_hover(&mut self, id: &str, hovered: bool) {
        let entry = self
            .hovers
            .entry(SharedString::from(id.to_string()))
            .or_insert(HoverAnim {
                value: 0.0,
                target: false,
                stamp: Instant::now(),
            });
        if entry.target != hovered {
            entry.target = hovered;
            entry.stamp = Instant::now();
        }
    }

    /// Returns eased hover amount 0..1; schedules repaint while animating.
    pub(crate) fn hover_t(&mut self, window: &mut Window, id: &str) -> f32 {
        let animating;
        let value;
        {
            let now = Instant::now();
            let e = self
                .hovers
                .entry(SharedString::from(id.to_string()))
                .or_insert(HoverAnim {
                    value: 0.0,
                    target: false,
                    stamp: now,
                });
            let dt = now.duration_since(e.stamp).as_secs_f32() * 1000.0;
            e.stamp = now;
            let step = dt / HOVER_MS;
            if e.target {
                e.value = (e.value + step).min(1.0);
            } else {
                e.value = (e.value - step).max(0.0);
            }
            value = e.value;
            animating = value > 0.0 && value < 1.0;
        }
        if animating {
            window.request_animation_frame();
        }
        ease_standard(value)
    }

    pub(crate) fn sidebar_progress(&mut self, window: &mut Window) -> f32 {
        let now = Instant::now();
        let dt = now.duration_since(self.sidebar_stamp).as_secs_f32();
        self.sidebar_stamp = now;
        let step = dt / 0.33;
        if self.sidebar_target > self.sidebar_t {
            self.sidebar_t = (self.sidebar_t + step).min(self.sidebar_target);
        } else if self.sidebar_target < self.sidebar_t {
            self.sidebar_t = (self.sidebar_t - step).max(self.sidebar_target);
        }
        if (self.sidebar_t - self.sidebar_target).abs() > 0.0005 {
            window.request_animation_frame();
        }
        ease_emphasized(self.sidebar_t.clamp(0.0, 1.0))
    }

    /// Sidebar shell background for the selected material. "Solid" is the
    /// opaque theme color; acrylic gets a strong theme tint (native acrylic
    /// stays readable over the blurred desktop — the material shows as
    /// texture, not as the dominant color); mica/vibrancy is already a flat
    /// readable surface, so it only gets a faint tint or it would look like
    /// a dirty smear instead of the system material.
    pub(crate) fn sidebar_surface(&self) -> Hsla {
        if !self.backdrop_active() {
            return c(SIDEBAR_BG());
        }
        match self.sb_material {
            1 => rgba(SIDEBAR_BG(), 0.8),
            2 => rgba(SIDEBAR_BG(), 0.12),
            _ => c(SIDEBAR_BG()),
        }
    }

    /// Whether the window has a real system backdrop active behind the
    /// sidebar (acrylic/mica/vibrancy on Windows/macOS).
    pub(crate) fn backdrop_active(&self) -> bool {
        self.sb_material != 0 && cfg!(any(target_os = "windows", target_os = "macos"))
    }

    /// Mica on Windows; vibrancy-style blur on macOS; opaque elsewhere.
    fn mica_appearance() -> gpui::WindowBackgroundAppearance {
        if cfg!(target_os = "windows") {
            gpui::WindowBackgroundAppearance::MicaBackdrop
        } else if cfg!(target_os = "macos") {
            gpui::WindowBackgroundAppearance::Blurred
        } else {
            gpui::WindowBackgroundAppearance::Opaque
        }
    }

    /// Drop retained hover targets. Elements that unmount while hovered never
    /// receive their `on_hover(false)`, which would otherwise leave the state
    /// stuck "on" until the next mouse move. The element under the cursor is
    /// kept: its `was_hovered` stays true, so no `on_hover(true)` will refire
    /// to restore it.
    pub(crate) fn reset_hovers(&mut self, keep: Option<&str>) {
        for (id, h) in self.hovers.iter_mut() {
            let pinned = self.sb_hover_id.as_ref() == Some(id) || keep == Some(&**id);
            if h.target && !pinned {
                h.target = false;
                h.stamp = Instant::now();
            }
        }
    }

    pub(crate) fn navigate(&mut self, route: Route) {
        if self.route != route {
            self.back_route = self.route.clone();
            self.route = route;
            self.options_open = false;
            self.options_for = None;
            if self.more_open {
                self.more_menu_stamp = Instant::now();
            }
            self.more_open = false;
            self.menu = None;
            self.dropdown = None;
            self.reset_hovers(None);
        }
    }

    pub(crate) fn board_opts(&self, key: &str) -> BoardOpts {
        self.boards.get(key).copied().unwrap_or_default()
    }

    /// Board-options storage key for the current route, or `None` on pages
    /// without display options (Trash, Calendar, Settings, task, ...).
    pub(crate) fn view_options_key(&self) -> Option<String> {
        match &self.route {
            Route::Inbox => Some("agenda.inbox.view".to_string()),
            Route::Today => Some("agenda.today.view".to_string()),
            Route::Plans => Some("agenda.plans.view".to_string()),
            Route::Someday => Some("agenda.someday.view".to_string()),
            Route::Logbook => Some("agenda.logbook.view".to_string()),
            Route::Project(id) => Some(format!("agenda.project.{id}.view")),
            _ => None,
        }
    }

    pub(crate) fn project<'a>(&'a self, id: &str) -> Option<&'a Project> {
        self.projects.iter().find(|p| p.id == id)
    }

    pub(crate) fn tag<'a>(&'a self, id: &str) -> Option<&'a Tag> {
        self.tags.iter().find(|t| t.id == id)
    }

    pub(crate) fn todo<'a>(&'a self, id: &str) -> Option<&'a Todo> {
        self.todos.iter().find(|t| t.id == id)
    }

    pub(crate) fn page_title(&self) -> (SharedString, &'static str) {
        match &self.route {
            Route::Inbox => ("Входящие".into(), "icons/inbox.svg"),
            Route::Today => ("Сегодня".into(), "icons/calendar-01.svg"),
            Route::Plans => ("Планы".into(), "icons/calendar-02.svg"),
            Route::Calendar => ("Календарь".into(), "icons/calendar-02.svg"),
            Route::Someday => ("Потом".into(), "icons/clock-01.svg"),
            Route::Statistics => ("Статистика".into(), "icons/analytics-01.svg"),
            Route::Recurring => ("Повторяющиеся".into(), "icons/repeat.svg"),
            Route::Logbook => ("Архив".into(), "icons/book-open.svg"),
            Route::Trash => ("Корзина".into(), "icons/delete.svg"),
            Route::Settings | Route::SettingsFuel => ("Настройки".into(), "icons/settings.svg"),
            Route::About => ("О приложении".into(), "icons/help-circle.svg"),
            Route::Project(id) => (
                self.project(id)
                    .map(|p| p.title.clone())
                    .unwrap_or_else(|| "Проект".into())
                    .into(),
                "icons/folder-open.svg",
            ),
            Route::Task(id) => (
                self.todo(id)
                    .map(|t| t.title.clone())
                    .unwrap_or_else(|| "Задача".into())
                    .into(),
                "icons/task-01.svg",
            ),
        }
    }
}

// ===========================================================================
// Render
// ===========================================================================

impl Render for Agenda {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.focused_once {
            self.focused_once = true;
            self.root_focus.focus(window, cx);
        }
        let sidebar_p = self.sidebar_progress(window);

        // Resolve theme mode ("system" follows the OS appearance) and material.
        let sys_dark = matches!(
            window.appearance(),
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
        );
        let dark = match self.theme_sel {
            0 => false,
            2 => sys_dark,
            _ => true,
        };
        set_mode(dark);
        set_theme(self.theme_idx);
        if self.applied_material != Some(self.sb_material) {
            self.applied_material = Some(self.sb_material);
            window.set_background_appearance(match self.sb_material {
                1 if cfg!(any(target_os = "windows", target_os = "macos")) => {
                    gpui::WindowBackgroundAppearance::Blurred
                }
                2 => Self::mica_appearance(),
                _ => gpui::WindowBackgroundAppearance::Opaque,
            });
        }
        // Pin the window's light/dark appearance to the selected theme mode so
        // Mica/Acrylic tint and system-chrome rendering follow the app theme,
        // not just the OS. "System" clears the override.
        if self.applied_theme_sel != Some(self.theme_sel) {
            self.applied_theme_sel = Some(self.theme_sel);
            cx.set_window_appearance(match self.theme_sel {
                0 => Some(gpui::WindowAppearance::Light),
                1 => Some(gpui::WindowAppearance::Dark),
                _ => None,
            });
        }

        let weak = cx.weak_entity();
        let root = div()
            .id("agenda-root")
            .track_focus(&self.root_focus)
            .size_full()
            .flex()
            .relative()
            .overflow_hidden()
            .when(!self.backdrop_active(), |d| d.bg(c(BG())))
            .text_color(c(FG()))
            .font_family("Inter")
            .on_key_down({
                let weak = weak.clone();
                move |ev: &KeyDownEvent, window, cx| {
                    let _ = weak.update(cx, |this, cx| this.on_key(ev, window, cx));
                }
            })
            .child(self.render_sidebar(sidebar_p, window, cx))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .bg(c(BG()))
                    .child(self.render_titlebar(window, cx))
                    .child(self.render_page(window, cx)),
            )
            .child(self.render_sidebar_toggle(window, cx));

        // overlays
        let mut overlays: Vec<AnyElement> = vec![];
        if let Some(menu) = self.menu.clone() {
            overlays.push(self.render_ctx_menu(&menu, window, cx).into_any_element());
        }
        if self.more_open || self.more_menu_t > 0.001 {
            overlays.push(self.render_more_menu(window, cx).into_any_element());
        }
        if self.quick_entry_open {
            overlays.push(self.render_quick_entry(window, cx).into_any_element());
        }
        if self.quick_open {
            overlays.push(self.render_quick_search(window, cx).into_any_element());
        }

        root.children(overlays)
    }
}

impl Agenda {
    pub(crate) fn on_key(
        &mut self,
        ev: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let k = &ev.keystroke;
        let ctrl = k.modifiers.control || k.modifiers.platform;
        if ctrl && k.key == "k" {
            self.quick_open = !self.quick_open;
            if self.quick_open {
                self.quick_query.clear();
                self.quick_sel = 0;
            } else {
                self.qs_focused = false;
                self.root_focus.focus(window, cx);
            }
            return;
        }
        if ctrl && k.key == "b" {
            self.sidebar_target = if self.sidebar_target > 0.5 { 0.0 } else { 1.0 };
            self.sidebar_stamp = Instant::now();
            return;
        }
        if ctrl && k.key == "n" && !k.modifiers.shift && !k.modifiers.alt {
            self.quick_entry_open = !self.quick_entry_open;
            self.qe_menu_open = false;
            if self.quick_entry_open {
                self.qe_billable = false;
                self.qe_sig = None;
                self.qe_date = None;
                self.qe_date_touched = false;
                self.qe_project = match &self.route {
                    Route::Project(id) => Some(id.clone()),
                    _ => None,
                };
            } else {
                self.qe_focused = false;
            }
            return;
        }
        if self.quick_open {
            match k.key.as_str() {
                "escape" => {
                    self.quick_open = false;
                    self.qs_focused = false;
                    self.root_focus.focus(window, cx);
                }
                "down" => {
                    let (todos, projects) = self.quick_matches();
                    let n = todos.len() + projects.len();
                    if n > 0 {
                        self.quick_sel = (self.quick_sel + 1).min(n - 1);
                    }
                }
                "up" => {
                    self.quick_sel = self.quick_sel.saturating_sub(1);
                }
                _ => {}
            }
            return;
        }
        if k.key == "escape" {
            self.options_for = None;
            if self.more_open {
                self.more_menu_stamp = Instant::now();
            }
            self.more_open = false;
            self.menu = None;
            self.dropdown = None;
            if self.quick_entry_open {
                self.quick_entry_open = false;
                self.qe_focused = false;
            }
            self.qe_menu_open = false;
            self.recur_open = false;
        }
    }

    /// InputState events (gpui-component): Enter saves, Blur persists fields,
    /// Change keeps derived state in sync (selection reset on new query).
    pub(crate) fn on_input_event(
        &mut self,
        key: &str,
        ev: &gpui_component::input::InputEvent,
        cx: &mut Context<Self>,
    ) {
        use gpui_component::input::InputEvent;
        match (key, ev) {
            ("qs", InputEvent::PressEnter { .. }) => {
                self.quick_select_current();
            }
            ("qs", InputEvent::Change) => {
                self.quick_sel = 0;
            }
            ("task-title", InputEvent::Blur) | ("task-title", InputEvent::PressEnter { .. }) => {
                self.save_task_title(cx);
            }
            ("task-notes", InputEvent::Blur) => {
                self.save_task_notes(cx);
            }
            ("qe-title", InputEvent::PressEnter { .. }) => {
                self.quick_entry_save(cx);
            }
            ("qe-title", InputEvent::Change) => {
                // @упоминание проекта в заголовке (parseProjectMention parity)
                if let Some(state) = self.inputs.get("qe-title") {
                    let raw = state.read(cx).value().to_string();
                    if self.qe_project.is_none() {
                        let (clean, pid) = parse_project_mention(&raw, &self.projects);
                        if let Some(pid) = pid {
                            self.qe_project = Some(pid);
                        }
                        let _ = clean;
                    }
                }
            }
            _ => {}
        }
    }

    fn quick_select_current(&mut self) {
        let (todos, projects) = self.quick_matches();
        if let Some(t) = todos.get(self.quick_sel) {
            let id = t.id.to_string();
            self.quick_open = false;
            self.navigate(Route::Task(id));
        } else if let Some(p) = projects.get(self.quick_sel.saturating_sub(todos.len())) {
            let id = p.id.to_string();
            self.quick_open = false;
            self.navigate(Route::Project(id));
        }
    }

    fn save_task_title(&mut self, cx: &mut Context<Self>) {
        let Some(tid) = self.current_task_id() else {
            return;
        };
        let Some(state) = self.inputs.get("task-title") else {
            return;
        };
        let title = state.read(cx).value().trim().to_string();
        if title.is_empty() {
            return;
        }
        if let Some(t) = self.todos.iter_mut().find(|t| t.id == tid) {
            t.title = title;
        }
    }

    fn save_task_notes(&mut self, cx: &mut Context<Self>) {
        let Some(tid) = self.current_task_id() else {
            return;
        };
        let Some(state) = &self.notes_input else {
            return;
        };
        let notes = state.read(cx).value().trim().to_string();
        if let Some(t) = self.todos.iter_mut().find(|t| t.id == tid) {
            t.notes = if notes.is_empty() { None } else { Some(notes) };
        }
    }

    pub(crate) fn current_task_id(&self) -> Option<String> {
        match &self.route {
            Route::Task(id) => Some(id.clone()),
            _ => None,
        }
    }

    /// Quick entry save: title/notes from inputs, date/project/billable/sig from state.
    fn quick_entry_save(&mut self, cx: &mut Context<Self>) {
        let title = self
            .inputs
            .get("qe-title")
            .map(|s| s.read(cx).value().to_string())
            .unwrap_or_default();
        let notes = self
            .inputs
            .get("qe-notes")
            .map(|s| s.read(cx).value().to_string())
            .unwrap_or_default();
        let title = title.trim().to_string();
        if title.is_empty() {
            return;
        }
        let default_date = if self.route == Route::Today {
            Some(today_key())
        } else {
            None
        };
        let uses_contextual =
            !self.qe_date_touched && default_date.is_some() && self.qe_date == default_date;
        let captured = parse_quick_entry_capture(
            &title,
            if uses_contextual {
                None
            } else {
                self.qe_date.clone()
            },
        );
        let scheduled = if uses_contextual && captured.1.is_none() {
            default_date
        } else {
            captured.1
        };
        let (clean_title, parsed_pid) = if self.qe_project.is_some() {
            (captured.0, None)
        } else {
            parse_project_mention(&captured.0, &self.projects)
        };
        let project = self.qe_project.clone().or(parsed_pid);
        let status = if scheduled.is_some() || project.is_some() {
            Status::Todo
        } else {
            Status::Inbox
        };
        let pid: Option<&'static str> = match project.as_deref() {
            Some("dev-proj-release") => Some("dev-proj-release"),
            Some("dev-proj-home") => Some("dev-proj-home"),
            Some("dev-proj-someday") => Some("dev-proj-someday"),
            _ => None,
        };
        let mut t = new_todo(format!("qe-{}", self.todos.len()), &clean_title);
        t.notes = if notes.trim().is_empty() {
            None
        } else {
            Some(notes.trim().to_string())
        };
        t.scheduled_date = scheduled;
        t.project_id = pid;
        t.status = status;
        t.billable = self.qe_billable;
        t.significance = self.qe_sig;
        t.sort_order = self.todos.len() as i32;
        t.created_at = today_key();
        self.todos.push(t);
        self.quick_entry_open = false;
    }

    /// Lazily create an InputState; subscriptions dispatch on `key`.
    pub(crate) fn input_state(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        key: &str,
        placeholder: impl Into<SharedString>,
    ) -> Entity<InputState> {
        if let Some(s) = self.inputs.get(key) {
            return s.clone();
        }
        let st = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
        let k = key.to_string();
        self._subs.push(cx.subscribe(
            &st,
            move |this, _s, ev: &gpui_component::input::InputEvent, cx| {
                this.on_input_event(&k, ev, cx);
            },
        ));
        self.inputs.insert(key.to_string(), st.clone());
        st
    }

    /// Lazily create the multi-line task notes state ("task-notes" key).
    pub(crate) fn notes_state(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        placeholder: impl Into<SharedString>,
    ) -> Entity<TextareaState> {
        if let Some(s) = &self.notes_input {
            return s.clone();
        }
        let st = cx.new(|cx| TextareaState::new(window, cx).placeholder(placeholder));
        self._subs.push(cx.subscribe(
            &st,
            move |this, _s, ev: &gpui_component::input::InputEvent, cx| {
                this.on_input_event("task-notes", ev, cx);
            },
        ));
        self.notes_input = Some(st.clone());
        st
    }

    pub(crate) fn scroll(&mut self, key: &str) -> ScrollHandle {
        self.scrolls.entry(key.to_string()).or_default().clone()
    }

    /// Todo mutations mirroring src/store/todos.ts (local, no ARK).
    pub(crate) fn set_todo_status(&mut self, id: &str, status: Status) {
        if status == Status::Done {
            self.complete_todo(id);
            return;
        }
        if let Some(t) = self.todos.iter_mut().find(|t| t.id == id) {
            t.status = status;
            t.is_completed = false;
            t.completed_at = if status == Status::Canceled {
                Some(format!("{}T12:00:00", today_key()))
            } else {
                None
            };
            t.is_cancelled = status == Status::Canceled;
            t.is_someday = status == Status::Deferred;
            t.is_today = false;
        }
    }

    pub(crate) fn complete_todo(&mut self, id: &str) {
        let n = self.todos.len();
        let Some(t) = self.todos.iter_mut().find(|t| t.id == id) else {
            return;
        };
        if t.is_completed {
            return;
        }
        let had_recurrence = t.recurrence.clone();
        t.status = Status::Done;
        t.is_completed = true;
        t.completed_at = Some(format!("{}T12:00:00", today_key()));
        // Recurrence: create next instance (createNextRecurrence simplified).
        if let Some(rule) = had_recurrence {
            let mut next = new_todo(format!("{}-next-{}", t.id, n), &t.title);
            next.scheduled_date = Some(next_recurrence_date(&rule, t));
            next.recurrence = Some(rule);
            next.sort_order = n as i32;
            next.created_at = today_key();
            next.project_id = t.project_id;
            next.area_id = t.area_id;
            next.tag_ids = t.tag_ids.clone();
            next.priority = t.priority;
            self.todos.push(next);
        }
    }

    pub(crate) fn update_todo(&mut self, id: &str, f: impl FnOnce(&mut Todo)) {
        if let Some(t) = self.todos.iter_mut().find(|t| t.id == id) {
            f(t);
        }
    }

    pub(crate) fn move_to_today(&mut self, id: &str) {
        self.update_todo(id, |t| {
            t.status = Status::Todo;
            t.scheduled_date = Some(today_key());
            t.deadline = None;
            t.is_today = false;
            t.is_someday = false;
        });
    }

    pub(crate) fn move_to_project(&mut self, id: &str, pid: Option<&'static str>) {
        self.update_todo(id, |t| {
            t.project_id = pid;
            if task_status(t) == Status::Inbox {
                t.status = Status::Todo;
            }
            t.is_someday = false;
        });
    }

    pub(crate) fn run_menu_action(&mut self, action: MenuAction) {
        match action {
            MenuAction::TrashTodo(id) => {
                if let Some(t) = self.todos.iter_mut().find(|t| t.id == id) {
                    t.is_trashed = true;
                }
            }
            MenuAction::OpenTask(id) => self.navigate(Route::Task(id)),
            MenuAction::RestoreProject(id) => {
                if let Some(p) = self.projects.iter_mut().find(|p| p.id == id) {
                    p.status = 0;
                }
            }
            MenuAction::ArchiveProject(id) => {
                if let Some(p) = self.projects.iter_mut().find(|p| p.id == id) {
                    p.status = 2;
                }
                if matches!(&self.route, Route::Project(r) if *r == id) {
                    self.navigate(Route::Inbox);
                }
            }
            MenuAction::DeleteProject(_id) => {
                // Vue: удаление проектов заблокировано (ARK resolvability).
                self.delete_blocked = true;
            }
            MenuAction::RenameProject(id) => self.navigate(Route::Project(id)),
            MenuAction::SetStatus(id, status) => self.set_todo_status(&id, status),
            MenuAction::SetPriority(id, p) => self.update_todo(&id, |t| t.priority = p),
            MenuAction::SetProject(id, pid) => {
                let p: Option<&'static str> = pid
                    .as_deref()
                    .and_then(|p| self.projects.iter().find(|pr| pr.id == p).map(|pr| pr.id));
                self.move_to_project(&id, p);
            }
            MenuAction::AddTag(id, tag) => {
                let st = self.tags.iter().find(|t| t.id == tag).map(|t| t.id);
                if let Some(st) = st {
                    self.update_todo(&id, |t| {
                        if !t.tag_ids.contains(&st) {
                            t.tag_ids.push(st);
                        }
                    });
                }
            }
            MenuAction::SetSignificance(id, s) => self.update_todo(&id, |t| t.significance = s),
            MenuAction::SetBillable(id, b) => self.update_todo(&id, |t| t.billable = b),
            MenuAction::SetDate(id, d) => self.update_todo(&id, |t| {
                t.scheduled_date = d;
                t.deadline = None;
            }),
            MenuAction::RestoreTodo(id) => self.update_todo(&id, |t| t.is_trashed = false),
            MenuAction::MoveToToday(id) => self.move_to_today(&id),
            MenuAction::DeferTodo(id) => self.set_todo_status(&id, Status::Deferred),
            MenuAction::CompleteTodo(id) => {
                let done = self
                    .todo(&id)
                    .is_some_and(|t| task_status(t) == Status::Done);
                if done {
                    self.set_todo_status(&id, Status::Todo);
                } else {
                    self.complete_todo(&id);
                }
            }
            MenuAction::RemoveTag(id, tag) => self.update_todo(&id, |t| {
                t.tag_ids.retain(|x| *x != tag.as_str());
            }),
            MenuAction::QeSetProject(p) => self.qe_project = p,
            MenuAction::QeSetDate(d) => {
                self.qe_date = d;
                self.qe_date_touched = true;
            }
            MenuAction::RecurSetFreq(v) => self.recur_freq = v,
            MenuAction::RecurSetType(v) => self.recur_type = v,
            MenuAction::Noop => {}
        }
    }

    pub(crate) fn quick_matches(&self) -> (Vec<Todo>, Vec<Project>) {
        let q = self.quick_query.trim().to_lowercase();
        if q.is_empty() {
            return (vec![], vec![]);
        }
        let todos: Vec<Todo> = self
            .todos
            .iter()
            .filter(|t| !t.is_trashed && t.title.to_lowercase().contains(&q))
            .take(8)
            .cloned()
            .collect();
        let projects: Vec<Project> = self
            .projects
            .iter()
            .filter(|p| p.title.to_lowercase().contains(&q))
            .take(5)
            .cloned()
            .collect();
        (todos, projects)
    }
}
