use super::*;

impl Agenda {
    // ==========================================================================
    // FAB (.btn-primary md = h40 px16).
    // ==========================================================================

    pub(crate) fn render_fab(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let t = self.hover_t(window, "fab");
        // Fade out as the quick-entry panel slides in — the panel takes over
        // the add-task role, so the button also stops being clickable.
        let fade = 1.0 - self.qe_now().clamp(0.0, 1.0);
        let clickable = fade > 0.5;
        let weak = cx.weak_entity();
        let fab = div()
            .id("fab-new")
            .debug_selector(|| "fab-new".to_string())
            .absolute()
            .right_6()
            .bottom_6()
            .w_11()
            .h_11()
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .bg(mix(FG(), 1.0 - 0.12 * t, BG()))
            .text_color(c(BG()))
            .shadow(vec![gpui::BoxShadow {
                color: rgba(0x000000, 0.2),
                offset: gpui::point(px(0.), px(4.)),
                blur_radius: px(14.),
                spread_radius: px(0.),
                inset: false,
            }])
            .child(icon("icons/plus.svg", 18., c(BG())))
            .opacity(fade)
            .on_hover(move |hovered, _, cx| {
                let _ = weak.update(cx, |this, _| this.set_hover("fab", *hovered));
            })
            .when(clickable, |el| {
                el.on_click({
                    let weak = cx.weak_entity();
                    move |_: &ClickEvent, window, cx| {
                        let _ = weak.update(cx, |this, cx| {
                            this.set_quick_entry(true, cx);
                            this.reset_quick_entry(window, cx);
                        });
                    }
                })
            })
            .a11y_button("Новая задача");
        div()
            .size_full()
            .absolute()
            .inset_0()
            .child(fab)
            .into_any_element()
    }
}
