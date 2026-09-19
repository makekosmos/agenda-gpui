// App chrome: sidebar (nav/projects/footer + sliding hover highlight),
// titlebar + view toggle, context menus.
use std::time::Instant;

use gpui::{
    deferred, div, prelude::*, px, ClickEvent, Context, MouseButton, MouseDownEvent, Pixels,
    SharedString, Window, WindowControlArea,
};

use crate::app::{Agenda, MenuAction, Route};
use crate::model::*;
use crate::theme::*;
use crate::widgets::*;

pub const SIDEBAR_W: f32 = 240.0;
pub const TITLEBAR_H: f32 = 40.0;

/// Left inset for the floating sidebar toggle. On macOS it must additionally
/// clear the traffic-light buttons (drawn by the OS inside the window).
#[cfg(target_os = "macos")]
const TOGGLE_LEFT: f32 = 84.0;
#[cfg(not(target_os = "macos"))]
const TOGGLE_LEFT: f32 = 12.0;
/// Window-drag areas start here so the floating toggle stays clickable.
const DRAG_INSET: f32 = TOGGLE_LEFT + 40.0;
/// Native caption button width (Windows convention).
pub(crate) const CAPTION_W: f32 = 46.0;

struct SbRect {
    y: f32,
    h: f32,
}

/// Sidebar row spec shared by nav buttons and project links.
struct SbItem {
    id: String,
    label: String,
    /// `Some` renders an SVG icon; `None` renders the 8px muted project dot.
    icon: Option<&'static str>,
    active: bool,
    /// Bottom-rounded row (last project in the accordion list).
    last: bool,
    route: Route,
    /// `Some(pid)` attaches the rename/archive right-click context menu.
    ctx_project: Option<String>,
}

impl SbItem {
    fn nav(id: &str, icon: &'static str, label: &str, active: bool, route: Route) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            icon: Some(icon),
            active,
            last: false,
            route,
            ctx_project: None,
        }
    }

    fn project(project: &Project, active: bool, last: bool) -> Self {
        let pid = project.id.to_string();
        Self {
            id: format!("proj-{pid}"),
            label: project.title.clone(),
            icon: None,
            active,
            last,
            route: Route::Project(pid.clone()),
            ctx_project: Some(pid),
        }
    }
}

