use super::*;
use gpui::ClipboardItem;
use gpui_component::tooltip::Tooltip;

impl Agenda {
    // ==========================================================================
    // Statistics: summary card + contribution heatmap + insights (settings page)
    // ==========================================================================

    pub(crate) fn statistics_page(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let today = Local::now().date_naive();

        // Per-day completed counts for all history.
        let mut days = std::collections::BTreeMap::<NaiveDate, i64>::new();
        for t in &self.todos {
            if let Some(d) = date_only(&t.completed_at).and_then(|k| parse_key(&k)) {
                *days.entry(d).or_insert(0) += 1;
            }
        }
        let total_done: i64 = days.values().sum();
        let last7: i64 = days
            .iter()
            .filter(|(d, _)| **d > today - Duration::days(7))
            .map(|(_, c)| c)
            .sum();
        let last30: i64 = days
            .iter()
            .filter(|(d, _)| **d > today - Duration::days(30))
            .map(|(_, c)| c)
            .sum();

        // Streaks: current streak may start yesterday if today is still empty.
        let mut cur_streak = 0i64;
        let mut d = today;
        if !days.contains_key(&d) {
            d -= Duration::days(1);
        }
        while days.contains_key(&d) {
            cur_streak += 1;
            d -= Duration::days(1);
        }
        let mut best_streak = 0i64;
        let mut run = 0i64;
        let mut prev: Option<NaiveDate> = None;
        for day in days.keys() {
            run = if prev == Some(*day - Duration::days(1)) {
                run + 1
            } else {
                1
            };
            best_streak = best_streak.max(run);
            prev = Some(*day);
        }

        // Summary strip (bordered card, one row, thin separators).
        let stat_items = [
            (format!("{total_done}"), "Всего выполнено"),
            (format!("{last7}"), "За 7 дней"),
            (format!("{last30}"), "За 30 дней"),
            (format!("{cur_streak}"), "Текущая серия"),
            (format!("{best_streak}"), "Лучшая серия"),
        ];
        let mut card = div()
            .flex()
            .items_stretch()
            .rounded_lg()
            .border_1()
            .border_color(c(BORDER()))
            .py_3();
        let last_i = stat_items.len() - 1;
        for (i, (value, label)) in stat_items.into_iter().enumerate() {
            card = card.child(
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
                            .text_color(c(FG()))
                            .child(value),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(c(MUTED_FG()))
                            .child(label),
                    ),
            );
            if i != last_i {
                card = card.child(div().w(px(1.)).my_1().bg(fg_mix(0.08)));
            }
        }

        // GitHub-style contribution heatmap: columns = calendar weeks
        // (Mon–Sun), rows = weekday. The last column is the current week and
        // the first one starts on the Monday `weeks-1` weeks ago. Width adapts
        // to the viewport (~a year on a wide window).
        const CELL: f32 = 12.0;
        const GAP: f32 = 3.0;
        let grid_w = (window.viewport_size().width - px(240.0 + 64.0)).min(px(832.));
        let weeks = ((grid_w / px(CELL + GAP)) as usize).clamp(4, 53);

        let dow = today.weekday().num_days_from_monday() as i64;
        let last_monday = today - Duration::days(dow);
        let start = last_monday - Duration::days(7 * (weeks as i64 - 1));
        let total = weeks * 7;

        let mut vals = vec![0f64; total];
        let mut max_v = 0f64;
        for (day, count) in &days {
            let idx = (*day - start).num_days();
            if (0..total as i64).contains(&idx) {
                vals[idx as usize] = *count as f64;
                max_v = max_v.max(*count as f64);
            }
        }

        let cell_bg = |v: f64| {
            if v <= 0.0 {
                fg_mix(0.06)
            } else {
                rgba(
                    ACCENT(),
                    0.25 + 0.75 * ((v / max_v.max(1.0)).min(1.0) as f32),
                )
            }
        };

        // Month captions under the week containing the 1st of a month
        // (3-letter). A label is written only when the month fits: its 1st
        // day must lie inside the grid and two columns must remain so the
        // text does not overflow the right edge.
        let mut months = div().flex().gap(px(GAP)).h(px(16.)).mt_3().flex_none();
        for w in 0..weeks {
            let monday = start + Duration::days(7 * w as i64);
            let sunday = monday + Duration::days(6);
            // The 1st of a month falls inside this week.
            let first = if monday.day() == 1 {
                Some(monday)
            } else if sunday.month() != monday.month() {
                NaiveDate::from_ymd_opt(sunday.year(), sunday.month(), 1)
            } else {
                None
            };
            let label = match first {
                Some(f) if f >= start && w + 2 <= weeks => MONTH_SHORT[(f.month() - 1) as usize]
                    .chars()
                    .take(3)
                    .collect::<String>(),
                _ => String::new(),
            };
            months = months.child(
                div()
                    .w(px(CELL))
                    .flex_none()
                    .text_size(px(10.))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(c(MUTED_FG()))
                    .whitespace_nowrap()
                    .child(label),
            );
        }

