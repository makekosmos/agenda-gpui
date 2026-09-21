// Root view: chrome (sidebar + titlebar), routing, pages, overlays.
use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use gpui::{
    div, prelude::*, AnyElement, Context, Entity, FocusHandle, Hsla, KeyDownEvent, ScrollHandle,
    SharedString, Subscription, WeakEntity, Window,
};
use gpui_component::input::{InputState, TextareaState};

use crate::model::*;
use crate::theme::*;

const HOVER_MS: f32 = 120.0;
mod storage;

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
    SettingsEnergy,
    Dev,
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
    pub(crate) demo: bool,
    pub(crate) storage: Option<crate::store::Worker>,
    pub(crate) storage_ready: bool,
    pub(crate) storage_busy: bool,
    pub(crate) storage_error: Option<String>,
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
    pub(crate) vsync_enabled: bool,
    pub(crate) applied_material: Option<u8>,
    pub(crate) applied_theme_sel: Option<u8>,
    /// (theme_idx, resolved dark) last pushed into gpui-component via
    /// `imago_gpui::theme::apply` — re-applies only on real changes.
    pub(crate) applied_palette: Option<(usize, bool)>,
    pub(crate) delete_blocked: bool,

    pub(crate) scrolls: HashMap<String, ScrollHandle>,
    /// Persistent uniform-list scroll handles (keep last_item_size and
    /// deferred_scroll_to_item across frames).
    pub(crate) list_scrolls: HashMap<String, gpui::UniformListScrollHandle>,
    /// Flat task rows for the current board list — rebuilt only when the
    /// list, options or `model_rev` change.
    pub(crate) row_cache: Option<crate::pages::RowCache>,
    /// Bumped on every todos/projects mutation — cheap cache-invalidation
    /// signal (zero per-frame cost vs fingerprinting the model).
    pub(crate) model_rev: u64,
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

    #[allow(dead_code)]
    pub(crate) stat_metric: u8, // 0 count 1 significance 2 fuel (selector hidden for now)
    /// Share-card overlay on the statistics page.
    pub(crate) share_open: bool,

    /// Dev page (settings): 8px design grid overlay + FPS counter + task
    /// generator state.
    pub(crate) dev_grid: bool,
    pub(crate) dev_fps: bool,
    pub(crate) dev_gen_batch: u64,
    pub(crate) fps_ema: f32,
    /// Frames counted since the FPS meter was enabled.
    pub(crate) fps_frames: u32,
    /// `AGENDA_FPS=1` also logs `fps ema` + `render_ms` to stderr every 60.
    pub(crate) fps_log: bool,
    /// Wall time of the last `render()` call — for the FPS log line.
    pub(crate) render_ms: f32,
    /// Wall time of the last visible-range row build inside the uniform_list
    /// processor — attributes scroll spikes (vs render/layout/present).
    pub(crate) rows_ms: f32,
    /// Per-frame intervals (seconds) covering the last ~60s — drives the
    /// avg/1%/0.1% stats; the graph reads only its 5s tail.
    pub(crate) fps_hist: VecDeque<f32>,
    /// Running sum of `fps_hist` — window is evicted once it exceeds 60s.
    pub(crate) fps_span: f32,
    /// Stats over the 60s window, recomputed every 60 frames.
    pub(crate) fps_avg: f32,
    pub(crate) fps_low1: f32,
    pub(crate) fps_low01: f32,
    /// Don't sample until this instant: startup/enable spikes would poison
    /// the avg/1%/0.1% window before the app is fully warmed up.
    pub(crate) fps_warmup: Option<Instant>,
    /// Frame-pipeline load: fraction of wall time the pump spent serving
    /// frame work (callbacks + draw + submit), recomputed with the stats.
    pub(crate) fps_load: f32,
    /// Accumulators for `fps_load` — GPUI-side busy nanos + wall time between
    /// stat recomputes.
    pub(crate) fps_busy_ns: u64,
    pub(crate) fps_busy_t0: Instant,
    /// The FPS meter as its own view: its per-frame `notify` dirties only this
    /// subtree, so idle frames replay the main UI's cached paint instead of
    /// re-laying out the whole window.
    pub(crate) fps_view: Entity<FpsOverlay>,
    /// Cached subtree boundaries: a notify inside one subtree (scroll in the
    /// page, hover in the sidebar, the meter ticking) repaints only that
    /// subtree — siblings replay their cached layout/paint.
    pub(crate) sidebar_view: Entity<SidebarView>,
    pub(crate) content_view: Entity<ContentView>,

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
        Some("settings-energy") => Route::SettingsEnergy,
        Some("dev") => Route::Dev,
        Some("about") => Route::About,
        Some(r) if r.starts_with("task/") => Route::Task(r[5..].to_string()),
        Some(r) if r.starts_with("project/") => Route::Project(r[8..].to_string()),
        _ => Route::Inbox,
    }
}

