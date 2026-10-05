use super::*;
use gpui::Focusable;
mod fields;
mod properties;

impl Agenda {
    pub(crate) fn qe_panel(
        &mut self,
        p: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        // contextual defaults — once the user picks/clears the chip
        // (`qe_project_touched`), a cleared state must not be re-seeded.
        if self.qe_project.is_none() && !self.qe_project_touched {
            if let Route::Project(pid) = &self.route {
                self.qe_project = Some(pid.clone());
            }
        }
        if !self.qe_date_touched && self.route == Route::Week {
            self.qe_week = true;
        }
        let title_state = self.qe_title_state(window, cx);
        let notes_state = self.qe_notes_state(window, cx);
        // Reseed only on task switch — reseeding every render where the field
        // is empty would resurrect model text under the user's cursor.
        if let Some(id) = self.qe_task.clone() {
            if self.input_task.as_deref() != Some(id.as_str()) {
                self.input_task = Some(id.clone());
                if let Some(t) = self.todo(&id) {
                    let tv = t.title.clone();
                    let nv = t.notes.clone().unwrap_or_default();
                    title_state.update(cx, |s, cx| s.set_value(tv, window, cx));
                    notes_state.update(cx, |s, cx| s.set_value(nv, window, cx));
                }
            }
        }
        // Focus on opening only. Reclaiming it on every render steals focus
        // from quick search and other controls as soon as they repaint.
        if self.quick_entry_open && !self.qe_focused && !self.quick_open {
            let title_focused = title_state
                .read(cx)
                .focus_handle(cx)
                .contains_focused(window, cx);
            let notes_focused = notes_state
                .read(cx)
                .focus_handle(cx)
                .contains_focused(window, cx);
            if !title_focused && !notes_focused {
                title_state.update(cx, |s, cx| s.focus(window, cx));
            }
            self.qe_focused = true;
        }

        let weak = cx.weak_entity();
        let title_empty = title_state.read(cx).value().trim().is_empty();

        // Edit mode: the same two property rows bind to the open task —
        // the panel is visually identical to task creation.
        let editing = self.qe_task.as_ref().and_then(|id| self.todo(id)).cloned();
        let next_mon = next_monday_key(&today_key());
        let (date_label, date_has, date_kind, date_todo) = if let Some(t) = &editing {
            let (d, _) = task_date(t);
            let label = if t.is_today && d.is_none() {
                "Эта неделя".to_string()
            } else {
                d.as_ref()
                    .map(|v| {
                        if Some(v) == next_mon.as_ref() {
                            "Следующая неделя".to_string()
                        } else {
                            fmt_day_month(v)
                        }
                    })
                    .unwrap_or_else(|| "Пусто".to_string())
            };
            (
                label,
                d.is_some() || t.is_today,
                DropKind::Date,
                self.qe_task.clone(),
            )
        } else {
            (
                self.qe_date_label(),
                self.qe_someday || self.qe_week || self.qe_date.is_some(),
                DropKind::QeDate,
                None,
            )
        };
        let date_row = self.qe_prop_row(
            window,
            cx,
            properties::Property {
                id: "qe-date",
                icon_path: "icons/calendar-02.svg",
                label: "Дата",
                value: date_label,
                has_value: date_has,
                kind: date_kind,
                clearable: editing.is_none(),
                todo_id: date_todo,
            },
        );
        let (proj_label, proj_has, proj_kind, proj_todo) = if let Some(t) = &editing {
            let pid = t.project_id.clone();
            let label = pid
                .as_ref()
                .and_then(|pid| self.projects.iter().find(|p| p.id == pid.as_str()))
                .map(|p| p.title.clone())
                .unwrap_or_else(|| "Без проекта".to_string());
            (
                label,
                pid.is_some(),
                DropKind::Project,
                self.qe_task.clone(),
            )
        } else {
            let label = self
                .qe_project
                .as_ref()
                .and_then(|pid| self.projects.iter().find(|p| p.id == pid.as_str()))
                .map(|p| p.title.clone())
                .unwrap_or_else(|| "Без проекта".to_string());
            (label, self.qe_project.is_some(), DropKind::QeProject, None)
        };
        let proj_row = self.qe_prop_row(
            window,
            cx,
            properties::Property {
                id: "qe-proj",
                icon_path: "icons/folder.svg",
                label: "Проект",
                value: proj_label,
                has_value: proj_has,
                kind: proj_kind,
                clearable: false,
                todo_id: proj_todo,
            },
        );

        let collapse_t = self.hover_t(window, "qe-collapse");
        let collapse = div()
            .id("qe-close")
            .debug_selector(|| "qe-close".to_string())
            .w_7()
            .h_7()
            .grid()
            .items_center()
            .justify_center()
            .rounded_md()
            .hover(|s| s.bg(rgba(FG(), 0.08)))
            .cursor_pointer()
            .child(icon(
                "icons/chevrons-right.svg",
                18.,
                rgba(FG(), 0.6 + 0.4 * collapse_t),
            ))
            .on_hover({
                let weak = weak.clone();
                move |hovered, _, cx| {
                    let _ = weak.update(cx, |this, _| this.set_hover("qe-collapse", *hovered));
                }
            })
            .on_click({
                let weak = weak.clone();
                move |_: &ClickEvent, window, cx| {
                    let _ = weak.update(cx, |this, cx| {
                        this.set_quick_entry(false, cx);
                        this.qe_focused = false;
                        this.root_focus.focus(window, cx);
                    });
                }
            })
            .a11y_button("Скрыть панель");

        let save_hover = self.hover_t(window, "qe-save");
        let save = div()
            .id("qe-save")
            .debug_selector(|| "qe-save".to_string())
            .h_9()
            .w_full()
            .rounded_lg()
            .flex()
            .items_center()
            .justify_center()
            .text_size(crate::theme::text_px(13.))
            .font_weight(gpui::FontWeight::MEDIUM)
            .when(title_empty, |el| {
                el.bg(panel_mix(0.10)).text_color(c(MUTED_FG()))
            })
            .when(!title_empty, |el| {
                // White/neutral primary — foreground fill, background text.
                el.bg(mix(FG(), 1.0 - 0.12 * save_hover, BG()))
                    .text_color(c(BG()))
                    .cursor_pointer()
                    .on_click({
                        let weak = weak.clone();
                        move |_: &ClickEvent, _, cx| {
                            let _ = weak.update(cx, |this, cx| this.quick_entry_save(cx));
                        }
                    })
            })
            .child("Создать")
            .a11y_button("Создать задачу");

        let panel = div()
            .id("qe-panel")
            .debug_selector(|| "qe-panel".to_string())
            .relative()
            .w(px(self.qe_w))
            .h_full()
            .ml_auto()
            .flex_none()
            .flex()
            .flex_col()
            //  — чуть серее основного фона, как у дропдаунов.
            .bg(c(POPOVER()))
            .border_l_1()
            .border_color(panel_mix(0.10))
            .shadow(vec![gpui::BoxShadow {
                color: rgba(0x000000, 0.20),
                offset: gpui::point(px(-8.), px(0.)),
                blur_radius: px(32.),
                spread_radius: px(0.),
                inset: false,
            }])
            .child(
                div()
                    .flex()
                    .items_center()
                    .pl_2()
                    .h(px(40.))
                    .flex_none()
                    .child(collapse),
            )
            // H1 title — the task name is the page heading, Notion-style.
            // Left edge shares the icon line (icons sit at ~22px).
            .child(fields::title(&title_state, cx))
            .child(
                div()
                    .pl_4()
                    .pr_3()
                    .pt_5()
                    .pb_4()
                    .flex_none()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .child(date_row)
                    .child(proj_row),
            )
            // Notes — a free-form page body under the properties, like the
            // content area of a Notion page. Fills the remaining height.
            .child(fields::notes(&notes_state, cx))
            .when(editing.is_none(), |el| {
                el.child(
                    div()
                        .p_3()
                        .flex_none()
                        .border_t_1()
                        .border_color(panel_mix(0.10))
                        .child(save),
                )
            })
            // Left-edge resize grab: live-updates `qe_w` (see root
            // on_mouse_move/on_mouse_up), zeron pane-resize style.
            .child(
                div()
                    .id("qe-resize")
                    .debug_selector(|| "qe-resize".to_string())
                    .absolute()
                    .left(px(-3.))
                    .top_0()
                    .bottom_0()
                    .w(px(8.))
                    .cursor_col_resize()
                    .on_mouse_down(MouseButton::Left, {
                        let weak = weak.clone();
                        move |ev: &MouseDownEvent, _, cx| {
                            cx.stop_propagation();
                            let _ = weak.update(cx, |this, _| {
                                this.qe_resizing = Some((f32::from(ev.position.x), this.qe_w));
                            });
                        }
                    }),
            );

        // The outer column carries the animated width; the inner panel keeps
        // a fixed width anchored right, so content slides rather than squishes.
        div()
            .h_full()
            .w(px(self.qe_w * p))
            .flex_none()
            .overflow_hidden()
            .child(panel)
            .into_any_element()
    }
}