        let mut cols = div().flex().gap(px(GAP)).flex_none();
        for w in 0..weeks {
            let mut col = div().flex().flex_col().gap(px(GAP));
            for row in 0..7usize {
                let idx = w * 7 + row;
                let date = start + Duration::days(idx as i64);
                if date > today {
                    break; // the grid ends at today, no future cells
                }
                let v = vals[idx];
                let caption = format!(
                    "{} задач {} {}",
                    v as i64,
                    date.day(),
                    MONTH_LONG[(date.month() - 1) as usize]
                );
                col = col.child(
                    div()
                        .id(SharedString::from(format!("heat-{idx}")))
                        .size(px(CELL))
                        .flex_none()
                        .rounded(px(2.5))
                        .bg(cell_bg(v))
                        .tooltip(move |window, cx| Tooltip::new(caption.clone()).build(window, cx)),
                );
            }
            cols = cols.child(col);
        }

        let grid = div().mt_3().flex().gap_0().child(cols);

        // "Поделиться" button, top-right like the reference header.
        let share_t = self.hover_t(window, "stat-share");
        let weak = cx.weak_entity();
        let share_btn = div()
            .id("el-stat-share")
            .size(px(32.))
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .bg(fg_mix(0.05 + 0.05 * share_t))
            .child(icon("icons/share.svg", 16., rgba(FG(), 0.9)))
            .on_hover(move |hovered, _, cx| {
                let _ = weak.update(cx, |this, _| this.set_hover("stat-share", *hovered));
            })
            .on_click({
                let weak = cx.weak_entity();
                move |_: &ClickEvent, _, cx| {
                    let _ = weak.update(cx, |this, _| this.share_open = true);
                }
            });

        // Share overlay: dim backdrop + share card + copy button.
        let share_overlay = if self.share_open {
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
                                }),
                        ),
                )
                .into_any_element(),
            )
        } else {
            None
        };

        // Profile header: empty avatar placeholder + stub name (wired up later).
        let profile = div()
            .mt_2()
            .flex()
            .flex_col()
            .items_center()
            .child(
                div()
                    .size(px(72.))
                    .rounded_full()
                    .border_1()
                    .border_color(fg_mix(0.12))
                    .bg(fg_mix(0.06)),
            )
            .child(
                div()
                    .mt_3()
                    .text_size(px(20.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(c(FG()))
                    .child("Имя Фамилия"),
            );

        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .relative()
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .id("stats-scroll")
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll("stats"))
                    .child(
                        div()
                            .w_full()
                            .max_w(px(896.))
                            .mx_auto()
                            .pt_4()
                            .px_8()
                            .flex()
                            .flex_col()
                            .child(div().flex().justify_end().child(share_btn))
                            .child(profile)
                            .child(div().mt_4().child(card))
                            .child(
                                div()
                                    .mt_6()
                                    .text_size(px(15.))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child("Активность"),
                            )
                            .child(grid)
                            .child(months)
                            .child(div().pb_8()),
                    ),
            )
            .when_some(share_overlay, |el, overlay| el.child(overlay))
            .into_any_element()
    }
}

/// Grab the card's on-screen region as pixels (Windows: BitBlt from the
/// screen DC; the overlay is always in the foreground when this runs).
#[cfg(target_os = "windows")]
fn capture_card(window: &Window, bounds: gpui::Bounds<gpui::Pixels>) -> Option<image::RgbaImage> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::Foundation::{HWND, POINT};
    use windows::Win32::Graphics::Gdi::*;

    let raw = HasWindowHandle::window_handle(window).ok()?.as_raw();
    let hwnd = match raw {
        RawWindowHandle::Win32(h) => HWND(h.hwnd.get() as *mut _),
        _ => return None,
    };
    let scale = window.scale_factor();

    unsafe {
        let mut pt = POINT::default();
        if !ClientToScreen(hwnd, &mut pt).as_bool() {
            return None;
        }
        let x = pt.x + (f32::from(bounds.origin.x) * scale).round() as i32;
        let y = pt.y + (f32::from(bounds.origin.y) * scale).round() as i32;
        let w = (f32::from(bounds.size.width) * scale).round() as i32;
        let h = (f32::from(bounds.size.height) * scale).round() as i32;
        if w <= 0 || h <= 0 {
            return None;
        }

        let screen = GetDC(None);
        let mem = CreateCompatibleDC(Some(screen));
        let bmp = CreateCompatibleBitmap(screen, w, h);
        let old = SelectObject(mem, bmp.into());
        let _ = BitBlt(mem, 0, 0, w, h, Some(screen), x, y, SRCCOPY);

        let mut bmi = BITMAPINFO::default();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = w;
        bmi.bmiHeader.biHeight = -h; // top-down rows
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB.0;
        let mut buf = vec![0u8; (w * h * 4) as usize];
        let got = GetDIBits(
            mem,
            bmp,
            0,
            h as u32,
            Some(buf.as_mut_ptr().cast()),
            &mut bmi,
            DIB_RGB_COLORS,
        );
        SelectObject(mem, old);
        let _ = DeleteObject(bmp.into());
        let _ = DeleteDC(mem);
        let _ = ReleaseDC(None, screen);
        if got == 0 {
            return None;
        }

        for p in buf.chunks_exact_mut(4) {
            p.swap(0, 2); // BGRA -> RGBA
            p[3] = 255;
        }
        image::RgbaImage::from_raw(w as u32, h as u32, buf)
    }
}