impl Agenda {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let demo = std::env::var("AGENDA_DEMO").as_deref() == Ok("1")
            || std::env::var("AGENDA_DEVTASKS")
                .ok()
                .and_then(|v| v.parse::<usize>().ok())
                .is_some();
        let mut this = Self {
            demo,
            storage: None,
            storage_ready: demo,
            storage_busy: false,
            storage_error: None,
            todos: if demo { seed_todos() } else { vec![] },
            projects: if demo { seed_projects() } else { vec![] },
            areas: if demo { seed_areas() } else { vec![] },
            tags: if demo { seed_tags() } else { vec![] },
            headings: if demo { seed_headings() } else { vec![] },
            events: if demo { seed_events() } else { vec![] },
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
            vsync_enabled: std::env::var("AGENDA_VSYNC").as_deref() != Ok("0"),
            applied_material: None,
            applied_theme_sel: None,
            applied_palette: None,
            delete_blocked: false,
            scrolls: HashMap::new(),
            list_scrolls: HashMap::new(),
            row_cache: None,
            model_rev: 0,
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
            share_open: false,
            dev_grid: false,
            dev_fps: std::env::var("AGENDA_FPS").is_ok(),
            dev_gen_batch: 0,
            fps_ema: 0.0,
            fps_frames: 0,
            fps_log: std::env::var("AGENDA_FPS").is_ok(),
            render_ms: 0.0,
            rows_ms: 0.0,
            fps_hist: VecDeque::new(),
            fps_span: 0.0,
            fps_avg: 0.0,
            fps_low1: 0.0,
            fps_low01: 0.0,
            fps_warmup: if std::env::var("AGENDA_FPS").is_ok() {
                Some(Instant::now() + std::time::Duration::from_secs(4))
            } else {
                None
            },
            fps_load: 0.0,
            fps_busy_ns: 0,
            fps_busy_t0: Instant::now(),
            stat_day: None,
            fps_view: {
                let agenda = cx.weak_entity();
                cx.new(|_| FpsOverlay {
                    agenda,
                    armed: false,
                    last_update: Instant::now(),
                })
            },
            sidebar_view: {
                let agenda = cx.entity();
                cx.new(|cx| SidebarView::new(agenda, cx))
            },
            content_view: {
                let agenda = cx.entity();
                cx.new(|cx| ContentView::new(agenda, cx))
            },
            root_focus: cx.focus_handle(),
            quick_focus: cx.focus_handle(),
            focused_once: false,
            qs_focused: false,
            qe_focused: false,
        };
        // AGENDA_DEVTASKS=N pre-seeds N generated tasks at startup so the
        // perf paths can be exercised hands-free (see the Dev settings page).
        let devtasks = std::env::var("AGENDA_DEVTASKS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);
        if devtasks > 0 {
            this.dev_gen_batch = 1;
            let first_sort = this.todos.iter().map(|t| t.sort_order).max().unwrap_or(0) + 1;
            this.todos
                .extend(gen_random_todos(devtasks, 0x5eed_5eed, 1, first_sort));
            this.model_rev += 1;
        }
        this.start_storage(cx);
        this
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

    /// One sampling tick for the dev FPS meter. Called by the `FpsOverlay`
    /// view's `on_next_frame` callback; drains the present-to-present deltas
    /// recorded at successful swapchain presents (RTSS-style: stats reflect
    /// real presented frames, not message-loop timing).
    pub(crate) fn sample_frame(&mut self) {
        // Warmup: keep the frame loop alive but don't record — startup spikes
        // (font/atlas/devtask seeding) would poison the stats window.
        if let Some(until) = self.fps_warmup {
            if Instant::now() < until {
                return;
            }
            self.fps_warmup = None;
            self.fps_hist.clear();
            self.fps_span = 0.0;
            self.fps_ema = 0.0;
            gpui::drain_present_deltas();
            gpui::take_frame_busy_ns();
            self.fps_busy_ns = 0;
            self.fps_busy_t0 = Instant::now();
        }
        self.fps_busy_ns += gpui::take_frame_busy_ns();
        for dt in gpui::drain_present_deltas() {
            if dt > 0.010 && self.fps_log {
                gpui::log_frame_diagnostic(format!(
                    "[slow] dt={:.1}ms render={:.1} rows={:.1}",
                    dt * 1000.0,
                    self.render_ms,
                    self.rows_ms
                ));
            }
            if dt > 0.0 {
                let f = 1.0 / dt;
                self.fps_ema = if self.fps_ema <= 0.0 {
                    f
                } else {
                    // Time-based smoothing also converges immediately at the 1 Hz idle cadence.
                    let alpha = 1.0 - (-dt / 0.1).exp();
                    self.fps_ema * (1.0 - alpha) + f * alpha
                };
                self.fps_hist.push_back(dt);
                self.fps_span += dt;
                while self.fps_span > 60.0 && self.fps_hist.len() > 1 {
                    if let Some(old) = self.fps_hist.pop_front() {
                        self.fps_span -= old;
                    }
                }
            }
        }
        self.fps_frames += 1;
        // Recompute window stats ~once a second: sort a copy of the dt window
        // (worst-case tails) — cheap relative to frame budget and off the
        // render path.
        if self.fps_busy_t0.elapsed().as_secs_f32() >= 1.0 && !self.fps_hist.is_empty() {
            let mut v: Vec<f32> = self.fps_hist.iter().copied().collect();
            v.sort_unstable_by(|a, b| a.total_cmp(b));
            let n = v.len();
            let p99 = ((n as f32 * 0.99) as usize).min(n - 1);
            let p999 = ((n as f32 * 0.999) as usize).min(n - 1);
            self.fps_avg = n as f32 / self.fps_span.max(1e-4);
            self.fps_low1 = 1.0 / v[p99].max(1e-4);
            self.fps_low01 = 1.0 / v[p999].max(1e-4);
            let wall_ns = self.fps_busy_t0.elapsed().as_nanos() as f64;
            self.fps_load = (self.fps_busy_ns as f64 / wall_ns.max(1.0)) as f32;
            self.fps_busy_ns = 0;
            self.fps_busy_t0 = Instant::now();
            if self.fps_log {
                gpui::log_frame_diagnostic(format!(
                    "[fps] avg={:.1} 1%={:.1} 0.1%={:.1} load={:.0}% render={:.1}ms",
                    self.fps_avg,
                    self.fps_low1,
                    self.fps_low01,
                    self.fps_load * 100.0,
                    self.render_ms
                ));
            }
        }
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

    /// Settings-group routes share the sidebar chrome and a single "Назад"
    /// exit point.
    pub(crate) fn is_settings_route(r: &Route) -> bool {
        matches!(
            r,
            Route::Settings
                | Route::SettingsFuel
                | Route::SettingsEnergy
                | Route::About
                | Route::Statistics
                | Route::Dev
        )
    }

    pub(crate) fn navigate(&mut self, route: Route) {
        if self.route != route {
            self.share_open = false;
            // Only crossing the settings boundary retargets "Назад" —
            // navigating between settings pages must not loop it.
            if Self::is_settings_route(&self.route) != Self::is_settings_route(&route) {
                self.back_route = self.route.clone();
            }
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
            Route::Settings => ("Отображение".into(), "icons/sliders.svg"),
            Route::SettingsFuel => ("Мыслетопливо".into(), "icons/star.svg"),
            Route::SettingsEnergy => ("Энергосбережение".into(), "icons/clock-01.svg"),
            Route::Dev => ("Для разработчиков".into(), "icons/settings.svg"),
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
        let render_t0 = Instant::now();
        window.set_frame_pacing(self.vsync_enabled);
        // Windows controls the tick source itself: full cadence during activity,
        // one genuinely rendered frame per second while idle/background.
        window.set_continuous_present(cfg!(target_os = "windows") || self.dev_fps);
        if !self.focused_once {
            self.focused_once = true;
            self.root_focus.focus(window, cx);
            let agenda = cx.weak_entity();
            window.on_window_should_close(cx, move |_, cx| {
                agenda
                    .update(cx, |this, cx| this.prepare_close(cx))
                    .unwrap_or(true)
            });
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
        // Push the imago palette into gpui-component (Inputs, tooltips,
        // scrollbars) when the selected theme or resolved mode changed.
        let palette_key = (self.theme_idx, dark);
        if self.applied_palette != Some(palette_key) {
            self.applied_palette = Some(palette_key);
            imago_gpui::theme::apply(cx);
        }
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

        // The shell renders the FPS meter beside this view. The sidebar and
        // content caches survive meter notifications. Reset when toggled off.
        if !self.dev_fps {
            self.fps_ema = 0.0;
            self.fps_frames = 0;
            self.fps_hist.clear();
            self.fps_span = 0.0;
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
            // Cached subtree boundaries: each is laid out at a definite style,
            // and its contents render lazily — only when that view (or this
            // one, via `observe`) was notified. A scroll inside the page
            // replays the sidebar; a sidebar hover replays the page.
            .child(
                self.sidebar_view.clone().cached(
                    gpui::StyleRefinement::default()
                        .w(gpui::px(crate::chrome::SIDEBAR_W * sidebar_p))
                        .h_full()
                        .flex_none()
                        .overflow_hidden(),
                ),
            )
            .child(
                self.content_view
                    .clone()
                    .cached(gpui::StyleRefinement::default().flex_1().min_w_0().h_full()),
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
        // Dev overlays: 8px design grid (minor every 8px, major every 40px)
        // and the FPS badge. Neither is interactive — no hitboxes are
        // painted, so input passes through to the UI below.
        if self.dev_grid {
            let minor = rgba(ACCENT(), 0.10);
            let major = rgba(ACCENT(), 0.22);
            overlays.push(
                gpui::canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        let w = f32::from(bounds.size.width);
                        let h = f32::from(bounds.size.height);
                        let (mut x, mut i) = (0.0f32, 0);
                        while x <= w {
                            window.paint_quad(gpui::fill(
                                gpui::Bounds::from_corners(
                                    gpui::point(gpui::px(x), gpui::px(0.)),
                                    gpui::point(gpui::px(x + 1.), gpui::px(h)),
                                ),
                                if i % 5 == 0 { major } else { minor },
                            ));
                            x += 8.0;
                            i += 1;
                        }
                        let (mut y, mut i) = (0.0f32, 0);
                        while y <= h {
                            window.paint_quad(gpui::fill(
                                gpui::Bounds::from_corners(
                                    gpui::point(gpui::px(0.), gpui::px(y)),
                                    gpui::point(gpui::px(w), gpui::px(y + 1.)),
                                ),
                                if i % 5 == 0 { major } else { minor },
                            ));
                            y += 8.0;
                            i += 1;
                        }
                    },
                )
                .absolute()
                .inset_0()
                .into_any_element(),
            );
        }
        self.render_ms = render_t0.elapsed().as_secs_f32() * 1000.0;
        if self.storage_busy {
            overlays.push(div().absolute().inset_0().occlude().into_any_element());
        }
        if self.storage_error.is_some() || self.storage_busy {
            overlays.push(self.storage_banner(cx));
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
        if ctrl && k.key == "r" && !self.demo {
            self.reload_storage();
            cx.notify();
            return;
        }
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
        self.update_todo(&tid, |t| t.title = title);
        cx.notify();
    }

    fn save_task_notes(&mut self, cx: &mut Context<Self>) {
        let Some(tid) = self.current_task_id() else {
            return;
        };
        let Some(state) = &self.notes_input else {
            return;
        };
        let notes = state.read(cx).value().trim().to_string();
        self.update_todo(&tid, |t| {
            t.notes = if notes.is_empty() { None } else { Some(notes) }
        });
        cx.notify();
    }

    pub(crate) fn current_task_id(&self) -> Option<String> {
        match &self.route {
            Route::Task(id) => Some(id.clone()),
            _ => None,
        }
    }

    /// Quick entry save: title/notes from inputs, date/project/billable/sig from state.
    fn quick_entry_save(&mut self, cx: &mut Context<Self>) {
        if !self.can_save() {
            cx.notify();
            return;
        }
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
        let mut t = new_todo(uuid::Uuid::new_v4().to_string(), &clean_title);
        t.notes = if notes.trim().is_empty() {
            None
        } else {
            Some(notes.trim().to_string())
        };
        t.scheduled_date = scheduled;
        t.project_id = project;
        t.status = status;
        t.billable = self.qe_billable;
        t.significance = self.qe_sig;
        t.sort_order = self.todos.len() as i32;
        t.created_at = chrono::Utc::now().to_rfc3339();
        if self.save_todo(None, t) {
            self.quick_entry_open = false;
        }
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

    /// Persistent `UniformListScrollHandle` per page key — wraps the page's
    /// regular `ScrollHandle` so the offset is shared, while the outer state
    /// (`last_item_size`, `deferred_scroll_to_item`) survives between frames
    /// instead of being rebuilt every render.
    pub(crate) fn list_scroll(&mut self, key: &str) -> gpui::UniformListScrollHandle {
        let base = self.scroll(key);
        self.list_scrolls
            .entry(key.to_string())
            .or_insert_with(|| {
                let h = gpui::UniformListScrollHandle::new();
                h.0.borrow_mut().base_handle = base;
                h
            })
            .clone()
    }

    /// Mutations are sent through the shared Engine persistence path.
    pub(crate) fn set_todo_status(&mut self, id: &str, status: Status) {
        if status == Status::Done {
            self.complete_todo(id);
            return;
        }
        self.update_todo(id, |t| {
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
        });
    }

    pub(crate) fn complete_todo(&mut self, id: &str) {
        let Some(before) = self.todo(id).cloned() else {
            return;
        };
        if before.is_completed {
            return;
        }
        let mut todo = before.clone();
        todo.status = Status::Done;
        todo.is_completed = true;
        todo.is_cancelled = false;
        todo.completed_at = Some(chrono::Utc::now().to_rfc3339());
        let next = todo.recurrence.as_ref().map(|rule| {
            let mut next = before.clone();
            next.id = uuid::Uuid::new_v4().to_string();
            next.status = Status::Todo;
            next.scheduled_date = Some(next_recurrence_date(rule, &todo));
            next.completed_at = None;
            next.is_completed = false;
            next.is_cancelled = false;
            next.is_today = false;
            next.is_evening = false;
            next.is_someday = false;
            next.deadline = None;
            next.reminder_date = None;
            next.checklist.clear();
            next.created_at = chrono::Utc::now().to_rfc3339();
            next.sort_order = self.todos.len() as i32;
            next
        });
        self.save_completion(before, todo, next);
    }

    pub(crate) fn update_todo(&mut self, id: &str, f: impl FnOnce(&mut Todo)) {
        let Some(before) = self.todos.iter().find(|t| t.id == id).cloned() else {
            return;
        };
        let mut todo = before.clone();
        f(&mut todo);
        self.save_todo(Some(before), todo);
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

    pub(crate) fn move_to_project(&mut self, id: &str, pid: Option<String>) {
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
            MenuAction::TrashTodo(id) => self.update_todo(&id, |t| t.is_trashed = true),
            MenuAction::OpenTask(id) => self.navigate(Route::Task(id)),
            MenuAction::RestoreProject(id) => {
                self.set_project_status(&id, 0);
            }
            MenuAction::ArchiveProject(id) => {
                self.set_project_status(&id, 2);
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
                let p: Option<String> = pid.as_deref().and_then(|p| {
                    self.projects
                        .iter()
                        .find(|pr| pr.id == p)
                        .map(|pr| pr.id.clone())
                });
                self.move_to_project(&id, p);
            }
            MenuAction::AddTag(id, tag) => {
                let st = self.tags.iter().find(|t| t.id == tag).map(|t| t.id.clone());
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
        // Covers arms that bypass update_todo (trash, project archive/restore).
        self.model_rev += 1;
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

/// Hosts the FPS meter beside Agenda. Sidebar and content have their own
/// caches; caching Agenda as well would force-refresh both when either is dirty.
pub(crate) struct AgendaShell {
    pub agenda: Entity<Agenda>,
}

impl gpui::Render for AgendaShell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let agenda = self.agenda.read(cx);
        div()
            .size_full()
            .relative()
            .child(self.agenda.clone())
            // A notified descendant invalidates every ancestor in GPUI.
            // Keep the meter outside Agenda's subtree.
            .when(agenda.dev_fps, |root| {
                root.child(
                    div()
                        .absolute()
                        .bottom_3()
                        .right_3()
                        .child(agenda.fps_view.clone()),
                )
            })
    }
}

/// Dev FPS meter as its own view (see `Agenda::fps_view`). Its per-frame
/// `on_next_frame` callback samples the interval onto `Agenda` and notifies
/// only this view, so meter-driven frames repaint a tiny subtree while the
/// main UI replays its cached paint. It is a sibling of Agenda in the shell.
pub(crate) struct FpsOverlay {
    agenda: WeakEntity<Agenda>,
    /// Whether the self-rearming sampling chain is running. It lives
    /// independently of renders so sampling continues on ticks where this
    /// view doesn't repaint.
    armed: bool,
    /// Repaint at most ~33 Hz, but update on every 1 Hz idle tick.
    last_update: Instant,
}

impl FpsOverlay {
    /// Samples present deltas every vsync tick; repaints the overlay every 5th
    /// tick. The entity survives toggling off, so explicitly stop the chain.
    fn arm(weak: WeakEntity<FpsOverlay>, agenda: WeakEntity<Agenda>, window: &mut Window) {
        window.on_next_frame(move |window, cx| {
            let alive = weak
                .update(cx, |this, cx| {
                    let enabled = agenda
                        .update(cx, |ag, _| {
                            if ag.dev_fps {
                                ag.sample_frame();
                            }
                            ag.dev_fps
                        })
                        .unwrap_or(false);
                    if !enabled {
                        this.armed = false;
                        return false;
                    }
                    if this.last_update.elapsed().as_millis() >= 30 {
                        this.last_update = Instant::now();
                        cx.notify();
                    }
                    true
                })
                .unwrap_or(false);
            if alive {
                Self::arm(weak, agenda, window);
            }
        });
    }
}

impl gpui::Render for FpsOverlay {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.armed {
            self.armed = true;
            Self::arm(cx.weak_entity(), self.agenda.clone(), window);
        }

        let (ema, avg, low1, low01, load, tail, warming) = self
            .agenda
            .upgrade()
            .map(|e| {
                let a = e.read(cx);
                // 5s tail of the 60s dt window for the graph.
                let mut tail: Vec<f32> = Vec::new();
                let mut acc = 0.0f32;
                for &dt in a.fps_hist.iter().rev() {
                    if acc > 5.0 {
                        break;
                    }
                    acc += dt;
                    tail.push(dt);
                }
                tail.reverse();
                (
                    a.fps_ema,
                    a.fps_avg,
                    a.fps_low1,
                    a.fps_low01,
                    a.fps_load,
                    tail,
                    a.fps_warmup.is_some(),
                )
            })
            .unwrap_or_default();

        let step = (tail.len() / 150).max(1);
        let off = tail.len() % step;
        let samples: Vec<f32> = tail.iter().skip(off).step_by(step).copied().collect();
        let accent = c(ACCENT());
        let warn = c(WARN());
        let guide = c(BORDER());
        div()
            .font_family("Inter")
            .text_color(c(FG()))
            .px_2()
            .py_1()
            .rounded_md()
            .border_1()
            .border_color(c(BORDER()))
            .bg(rgba(BG(), 0.85))
            .flex()
            .items_center()
            .gap_2()
            .child(
                gpui::canvas(
                    move |_, _, _| samples,
                    move |bounds, samples, window, _| {
                        let w = f32::from(bounds.size.width);
                        let h = f32::from(bounds.size.height);
                        let y_of = |dt: f32| {
                            let fps = 1.0 / dt.max(1e-4);
                            h - (fps / 180.0).clamp(0.02, 1.0) * h
                        };
                        // Reference line at 165fps.
                        let ty = y_of(1.0 / 165.0);
                        window.paint_quad(gpui::fill(
                            gpui::Bounds::from_corners(
                                gpui::point(bounds.origin.x, bounds.origin.y + gpui::px(ty)),
                                gpui::point(
                                    bounds.origin.x + gpui::px(w),
                                    bounds.origin.y + gpui::px(ty + 1.),
                                ),
                            ),
                            guide,
                        ));
                        // Polyline: each column spans prev→cur y.
                        let n = samples.len();
                        if n >= 2 {
                            let xs = w / (n - 1) as f32;
                            for i in 1..n {
                                let y0 = y_of(samples[i - 1]);
                                let y1 = y_of(samples[i]);
                                let x0 = bounds.origin.x + gpui::px((i - 1) as f32 * xs);
                                let x1 = (bounds.origin.x + gpui::px(i as f32 * xs))
                                    .max(x0 + gpui::px(1.5));
                                let top = y0.min(y1);
                                let bot = y0.max(y1).max(top + 1.5);
                                let slow = (1.0 / samples[i].max(1e-4)) < 60.0;
                                window.paint_quad(gpui::fill(
                                    gpui::Bounds::from_corners(
                                        gpui::point(x0, bounds.origin.y + gpui::px(top)),
                                        gpui::point(x1, bounds.origin.y + gpui::px(bot)),
                                    ),
                                    if slow { warn } else { accent },
                                ));
                            }
                        }
                    },
                )
                .w(gpui::px(150.))
                .h(gpui::px(26.)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(gpui::px(12.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(c(FG()))
                            .child(if warming {
                                "прогрев…".to_string()
                            } else {
                                format!("{:.0} fps", ema)
                            }),
                    )
                    .child(
                        div()
                            .text_size(gpui::px(9.))
                            .text_color(c(MUTED_FG()))
                            .child(if warming {
                                "замер начнётся после загрузки".to_string()
                            } else {
                                format!(
                                    "avg {:.0} · 1% {:.0} · 0.1% {:.0} · load {:.0}%",
                                    avg,
                                    low1,
                                    low01,
                                    load * 100.0
                                )
                            }),
                    ),
            )
    }
}

/// Sidebar as a cached subtree boundary (see `Agenda::sidebar_view`). Render
/// delegates back into `Agenda` — this entity exists only so GPUI's per-view
/// dirty tracking can replay the sidebar while the page repaints (scroll,
/// meter ticks) and vice versa. The `observe` subscription propagates
/// `Agenda` notifications down: any `cx.notify()` on Agenda also dirties
/// this view so its next render reflects current state.
pub(crate) struct SidebarView {
    agenda: Entity<Agenda>,
    _sub: Subscription,
}

impl SidebarView {
    fn new(agenda: Entity<Agenda>, cx: &mut Context<Self>) -> Self {
        let _sub = cx.observe(&agenda, |_, _, cx| cx.notify());
        Self { agenda, _sub }
    }
}

impl gpui::Render for SidebarView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.agenda.update(cx, |a, cx| {
            let p = a.sidebar_progress(window);
            a.render_sidebar(p, window, cx)
        })
    }
}

/// Titlebar + page column as a cached subtree boundary (see
/// `Agenda::content_view`) — same reasoning as `SidebarView`.
pub(crate) struct ContentView {
    agenda: Entity<Agenda>,
    _sub: Subscription,
}

impl ContentView {
    fn new(agenda: Entity<Agenda>, cx: &mut Context<Self>) -> Self {
        let _sub = cx.observe(&agenda, |_, _, cx| cx.notify());
        Self { agenda, _sub }
    }
}

impl gpui::Render for ContentView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.agenda.update(cx, |a, cx| {
            div()
                .size_full()
                .flex()
                .flex_col()
                .overflow_hidden()
                .bg(c(BG()))
                .child(a.render_titlebar(window, cx))
                .child(a.render_page(window, cx))
        })
    }
}
