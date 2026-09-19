use super::*;

impl Agenda {
    // ==========================================================================
    // Pages
    // ==========================================================================

    pub(crate) fn logbook_page(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
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
                    .text_color(c(MUTED_FG()))
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
                    .text_color(c(MUTED_FG()))
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
                        .child(icon("icons/folder.svg", 16., c(MUTED_FG())))
                        .child(div().flex_1().text_color(c(FG())).child(p.title.clone()))
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
            .child(list)
            .into_any_element()
    }
}
