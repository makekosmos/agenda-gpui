// App chrome: sidebar (nav/projects/footer + sliding hover highlight),
// titlebar, floating toggle, context menus.
use std::time::Instant;

use gpui::{
    deferred, div, prelude::*, px, ClickEvent, Context, MouseButton, MouseDownEvent, Pixels,
    SharedString, Window,
};

use crate::app::{Agenda, MenuAction, Route};
use crate::model::*;
use crate::theme::*;
use crate::widgets::*;

pub const SIDEBAR_W: f32 = 240.0;
pub const TITLEBAR_H: f32 = 40.0;

struct SbRect {
    y: f32,
    h: f32,
}

impl Agenda {
    /// One sidebar row (`.kosmos-sidebar-btn`): h32, pl10 pr8, r8, gap6,
    /// fs13 lh1.15, children opacity .6 (→1 on hover/active).
    /// Hover bg comes from the sliding highlight, not per-item bg.
    fn nav_btn(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        id: &str,
        icon_path: &'static str,
        label: &str,
        active: bool,
        route: Route,
    ) -> gpui::Stateful<gpui::Div> {
        let t = self.hover_t(window, &format!("nav-{id}"));
        let glyph_alpha = if active { 1.0 } else { 0.6 + 0.4 * t };
        let glyph = rgba(FG, glyph_alpha);
        let weak = cx.weak_entity();
        let key = SharedString::from(format!("nav-{id}"));
        let mut el = div()
            .id(SharedString::from(format!("sb-{id}")))
            .h(px(32.))
            .w_full()
            .flex()
            .items_center()
            .gap_1p5()
            .pl(px(10.))
            .pr_2()
            .rounded_lg()
            .text_size(px(13.))
            .line_height(px(15.))
            .child(icon(icon_path, 16., glyph))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_color(glyph)
                    .child(label.to_string()),
            );
        if active {
            el = el.bg(fg_mix(0.12));
        }
        el.on_hover({
            let key2 = key.clone();
            let weak2 = weak.clone();
            move |hovered, _, cx| {
                let _ = weak2.update(cx, |this, _| {
                    this.set_hover(&key2, *hovered);
                    this.sb_hover(*hovered, &key2);
                });
            }
        })
        .on_click(move |_: &ClickEvent, _, cx| {
            let _ = weak.update(cx, |this, _| this.navigate(route.clone()));
        })
    }

    /// Sidebar project link (`.kosmos-sidebar-project-link`): same metrics,
    /// 8px muted dot instead of an icon.
    fn project_link(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        id: &str,
        label: &str,
        active: bool,
        last: bool,
        project_id: &str,
    ) -> gpui::Stateful<gpui::Div> {
        let t = self.hover_t(window, &format!("nav-{id}"));
        let glyph_alpha = if active { 1.0 } else { 0.6 + 0.4 * t };
        let glyph = rgba(FG, glyph_alpha);
        let weak = cx.weak_entity();
        let pid = project_id.to_string();
        let key = SharedString::from(format!("nav-{id}"));
        let mut el = div()
            .id(SharedString::from(format!("sb-{id}")))
            .min_h(px(32.))
            .w_full()
            .flex()
            .items_center()
            .gap_1p5()
            .pl(px(10.))
            .pr_2()
            .child(
                div()
                    .w_2()
                    .h_2()
                    .rounded_full()
                    .flex_none()
                    .bg(rgba(MUTED_FG, glyph_alpha)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_size(px(13.))
                    .line_height(px(15.))
                    .text_color(glyph)
                    .child(label.to_string()),
            );
        if last {
            el = el.rounded_b_lg();
        }
        if active {
            el = el.bg(fg_mix(0.12));
        }
        el.on_hover(move |hovered, _, cx| {
            let _ = weak.update(cx, |this, _| {
                this.set_hover(&key, *hovered);
                this.sb_hover(*hovered, &key);
            });
        })
        .on_click({
            let weak = cx.weak_entity();
            let pid = pid.clone();
            move |_: &ClickEvent, _, cx| {
                let _ = weak.update(cx, |this, _| this.navigate(Route::Project(pid.clone())));
            }
        })
        .on_mouse_down(MouseButton::Right, {
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
        })
    }

    fn sb_hover(&mut self, hovered: bool, id: &str) {
        if hovered {
            self.sb_hover_id = Some(SharedString::from(id.to_string()));
        } else if self
            .sb_hover_id
            .as_ref()
            .map_or(false, |s| s.as_str() == id)
        {
            self.sb_hover_id = None;
        }
    }

    /// Advance the sliding highlight 160ms cubic-bezier(0.23,1,0.32,1)
    /// plus 80ms opacity fade.
    fn highlight_tick(&mut self, window: &mut Window) {
        let h = &mut self.sb_highlight;
        let now = Instant::now();
        let dt = now.duration_since(h.stamp).as_secs_f32() * 1000.0;
        h.stamp = now;
        let step = (dt / 160.0).min(1.0);
        h.y += (h.ty - h.y) * (1.0 - ease_emphasized(1.0 - step));
        h.hh += (h.th - h.hh) * (1.0 - ease_emphasized(1.0 - step));
        let ostep = dt / 80.0;
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

    pub(crate) fn render_sidebar(
        &mut self,
        p: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let settings = matches!(self.route, Route::Settings | Route::SettingsFuel);
        let w = SIDEBAR_W * p;
        let mut rects: Vec<(SharedString, SbRect)> = vec![];
        let mut y = 44.0f32; // pt40 shell + pt4 body

        let mut navs: Vec<gpui::Stateful<gpui::Div>> = vec![];
        if settings {
            navs.push(self.nav_btn(
                window,
                cx,
                "back",
                "icons/arrow-left.svg",
                "Назад",
                false,
                self.back_route.clone(),
            ));
            rects.push(("nav-back".into(), SbRect { y, h: 32. }));
            y += 33.;
            navs.push(self.nav_btn(
                window,
                cx,
                "general",
                "icons/settings.svg",
                "Общие",
                self.route == Route::Settings,
                Route::Settings,
            ));
            rects.push(("nav-general".into(), SbRect { y, h: 32. }));
            y += 33.;
            navs.push(self.nav_btn(
                window,
                cx,
                "fuel",
                "icons/star.svg",
                "Мыслетопливо",
                self.route == Route::SettingsFuel,
                Route::SettingsFuel,
            ));
            rects.push(("nav-fuel".into(), SbRect { y, h: 32. }));
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
                navs.push(self.nav_btn(window, cx, id, ic, label, active, route));
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
            let weak = cx.weak_entity();
            let header = div()
                .id("sb-projects-head")
                .h(px(32.))
                .flex()
                .items_center()
                .pl(px(10.))
                .rounded_t_lg()
                .bg(mix(FG, 0.05 + 0.03 * t_head, BG))
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
                        .text_color(rgba(FG, head_alpha))
                        .child(icon(
                            if self.kanban_group_open {
                                "icons/folder-open2.svg"
                            } else {
                                "icons/folder-closed.svg"
                            },
                            16.,
                            rgba(FG, head_alpha),
                        ))
                        .child("Проекты"),
                )
                .child(
                    // Vue: Sidebar "+" opens ProjectCreateDialog (doesn't toggle the group).
                    div()
                        .id("sb-projects-plus")
                        .w(px(32.))
                        .h_full()
                        .grid()
                        .items_center()
                        .justify_center()
                        .child(icon("icons/plus.svg", 16., rgba(FG, 0.6)))
                        .hover(|s| s.bg(fg_mix(0.10)))
                        .on_click({
                            let weak = weak.clone();
                            move |_: &ClickEvent, _, cx| {
                                cx.stop_propagation();
                                let _ = weak.update(cx, |this, _| {
                                    this.open_project_create();
                                });
                            }
                        }),
                )
                .on_hover({
                    let weak = weak.clone();
                    move |hovered, _, cx| {
                        let _ = weak.update(cx, |this, _| {
                            this.set_hover("nav-projects-head", *hovered);
                        });
                    }
                })
                .on_click({
                    let weak = weak.clone();
                    move |_: &ClickEvent, _, cx| {
                        let _ = weak.update(cx, |this, _| {
                            this.kanban_group_open = !this.kanban_group_open;
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

            let mut links: Vec<gpui::Stateful<gpui::Div>> = vec![];
            if self.kanban_group_open {
                let count = active_projects.len();
                for (i, pr) in active_projects.iter().enumerate() {
                    let pid = pr.id.to_string();
                    let active = matches!(&self.route, Route::Project(r) if *r == pid);
                    let id = format!("proj-{}", pr.id);
                    // Vue: label = `${icon} ${title}` when an icon is set.
                    let label = match &pr.icon {
                        Some(ic) => format!("{ic} {}", pr.title),
                        None => pr.title.clone(),
                    };
                    links.push(self.project_link(
                        window,
                        cx,
                        &id,
                        &label,
                        active,
                        i == count - 1,
                        &pid,
                    ));
                    rects.push((format!("nav-{id}").into(), SbRect { y, h: 32. }));
                    y += 32.;
                }
            }
            let list = div()
                .flex()
                .flex_col()
                .bg(fg_mix(0.03))
                .rounded_b_lg()
                .children(links);
            projects_el = Some(div().mt_4().flex().flex_col().child(header).child(list));
        }

        // Footer (.sidebar__part-20): "Другое" flex-1.
        let mut footer: Option<gpui::Stateful<gpui::Div>> = None;
        if !settings {
            let t = self.hover_t(window, "nav-more");
            let active = self.more_open;
            let glyph_alpha = if active { 1.0 } else { 0.6 + 0.4 * t };
            let glyph = rgba(FG, glyph_alpha);
            let weak = cx.weak_entity();
            let mut el = div()
                .id("sb-more")
                .min_h(px(32.))
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
            if self.more_open {
                el = el.bg(fg_mix(0.05)).rounded_b_lg();
            } else {
                el = el.rounded_lg();
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
                .on_click(move |_: &ClickEvent, _, cx| {
                    let _ = weak.update(cx, |this, _| {
                        this.more_open = !this.more_open;
                    });
                });
            footer = Some(el);
        }

        // Resolve highlight target now that rects are known.
        let sidebar_h = window.viewport_size().height;
        if let Some(footer_el) = &footer {
            let _ = footer_el;
            let fy: f32 = sidebar_h.into();
            rects.push((
                "nav-more".into(),
                SbRect {
                    y: fy - 32. - 12.,
                    h: 32.,
                },
            ));
        }
        if let Some(id) = self.sb_hover_id.clone() {
            if let Some((_, r)) = rects.iter().find(|(rid, _)| *rid == id) {
                self.sb_highlight.ty = r.y;
                self.sb_highlight.th = r.h;
                if self.sb_highlight.opacity == 0.0 {
                    // Appear directly at target (no travel on first hover).
                    self.sb_highlight.y = r.y;
                    self.sb_highlight.hh = r.h;
                }
                self.sb_highlight.visible = true;
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
            .bg(c(BG))
            .px_2()
            .pt(px(40.))
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
                    .bg(rgba(FG, 0.06 * self.sb_highlight.opacity)),
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
                    .border_color(rgba(SIDEBAR_DIVIDER, if p > 0.5 { 1.0 } else { 0.0 }))
                    .child(shell),
            )
    }

    pub(crate) fn render_titlebar(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = ease_emphasized(self.sidebar_t.clamp(0.0, 1.0));
        let pl = 48.0 + (16.0 - 48.0) * p;
        let (title, icon_path) = self.page_title();
        div()
            .h(px(TITLEBAR_H))
            .flex_none()
            .w_full()
            .flex()
            .items_center()
            .pl(px(pl))
            .pr_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .min_w_0()
                    .child(icon(icon_path, 18., rgba(FG, 0.82)))
                    .child(
                        div()
                            .text_size(px(13.))
                            .line_height(px(15.))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(c(FG))
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(title),
                    ),
            )
            .into_any_element()
    }

    /// Floating sidebar toggle (always top-left, over sidebar or titlebar).
    pub(crate) fn render_sidebar_toggle(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let t = self.hover_t(window, "sb-toggle");
        let weak = cx.weak_entity();
        div()
            .absolute()
            .top_0()
            .left_0()
            .h(px(TITLEBAR_H))
            .pl_3()
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
                    .bg(rgba(FG, 0.08 * t))
                    .child(icon("icons/sidebar-left.svg", 18., rgba(FG, 0.6 + 0.4 * t)))
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

    /// "Другое" popover: opens upward from the footer button, width = button
    /// width (sidebar inner width), bg fg3%, radius top lg, shadow-lg.
    pub(crate) fn render_more_menu(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
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
        let mut rows: Vec<gpui::Stateful<gpui::Div>> = vec![];
        for (id, ic, label, route) in items {
            let t = self.hover_t(window, &format!("more-{id}"));
            let active = self.route == route;
            let glyph_alpha = if active { 1.0 } else { 0.6 + 0.4 * t };
            let glyph = rgba(FG, glyph_alpha);
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
                el = el.bg(fg_mix(0.12));
            } else {
                el = el.bg(fg_mix(0.08 * t));
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

        let menu = div()
            .absolute()
            .left_2()
            .bottom(px(44.))
            .w(px(SIDEBAR_W - 16.))
            .occlude()
            .flex()
            .flex_col()
            .rounded_t_lg()
            .border_1()
            .border_color(c(BORDER))
            .bg(c(BG))
            .shadow(vec![gpui::BoxShadow {
                color: rgba(0x000000, 0.16),
                offset: gpui::point(px(0.), px(8.)),
                blur_radius: px(24.),
                spread_radius: px(0.),
            }])
            .overflow_hidden()
            .children(rows);

        // Click-away backdrop.
        let weak = cx.weak_entity();
        div()
            .absolute()
            .inset_0()
            .child(div().id("more-backdrop").size_full().on_mouse_down(
                MouseButton::Left,
                move |_: &MouseDownEvent, _, cx| {
                    let _ = weak.update(cx, |this, _| this.more_open = false);
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
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let weak = cx.weak_entity();
        let mut rows: Vec<gpui::Stateful<gpui::Div>> = vec![];
        for (i, (label, destructive, action)) in menu.items.iter().enumerate() {
            let t = self.hover_t(window, &format!("ctx-{i}"));
            let action = action.clone();
            let color = if *destructive { c(DESTRUCTIVE) } else { c(FG) };
            let key = SharedString::from(format!("ctx-{i}"));
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
                    .bg(fg_mix(0.06 * t))
                    .child(label.clone())
                    .on_hover({
                        let weak = weak.clone();
                        move |hovered, _, cx| {
                            let _ = weak.update(cx, |this, _| this.set_hover(&key, *hovered));
                        }
                    })
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
            .occlude()
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