/// Copy the card image to the Windows clipboard in a single open session:
/// registered "PNG" format + classic bottom-up CF_DIB (for apps that don't
/// understand PNG clipboard data).
#[cfg(target_os = "windows")]
fn write_card_to_clipboard(img: &image::RgbaImage) -> bool {
    use windows::core::s;
    use windows::Win32::Foundation::{GlobalFree, HANDLE};
    use windows::Win32::Graphics::Gdi::BITMAPINFOHEADER;
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatA, SetClipboardData,
    };
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

    const CF_DIB: u32 = 8;
    let (w, h) = (img.width() as usize, img.height() as usize);

    let mut png = Vec::new();
    if image::DynamicImage::ImageRgba8(img.clone())
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .is_err()
    {
        return false;
    }

    // Bottom-up BGRA DIB: BITMAPINFOHEADER + pixel data.
    let header_len = std::mem::size_of::<BITMAPINFOHEADER>();
    let data_len = w * h * 4;
    let mut dib = vec![0u8; header_len + data_len];
    unsafe {
        (dib.as_mut_ptr() as *mut BITMAPINFOHEADER).write(BITMAPINFOHEADER {
            biSize: header_len as u32,
            biWidth: w as i32,
            biHeight: h as i32, // positive = bottom-up rows
            biPlanes: 1,
            biBitCount: 32,
            biCompression: 0, // BI_RGB
            biSizeImage: data_len as u32,
            ..Default::default()
        });
    }
    let raw = img.as_raw();
    for row in 0..h {
        let src = &raw[(h - 1 - row) * w * 4..(h - row) * w * 4];
        let dst = &mut dib[header_len + row * w * 4..header_len + (row + 1) * w * 4];
        for (i, px) in src.chunks_exact(4).enumerate() {
            dst[i * 4] = px[2];
            dst[i * 4 + 1] = px[1];
            dst[i * 4 + 2] = px[0];
            dst[i * 4 + 3] = 255;
        }
    }

    /// Move `data` onto the clipboard under `format`; false on failure.
    unsafe fn put(format: u32, data: &[u8]) -> bool {
        let Ok(hglob) = GlobalAlloc(GMEM_MOVEABLE, data.len()) else {
            return false;
        };
        let ptr = GlobalLock(hglob).cast::<u8>();
        if ptr.is_null() {
            let _ = GlobalFree(Some(hglob));
            return false;
        }
        std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
        let _ = GlobalUnlock(hglob);
        if SetClipboardData(format, Some(HANDLE(hglob.0 as *mut _))).is_err() {
            let _ = GlobalFree(Some(hglob));
            return false;
        }
        true
    }

    unsafe {
        // Clipboard contention is transient — retry briefly.
        let mut opened = false;
        for _ in 0..5 {
            if OpenClipboard(None).is_ok() {
                opened = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        if !opened {
            eprintln!("[share] OpenClipboard failed");
            return false;
        }
        let _ = EmptyClipboard();
        let png_fmt = RegisterClipboardFormatA(s!("PNG"));
        let ok_dib = put(CF_DIB, &dib);
        let ok_png = put(png_fmt, &png);
        eprintln!("[share] clipboard: dib={ok_dib} png={ok_png}");
        let _ = CloseClipboard();
        ok_dib || ok_png
    }
}

#[cfg(not(target_os = "windows"))]
fn capture_card(_window: &Window, _bounds: gpui::Bounds<gpui::Pixels>) -> Option<image::RgbaImage> {
    None
}

#[cfg(not(target_os = "windows"))]
fn write_card_to_clipboard(_img: &image::RgbaImage) -> bool {
    false
}