impl Agenda {
    /// One sidebar row (`.kosmos-sidebar-btn` / `.kosmos-sidebar-project-link`):
    /// h32, pl10 pr8, r8, gap6, fs13 lh1.15, children opacity .6 (→1 on
    /// hover/active). Hover bg comes from the sliding highlight, not per-item
    /// bg.
    fn sb_item(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        spec: SbItem,
    ) -> gpui::Stateful<gpui::Div> {
        let t = self.hover_t(window, &format!("nav-{}", spec.id));
        let glyph_alpha = if spec.active { 1.0 } else { 0.6 + 0.4 * t };
        let glyph = rgba(FG(), glyph_alpha);
        let weak = cx.weak_entity();
        let key = SharedString::from(format!("nav-{}", spec.id));
        let mut el = div()
            .id(SharedString::from(format!("sb-{}", spec.id)))
            .min_h(px(32.))
            .w_full()
            .flex()
            .items_center()
            .gap_1p5()
            .pl(px(10.))
            .pr_2()
            .text_size(px(13.))
            .line_height(px(15.));
        el = match spec.icon {
            Some(path) => el.child(icon(path, 16., glyph)),
            None => el.child(
                div()
                    .w_2()
                    .h_2()
                    .rounded_full()
                    .flex_none()
                    .bg(rgba(MUTED_FG(), glyph_alpha)),
            ),
        };
        let mut el = el.child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .text_color(glyph)
                .child(spec.label),
        );
        if spec.last {
            el = el.rounded_b_lg();
        } else if spec.icon.is_some() {
            el = el.rounded_lg();
        }
        if spec.active {
            el = el.bg(rgba(FG(), 0.12));
        } else {
            // Paint-time hover truth (like CSS :hover in zeron): the bg is
            // resolved from the current hitbox each frame, so it can never
            // get stuck if a hover event is lost.
            el = el.hover(|s| s.bg(rgba(FG(), 0.08)));
        }
        let mut el = el
            .on_hover(move |hovered, _, cx| {
                let _ = weak.update(cx, |this, _| {
                    this.set_hover(&key, *hovered);
                    this.sb_hover(*hovered, &key);
                });
            })
            .on_click({
                let weak = cx.weak_entity();
                let route = spec.route.clone();
                move |_: &ClickEvent, _, cx| {
                    let _ = weak.update(cx, |this, _| this.navigate(route.clone()));
                }
            });
        if let Some(pid) = spec.ctx_project {
            el = el.on_mouse_down(MouseButton::Right, {
                let weak = cx.weak_entity();
                move |ev: &MouseDownEvent, _, cx| {
                    let pos = ev.position;
                    let pid = pid.clone();
                    let _ = weak.update(cx, |this, _| {
                        this.menu = Some(crate::app::CtxMenu {
                            x: pos.x.into(),
                            y: pos.y.into(),
                            items: vec![
                                (
                                    "Переименовать".into(),
                                    false,
                                    MenuAction::RenameProject(pid.clone()),
                                ),
                                (
                                    "Архивировать".into(),
                                    false,
                                    MenuAction::ArchiveProject(pid.clone()),
                                ),
                            ],
                        });
                    });
                }
            });
        }
        el
    }

    fn sb_hover(&mut self, hovered: bool, id: &str) {
        if hovered {
            self.sb_hover_id = Some(SharedString::from(id.to_string()));
        } else if self.sb_hover_id.as_ref().is_some_and(|s| s.as_str() == id) {
            self.sb_hover_id = None;
        }
    }

    /// Advance the sliding highlight: y/hh interpolate from the retarget
    /// anchors (y0/h0) over a fixed 160ms cubic-bezier(0.23,1,0.32,1) window;
    /// opacity fades at 80ms. `stamp` is anchored on retarget only — the
    /// slide progress is cumulative, so a hidden/occluded stretch simply
    /// completes the move instead of restarting or crawling.
    fn highlight_tick(&mut self, window: &mut Window) {
        let h = &mut self.sb_highlight;
        let now = Instant::now();
        let progress = (now.duration_since(h.stamp).as_secs_f32() * 1000.0 / 160.0).min(1.0);
        let f = ease_out_quint(progress);
        h.y = h.y0 + (h.ty - h.y0) * f;
        h.hh = h.h0 + (h.th - h.h0) * f;
        let frame_dt = now.duration_since(h.tick_stamp).as_secs_f32() * 1000.0;
        h.tick_stamp = now;
        let ostep = frame_dt / 80.0;
        if h.visible {
            h.opacity = (h.opacity + ostep).min(1.0);
        } else {
            h.opacity = (h.opacity - ostep).max(0.0);
        }
        let animating = (h.y - h.ty).abs() > 0.2
            || (h.hh - h.th).abs() > 0.2
            || (h.opacity > 0.0 && h.opacity < 1.0)
            || (h.opacity > 0.0 && !h.visible);
        if animating {
            window.request_animation_frame();
        }
    }

    /// Advance the projects-folder icon swap (350ms emphasized); returns
    /// eased 0..1 (0 = closed, 1 = open).
    fn group_open_tick(&mut self, window: &mut Window) -> f32 {
        let now = Instant::now();
        let dt = now.duration_since(self.group_open_stamp).as_secs_f32() * 1000.0;
        self.group_open_stamp = now;
        let target = if self.kanban_group_open { 1.0 } else { 0.0 };
        let step = dt / 350.0;
        if self.group_open_t < target {
            self.group_open_t = (self.group_open_t + step).min(target);
        } else if self.group_open_t > target {
            self.group_open_t = (self.group_open_t - step).max(target);
        }
        if (self.group_open_t - target).abs() > 0.001 {
            window.request_animation_frame();
        }
        ease_emphasized(self.group_open_t)
    }

    pub(crate) fn render_sidebar(
        &mut self,
        p: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let settings = Self::is_settings_route(&self.route);
        let w = SIDEBAR_W * p;
        let mut rects: Vec<(SharedString, SbRect)> = vec![];
        let mut y = 44.0f32; // pt40 shell + pt4 body

        let mut navs: Vec<gpui::Stateful<gpui::Div>> = vec![];
        if settings {
            navs.push(self.sb_item(
                window,
                cx,
                SbItem::nav(
                    "back",
                    "icons/arrow-left.svg",
                    "Назад",
                    false,
                    self.back_route.clone(),
                ),
            ));
            rects.push(("nav-back".into(), SbRect { y, h: 32. }));
            y += 33.;
            navs.push(self.sb_item(
                window,
                cx,
                SbItem::nav(
                    "appearance",
                    "icons/sliders.svg",
                    "Отображение",
                    self.route == Route::Settings,
                    Route::Settings,
                ),
            ));
            rects.push(("nav-appearance".into(), SbRect { y, h: 32. }));
            y += 33.;
            navs.push(self.sb_item(
                window,
                cx,
                SbItem::nav(
                    "about",
                    "icons/help-circle.svg",
                    "О приложении",
                    self.route == Route::About,
                    Route::About,
                ),
            ));
            rects.push(("nav-about".into(), SbRect { y, h: 32. }));
        } else {
            let items: [(&str, &'static str, &str, Route); 7] = [
                ("inbox", "icons/inbox.svg", "Входящие", Route::Inbox),
                ("today", "icons/calendar-01.svg", "Сегодня", Route::Today),
                ("plans", "icons/calendar-02.svg", "Планы", Route::Plans),
                (
                    "calendar",
                    "icons/calendar-02.svg",
                    "Календарь",
                    Route::Calendar,
                ),
                ("someday", "icons/clock-01.svg", "Потом", Route::Someday),
                (
                    "statistics",
                    "icons/analytics-01.svg",
                    "Статистика",
                    Route::Statistics,
                ),
                (
                    "recurring",
                    "icons/repeat.svg",
                    "Повторяющиеся",
                    Route::Recurring,
                ),
            ];
            for (id, ic, label, route) in items {
                let active = self.route == route;
                navs.push(self.sb_item(window, cx, SbItem::nav(id, ic, label, active, route)));
                rects.push((format!("nav-{id}").into(), SbRect { y, h: 32. }));
                y += 33.;
            }
        }

        // Projects section (.sidebar__part-10 mt-16 → section)
        let mut projects_el: Option<gpui::Div> = None;
        if !settings {
            y += 16.;
            let header_y = y;
            let active_projects: Vec<Project> = self
                .projects
                .iter()
                .filter(|pr| pr.status == 0)
                .cloned()
                .collect();
            let t_head = self.hover_t(window, "nav-projects-head");
            let head_alpha = 0.6 + 0.4 * t_head;

            // Folder icon swap: closed collapses upward, open unfolds from top —
            // 350ms cubic-bezier(0.22,1,0.36,1) (agenda ::before/::after masks).
            let gt = self.group_open_tick(window);
            let folder_icons = div()
                .w(px(16.))
                .h(px(16.))
                .flex_none()
                .relative()
                .child(div().absolute().top(px(-4.0 * gt)).left_0().child(icon(
                    "icons/folder-closed.svg",
                    16.,
                    rgba(FG(), head_alpha * (1.0 - gt)),
                )))
                .child(
                    div()
                        .absolute()
                        .top(px(-4.0 * (1.0 - gt)))
                        .left_0()
                        .child(icon(
                            "icons/folder-open2.svg",
                            16.,
                            rgba(FG(), head_alpha * gt),
                        )),
                );

            let weak = cx.weak_entity();
            let header = div()
                .id("sb-projects-head")
                .h(px(32.))
                .flex()
                .items_center()
                .pl(px(10.))
                .when(gt > 0.001, |el| el.rounded_t_lg())
                .when(gt <= 0.001, |el| el.rounded_lg())
                .bg(mix(FG(), 0.05 + 0.03 * t_head, BG()))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .flex()
                        .items_center()
                        .gap_1p5()
                        .text_size(px(12.))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(rgba(FG(), head_alpha))
                        .child(folder_icons)
                        .child("Проекты"),
                )
                .child(
                    div()
                        .w(px(32.))
                        .h_full()
                        .grid()
                        .items_center()
                        .justify_center()
                        .child(icon("icons/plus.svg", 16., rgba(FG(), head_alpha))),
                )
                .on_hover({
                    let weak = weak.clone();
                    move |hovered, _, cx| {
                        let _ = weak.update(cx, |this, _| {
                            this.set_hover("nav-projects-head", *hovered);
                            if *hovered {
                                this.sb_hover_id = None;
                            }
                        });
                    }
                })
                .on_click({
                    let weak = weak.clone();
                    move |_: &ClickEvent, _, cx| {
                        let _ = weak.update(cx, |this, _| {
                            this.kanban_group_open = !this.kanban_group_open;
                            this.group_open_stamp = Instant::now();
                            // The header itself stays mounted and hovered —
                            // keep its hover target so its bg/icons don't dim.
                            this.reset_hovers(Some("nav-projects-head"));
                        });
                    }
                });
            rects.push((
                "nav-projects-head".into(),
                SbRect {
                    y: header_y,
                    h: 32.,
                },
            ));
            y += 32.;

            // Accordion: the list height follows the same 350ms `gt` ramp the
            // folder icons use; rows stay mounted while 0 < gt < 1 and are
            // clipped by the shrinking container.
            let mut links: Vec<gpui::Stateful<gpui::Div>> = vec![];
            let count = active_projects.len();
            if gt > 0.001 {
                for (i, pr) in active_projects.iter().enumerate() {
                    let active = matches!(&self.route, Route::Project(r) if r == pr.id);
                    let spec = SbItem::project(pr, active, i == count - 1);
                    rects.push((format!("nav-{}", spec.id).into(), SbRect { y, h: 32. * gt }));
                    links.push(self.sb_item(window, cx, spec));
                    y += 32. * gt;
                }
            }
            let list = div()
                .flex()
                .flex_col()
                .bg(fg_mix(0.03))
                .rounded_b_lg()
                .h(px(count as f32 * 32. * gt))
                .overflow_hidden()
                .children(links);
            projects_el = Some(div().mt_4().flex().flex_col().child(header).child(list));
        }

        // Footer (.sidebar__part-20): "Другое" flex-1.
        let mut footer: Option<gpui::Stateful<gpui::Div>> = None;
        if !settings {
            let t = self.hover_t(window, "nav-more");
            // Visually "open" until the accordion finishes its 150ms
            // fade-out — the header styling and the sliding highlight must
            // not snap off while the menu is still animating away.
            let menu_live = self.more_open || self.more_menu_t > 0.001;
            let glyph_alpha = if menu_live { 1.0 } else { 0.6 + 0.4 * t };
            let glyph = rgba(FG(), glyph_alpha);
            let weak = cx.weak_entity();
            let mut el = div()
                .id("sb-more")
                .h(px(32.))
                .w_full()
                .flex_1()
                .flex()
                .items_center()
                .gap_1p5()
                .pl(px(10.))
                .pr_2()
                .text_size(px(13.))
                .line_height(px(15.))
                .child(icon("icons/more-circle.svg", 16., glyph))
                .child(div().text_color(glyph).child("Другое"));
            if menu_live {
                // Mirrors the Projects header: same fg5%(+3% hover) mix,
                // rounded at the outer edge (bottom for a group below the list).
                el = el.bg(mix(FG(), 0.05 + 0.03 * t, BG())).rounded_b_lg();
            } else {
                el = el.rounded_lg().hover(|s| s.bg(rgba(FG(), 0.08)));
            }
            el = el
                .on_hover({
                    let weak = weak.clone();
                    move |hovered, _, cx| {
                        let _ = weak.update(cx, |this, _| {
                            this.set_hover("nav-more", *hovered);
                            this.sb_hover(*hovered, "nav-more");
                        });
                    }
                })
                .on_mouse_down(MouseButton::Left, {
                    let weak = weak.clone();
                    move |_: &MouseDownEvent, _, cx| {
                        let _ = weak.update(cx, |this, _| {
                            this.more_down_was_open = this.more_open;
                        });
                    }
                })
                .on_click(move |_: &ClickEvent, _, cx| {
                    let _ = weak.update(cx, |this, _| {
                        // Open-invert of the press-time state: when the menu
                        // was open, the backdrop already closed it on
                        // mouse_down and this click must not reopen it.
                        this.more_open = !this.more_down_was_open;
                        this.more_menu_stamp = Instant::now();
                    });
                });
            footer = Some(el);
        }

        // Resolve highlight target now that rects are known.
        let sidebar_h = window.viewport_size().height;
        if let Some(footer_el) = &footer {
            let _ = footer_el;
            let fy: f32 = sidebar_h.into();
            // Footer button sits above the 16px bottom padding: the 32px row
            // occupies [fy-48, fy-16).
            rects.push((
                "nav-more".into(),
                SbRect {
                    y: fy - 48.,
                    h: 32.,
                },
            ));
        }
        // While the "Другое" accordion is live (open or fading out) the
        // highlight stays pinned to its button — it must only fade after the
        // accordion has fully hidden.
        let highlight_id = if self.more_open || self.more_menu_t > 0.001 {
            Some(SharedString::from("nav-more"))
        } else {
            self.sb_hover_id.clone()
        };
        if let Some(id) = highlight_id {
            if let Some((_, r)) = rects.iter().find(|(rid, _)| *rid == id) {
                let h = &mut self.sb_highlight;
                if h.ty != r.y || h.th != r.h {
                    // Retarget: anchor the slide origins and restart the
                    // fixed 160ms progress window.
                    h.y0 = h.y;
                    h.h0 = h.hh;
                    h.ty = r.y;
                    h.th = r.h;
                    h.stamp = Instant::now();
                }
                if h.opacity == 0.0 {
                    // Appear directly at target (no travel on first hover).
                    h.y = r.y;
                    h.hh = r.h;
                    h.y0 = r.y;
                    h.h0 = r.h;
                }
                h.visible = true;
            } else {
                self.sb_highlight.visible = false;
            }
        } else {
            self.sb_highlight.visible = false;
        }
        self.highlight_tick(window);

        let body = div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .gap_px()
            .pt_1()
            .overflow_y_hidden()
            .children(navs)
            .children(projects_el);

        let mut shell = div()
            .size_full()
            .flex()
            .flex_col()
            .justify_between()
            .bg(self.sidebar_surface())
            .px_2()
            .pt(px(40.))
            .pb_4()
            .relative()
            .child(
                // sliding hover highlight
                div()
                    .absolute()
                    .left_2()
                    .right_2()
                    .top(px(self.sb_highlight.y))
                    .h(px(self.sb_highlight.hh))
                    .rounded_lg()
                    .bg(rgba(FG(), 0.06 * self.sb_highlight.opacity)),
            )
            .child(
                // top strip doubles as a window drag area; the left inset
                // keeps the hitbox clear of the macOS traffic lights
                div()
                    .id("sb-top-drag")
                    .absolute()
                    .top_0()
                    .left(px(DRAG_INSET))
                    .right_0()
                    .h(px(TITLEBAR_H))
                    .window_control_area(WindowControlArea::Drag),
            )
            .child(body);
        if let Some(f) = footer {
            shell = shell.child(div().mt_3().flex().child(f));
        }

        div()
            .w(px(w))
            .h_full()
            .flex_none()
            .overflow_hidden()
            .relative()
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .bottom_0()
                    .w(px(SIDEBAR_W))
                    .h_full()
                    .border_r_1()
                    .border_color(rgba(SIDEBAR_DIVIDER(), if p > 0.5 { 1.0 } else { 0.0 }))
                    .child(shell),
            )
    }

    /// Custom titlebar: drag region (HTCAPTION → native move/snap/dbl-click
    /// maximize), the display-options button at the right edge, then platform
    /// caption buttons. The left spacer keeps the drag hitbox clear of the
    /// floating sidebar toggle and macOS traffic lights.
    pub(crate) fn render_titlebar(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = ease_emphasized(self.sidebar_t.clamp(0.0, 1.0));
        let pl = DRAG_INSET + (16.0 - DRAG_INSET) * p;
        let (title, icon_path) = self.page_title();
        #[allow(unused_mut)]
        let mut drag = div()
            .id("titlebar-drag")
            .flex_1()
            .h_full()
            .min_w_0()
            .flex()
            .items_center()
            .gap_1p5()
            .window_control_area(WindowControlArea::Drag)
            .child(icon(icon_path, 18., rgba(FG(), 0.82)))
            .child(
                div()
                    .text_size(px(13.))
                    .line_height(px(15.))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(c(FG()))
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(title),
            );
        #[cfg(target_os = "macos")]
        {
            drag = drag.on_double_click(|_, window, _| window.titlebar_double_click());
        }
        #[cfg(target_os = "linux")]
        {
            drag = drag.on_double_click(|_, window, _| window.zoom_window());
        }

        #[allow(unused_mut)]
        let mut bar = div()
            .h(px(TITLEBAR_H))
            .flex_none()
            .w_full()
            .flex()
            .child(div().w(px(pl)).h_full().flex_none())
            .child(drag)
            .child(
                // display-options button hugs the right edge, just before
                // the native caption buttons
                div()
                    .h_full()
                    .flex_none()
                    .flex()
                    .items_center()
                    .pr_2()
                    .children(self.render_options_button(window, cx)),
            );
        #[cfg(not(target_os = "macos"))]
        {
            bar = bar.child(self.render_window_controls(window, cx));
        }
        bar.into_any_element()
    }

    /// Native window caption buttons (min / max|restore / close).
    /// Windows: hitboxes are `WindowControlArea`s — the platform handles press,
    /// snap flyout and system tooltips; we only draw the glyphs and hover bg.
    /// Linux: plain click handlers on the same visuals. macOS uses the OS
    /// traffic lights instead.
    #[cfg(not(target_os = "macos"))]
    fn render_window_controls(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        const CLOSE_RED: u32 = 0xe81123;
        let maximized = window.is_maximized();
        let specs: [(&str, &str, WindowControlArea, bool); 3] = [
            ("min", "icons/window-min.svg", WindowControlArea::Min, false),
            (
                "max",
                if maximized {
                    "icons/window-restore.svg"
                } else {
                    "icons/window-max.svg"
                },
                WindowControlArea::Max,
                false,
            ),
            (
                "close",
                "icons/status-x.svg",
                WindowControlArea::Close,
                true,
            ),
        ];
        let mut row = div().flex().flex_none().h_full();
        for (id, path, area, danger) in specs {
            let key = SharedString::from(format!("win-{id}"));
            let t = self.hover_t(window, key.as_ref());
            let glyph = if danger {
                lerp(FG(), 0xffffff, t)
            } else {
                rgba(FG(), 0.82)
            };
            let weak = cx.weak_entity();
            let mut btn = div()
                .id(SharedString::from(format!("win-{id}")))
                .w(px(CAPTION_W))
                .h_full()
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .hover(move |s| {
                    s.bg(if danger {
                        rgba(CLOSE_RED, 1.0)
                    } else {
                        rgba(FG(), 0.10)
                    })
                })
                .child(icon(path, 14., glyph))
                .on_hover(move |hovered, _, cx| {
                    let _ = weak.update(cx, |this, _| this.set_hover(&key, *hovered));
                });
            #[cfg(target_os = "windows")]
            {
                btn = btn.window_control_area(area);
            }
            #[cfg(target_os = "linux")]
            {
                btn = btn.on_click(move |_: &ClickEvent, window, _| match area {
                    WindowControlArea::Min => window.minimize_window(),
                    WindowControlArea::Max => window.zoom_window(),
                    WindowControlArea::Close => window.remove_window(),
                    _ => {}
                });
            }
            row = row.child(btn);
        }
        row
    }

    /// Sidebar toggle button: floating overlay at the top-left, inside the
    /// titlebar strip. Drag hitboxes start at DRAG_INSET so it stays
    /// clickable; on macOS TOGGLE_LEFT clears the traffic lights.
    pub(crate) fn render_sidebar_toggle(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let t = self.hover_t(window, "sb-toggle");
        let weak = cx.weak_entity();
        div()
            .id("sb-toggle")
            .absolute()
            .top_0()
            .left(px(TOGGLE_LEFT))
            .h(px(TITLEBAR_H))
            .flex()
            .items_center()
            .child(
                div()
                    .id("sb-toggle-btn")
                    .w_7()
                    .h_7()
                    .grid()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .hover(|s| s.bg(rgba(FG(), 0.08)))
                    .child(icon(
                        "icons/sidebar-left.svg",
                        18.,
                        rgba(FG(), 0.6 + 0.4 * t),
                    ))
                    .on_hover({
                        let weak = weak.clone();
                        move |hovered, _, cx| {
                            let _ = weak.update(cx, |this, _| {
                                this.set_hover("sb-toggle", *hovered);
                            });
                        }
                    })
                    .on_click(move |_: &ClickEvent, _, cx| {
                        let _ = weak.update(cx, |this, _| {
                            this.sidebar_target = if this.sidebar_target > 0.5 { 0.0 } else { 1.0 };
                            this.sidebar_stamp = Instant::now();
                        });
                    }),
            )
    }

    /// Display-options button (view / sort / group popover) pinned to the
    /// right end of the titlebar, just left of the native caption controls.
    /// `None` on routes without display options.
    pub(crate) fn render_options_button(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<gpui::Stateful<gpui::Div>> {
        let storage = self.view_options_key()?;
        let open = self.options_for.is_some();
        let t = self.hover_t(window, "opt-btn");
        let weak = cx.weak_entity();
        let key = SharedString::from(storage);
        Some(
            div()
                .id("opt-btn")
                .w_7()
                .h_7()
                .grid()
                .items_center()
                .justify_center()
                .rounded_md()
                .hover(|s| s.bg(rgba(FG(), 0.08)))
                .when(open, |el| el.bg(rgba(FG(), 0.10)))
                .child(icon(
                    "icons/preference-vertical.svg",
                    18.,
                    rgba(FG(), 0.6 + 0.4 * t),
                ))
                .on_hover({
                    let weak = weak.clone();
                    move |hovered, _, cx| {
                        let _ = weak.update(cx, |this, _| {
                            this.set_hover("opt-btn", *hovered);
                        });
                    }
                })
                .on_click(move |_: &ClickEvent, _, cx| {
                    let key = key.clone();
                    let _ = weak.update(cx, |this, _| {
                        this.options_for = if this.options_for.is_some() {
                            None
                        } else {
                            Some(key.to_string())
                        };
                    });
                }),
        )
    }

    /// "Другое" popover: opens upward flush with the footer button, width =
    /// sidebar inner width, fg3% bg, radius top lg — a mirrored Projects
    /// group. Same 350ms height accordion (cubic-bezier(0.22,1,0.36,1)):
    /// the clipped box grows from the button and top rows reveal last.
    pub(crate) fn render_more_menu(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let now = Instant::now();
        let dt = now.duration_since(self.more_menu_stamp).as_secs_f32() * 1000.0;
        self.more_menu_stamp = now;
        let target = if self.more_open { 1.0 } else { 0.0 };
        let step = dt / 350.0;
        if self.more_menu_t < target {
            self.more_menu_t = (self.more_menu_t + step).min(target);
        } else if self.more_menu_t > target {
            self.more_menu_t = (self.more_menu_t - step).max(target);
        }
        if (self.more_menu_t - target).abs() > 0.001 {
            window.request_animation_frame();
        }
        if !self.more_open && self.more_menu_t <= 0.001 {
            // Fully closed: render nothing so the backdrop hitbox is removed
            // (otherwise an invisible overlay keeps swallowing every click).
            return div().into_any_element();
        }
        let pop = ease_emphasized(self.more_menu_t);

        let items: [(&str, &'static str, &str, Route); 4] = [
            ("logbook", "icons/book-open.svg", "Архив", Route::Logbook),
            ("trash", "icons/delete.svg", "Корзина", Route::Trash),
            (
                "settings",
                "icons/settings.svg",
                "Настройки",
                Route::Settings,
            ),
            (
                "about",
                "icons/help-circle.svg",
                "О приложении",
                Route::About,
            ),
        ];
        let menu_h = items.len() as f32 * 32.;
        let mut rows: Vec<gpui::Stateful<gpui::Div>> = vec![];
        for (id, ic, label, route) in items {
            let t = self.hover_t(window, &format!("more-{id}"));
            let active = self.route == route;
            let glyph_alpha = if active { 1.0 } else { 0.6 + 0.4 * t };
            let glyph = rgba(FG(), glyph_alpha);
            let weak = cx.weak_entity();
            let key = SharedString::from(format!("more-{id}"));
            let mut el = div()
                .id(SharedString::from(format!("more-item-{id}")))
                .min_h(px(32.))
                .w_full()
                .flex()
                .items_center()
                .gap_1p5()
                .pl(px(10.))
                .pr_2()
                .text_size(px(13.))
                .line_height(px(15.))
                .child(icon(ic, 16., glyph))
                .child(div().text_color(glyph).child(label));
            if active {
                el = el.bg(rgba(FG(), 0.12));
            } else {
                el = el.hover(|s| s.bg(rgba(FG(), 0.08)));
            }
            el = el
                .on_hover({
                    let weak = weak.clone();
                    move |hovered, _, cx| {
                        let _ = weak.update(cx, |this, _| this.set_hover(&key, *hovered));
                    }
                })
                .on_click(move |_: &ClickEvent, _, cx| {
                    let _ = weak.update(cx, |this, _| {
                        this.more_open = false;
                        this.navigate(route.clone());
                    });
                });
            rows.push(el);
        }

        // 48px = 32px footer button + 16px bottom padding. Height-animated
        // clip box pinned flush to the button — same accordion mechanics as
        // the Projects list, mirrored: justify_end keeps rows glued to the
        // button so the nearest row reveals first as the box grows upward.
        let menu = div()
            .absolute()
            .left_2()
            .bottom(px(48.))
            .w(px(SIDEBAR_W - 16.))
            .h(px(menu_h * pop))
            .overflow_hidden()
            .flex()
            .flex_col()
            .justify_end()
            .child(
                div()
                    .h(px(menu_h))
                    .flex_none()
                    .w_full()
                    .flex()
                    .flex_col()
                    .rounded_t_lg()
                    .bg(fg_mix(0.03))
                    .children(rows),
            );

        // Click-away backdrop. Presses that land on the footer button itself
        // (the bottom 48px of the sidebar column) are ignored so the event
        // propagates to the button, which toggles the menu closed.
        let weak = cx.weak_entity();
        div()
            .absolute()
            .inset_0()
            .child(div().id("more-backdrop").size_full().on_mouse_down(
                MouseButton::Left,
                move |ev: &MouseDownEvent, window, cx| {
                    let sidebar_bottom = f32::from(window.viewport_size().height) - 48.;
                    let on_button =
                        ev.position.x < px(SIDEBAR_W) && ev.position.y > px(sidebar_bottom);
                    let _ = weak.update(cx, |this, _| {
                        if on_button {
                            return;
                        }
                        if this.more_open {
                            this.more_menu_stamp = Instant::now();
                        }
                        this.more_open = false;
                    });
                },
            ))
            .child(deferred(menu))
            .into_any_element()
    }

    /// Context menu panel (imago ContextMenu): min-w ~160, p4, r8, border,
    /// shadow-lg. Items h28 fs13, destructive in red.
    pub(crate) fn render_ctx_menu(
        &mut self,
        menu: &crate::app::CtxMenu,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let weak = cx.weak_entity();
        let mut rows: Vec<gpui::Stateful<gpui::Div>> = vec![];
        for (i, (label, destructive, action)) in menu.items.iter().enumerate() {
            let action = action.clone();
            let color = if *destructive {
                c(DESTRUCTIVE())
            } else {
                c(FG())
            };
            rows.push(
                div()
                    .id(SharedString::from(format!("ctx-item-{i}")))
                    .h_7()
                    .w_full()
                    .flex()
                    .items_center()
                    .px_2()
                    .rounded(px(5.))
                    .text_size(px(13.))
                    .text_color(color)
                    .hover(|s| s.bg(fg_mix(0.06)))
                    .child(label.clone())
                    .on_click({
                        let weak = weak.clone();
                        move |_: &ClickEvent, _, cx| {
                            let _ = weak.update(cx, |this, _| {
                                this.menu = None;
                                this.run_menu_action(action.clone());
                            });
                        }
                    }),
            );
        }

        let x = Pixels::from(menu.x);
        let y = Pixels::from(menu.y);
        let panel = div()
            .absolute()
            .left(x)
            .top(y)
            .min_w(px(160.))
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
                    .id("ctx-backdrop")
                    .size_full()
                    .on_mouse_down(MouseButton::Left, {
                        let weak = weak.clone();
                        move |_: &MouseDownEvent, _, cx| {
                            let _ = weak.update(cx, |this, _| this.menu = None);
                        }
                    })
                    .on_mouse_down(MouseButton::Right, move |_: &MouseDownEvent, _, cx| {
                        let _ = weak.update(cx, |this, _| this.menu = None);
                    }),
            )
            .child(deferred(panel))
            .into_any_element()
    }
}
