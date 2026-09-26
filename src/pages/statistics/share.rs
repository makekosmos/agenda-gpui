use super::clipboard::{capture_card, write_card_to_clipboard};
use super::*;
use gpui::ClipboardItem;

impl Agenda {
    pub(super) fn statistics_share_overlay(
        &mut self,
        today: NaiveDate,
        days: &std::collections::BTreeMap<NaiveDate, i64>,
        cell_bg: impl Fn(f64) -> HslaAlias,
        stats: [i64; 4],
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let [total_done, last7, cur_streak, best_streak] = stats;
        let last_monday = today - Duration::days(today.weekday().num_days_from_monday() as i64);
        // Share overlay: dim backdrop + share card + copy button.
        if self.share_open {
            const CCELL: f32 = 14.0;
            const CGAP: f32 = 4.0;
            let cweeks = 28usize; // half a year inside the card
            let cstart = last_monday - Duration::days(7 * (cweeks as i64 - 1));

            let mut ccols = div().flex().gap(px(CGAP)).w_full();
            for w in 0..cweeks {
                let mut col = div().flex_1().min_w_0().flex().flex_col().gap(px(CGAP));
                for row in 0..7usize {
                    let date = cstart + Duration::days((w * 7 + row) as i64);
                    if date > today {
                        break;
                    }
                    let v = days.get(&date).copied().unwrap_or(0) as f64;
                    col = col.child(div().w_full().h(px(CCELL)).rounded(px(3.)).bg(cell_bg(v)));
                }
                ccols = ccols.child(col);
            }

            let card_stats = [
                (format!("{total_done}"), "всего"),
                (format!("{last7}"), "за неделю"),
                (format!("{cur_streak} дн."), "серия"),
                (format!("{best_streak} дн."), "лучшая"),
            ];
            let mut cstats = div().flex().items_stretch().mt_4();
            let clast = card_stats.len() - 1;
            for (i, (value, label)) in card_stats.into_iter().enumerate() {
                cstats = cstats.child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap_0p5()
                        .child(
                            div()
                                .text_size(px(16.))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child(value),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(c(MUTED_FG()))
                                .child(label),
                        ),
                );
                if i != clast {
                    cstats = cstats.child(div().w(px(1.)).my_1().bg(fg_mix(0.08)));
                }
            }

            // Canvas records the card's on-screen bounds so "Скопировать" can
            // grab exactly this region into a PNG.
            let card_bounds =
                std::rc::Rc::new(std::cell::Cell::new(None::<gpui::Bounds<gpui::Pixels>>));
            let bounds_probe = {
                let card_bounds = card_bounds.clone();
                gpui::canvas(move |b, _, _| card_bounds.set(Some(b)), |_, _, _, _| {})
                    .absolute()
                    .inset_0()
            };

            let card = div()
                .w(px(560.))
                .rounded_xl()
                .border_1()
                .border_color(c(BORDER()))
                .bg(c(SECONDARY()))
                .p_5()
                .flex()
                .flex_col()
                .relative()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(bounds_probe)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .child(
                            div()
                                .size(px(40.))
                                .rounded_full()
                                .border_1()
                                .border_color(fg_mix(0.12))
                                .bg(fg_mix(0.06)),
                        )
                        .child(
                            div().ml_3().flex().flex_col().child(
                                div()
                                    .text_size(px(15.))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child("Имя Фамилия"),
                            ),
                        )
                        .child(div().flex_1())
                        .child(
                            div()
                                .text_size(px(14.))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(c(MUTED_FG()))
                                .child("Agenda"),
                        ),
                )
                .child(div().mt_4().child(ccols))
                .child(cstats);

            let weak_close = cx.weak_entity();
            let weak_copy = cx.weak_entity();
            let copy_text = format!(
                "Agenda: выполнено {total_done} задач, за неделю {last7}, серия {cur_streak} дн., лучшая серия {best_streak} дн."
            );
            Some(
                deferred(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .bg(rgba(0x000000, 0.5))
                        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                            let _ = weak_close.update(cx, |this, _| this.share_open = false);
                        })
                        .child(
                            div()
                                .text_size(px(15.))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .mb_4()
                                .child("Поделиться активностью"),
                        )
                        .child(card)
                        .child(
                            div()
                                .id("el-share-copy")
                                .mt_5()
                                .h_9()
                                .px_4()
                                .flex()
                                .items_center()
                                .rounded_full()
                                .bg(c(FG()))
                                .text_color(c(BG()))
                                .text_size(px(13.))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .child("Скопировать")
                                .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                    cx.stop_propagation();
                                })
                                .on_click(move |_: &ClickEvent, window, cx| {
                                    // Copy the card region as a PNG image.
                                    let ok = card_bounds
                                        .get()
                                        .and_then(|b| capture_card(window, b))
                                        .map(|img| write_card_to_clipboard(&img))
                                        .unwrap_or(false);
                                    if !ok {
                                        cx.write_to_clipboard(ClipboardItem::new_string(
                                            copy_text.clone(),
                                        ));
                                    }
                                    let _ = weak_copy.update(cx, |this, _| this.share_open = false);
                                })
                                .a11y_button("Скопировать"),
                        ),
                )
                .into_any_element(),
            )
        } else {
            None
        }
    }
}
