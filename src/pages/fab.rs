use super::*;

impl Agenda {
    // ==========================================================================
    // FAB (.btn-primary md = h40 px16).
    // ==========================================================================

    pub(crate) fn render_fab(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let t = self.hover_t(window, "fab");
        let weak = cx.weak_entity();
        let fab = div()
            .id("fab-new")
            .absolute()
            .right_6()
            .bottom_6()
            .h_10()
            .px_4()
            .flex()
            .items_center()
            .gap_2()
            .rounded_lg()
            .bg(lerp(ACCENT(), ACCENT_DIM(), t * 0.5))
            .text_color(c(ACCENT_FG()))
            .text_size(px(14.))
            .font_weight(gpui::FontWeight::MEDIUM)
            .shadow(vec![gpui::BoxShadow {
                color: rgba(0x000000, 0.15),
                offset: gpui::point(px(0.), px(4.)),
                blur_radius: px(12.),
                spread_radius: px(0.),
                inset: false,
            }])
            .child(icon("icons/plus.svg", 16., c(ACCENT_FG())))
            .child("Новая задача")
            .on_hover(move |hovered, _, cx| {
                let _ = weak.update(cx, |this, _| this.set_hover("fab", *hovered));
            })
            .on_click({
                let weak = cx.weak_entity();
                move |_: &ClickEvent, _, cx| {
                    let _ = weak.update(cx, |this, _| {
                        this.quick_entry_open = true;
                    });
                }
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
