use super::*;

impl Agenda {
    // ==========================================================================
    // Pages
    // ==========================================================================

    /// Archived-project row in the logbook list — same 37px chrome as task
    /// rows so it can share the virtualized slot.
    pub(crate) fn archived_project_row(
        &mut self,
        p: &Project,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let hid = format!("lb-{}", p.id);
        let t = self.hover_t(window, &hid);
        let weak = cx.weak_entity();
        let pid = p.id.to_string();
        let key = SharedString::from(hid.clone());
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
                let weak = cx.weak_entity();
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
            })
    }

    pub(crate) fn logbook_page(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let items = filter_idx(SmartList::Logbook, &self.todos);
        let archived: Vec<Project> = self
            .projects
            .iter()
            .filter(|p| p.status == 2)
            .cloned()
            .collect();
        let mut rows: Vec<FlatRow> = vec![];
        if !items.is_empty() {
            rows.push(FlatRow::Header("Задачи".into(), items.len()));
            rows.extend(items.into_iter().map(FlatRow::Task));
        }
        if !archived.is_empty() {
            rows.push(FlatRow::Header("Проекты".into(), archived.len()));
            rows.extend(archived.into_iter().map(FlatRow::ArchivedProject));
        }
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(if rows.is_empty() {
                empty_state().into_any_element()
            } else {
                self.task_list(
                    "list-logbook".to_string(),
                    "logbook",
                    std::rc::Rc::new(rows),
                    true,
                    cx,
                )
            })
            .into_any_element()
    }
}
