// Shared icon/ring/bars/button widgets matching the Vue components pixel-for-pixel.
use gpui::prelude::*;
use gpui::{canvas, div, point, px, svg, Hsla, PathBuilder, Styled};

use crate::model::Status;
use crate::theme::*;

/// Render a bundled monochrome SVG icon at `size` px tinted `color`.
pub fn icon(path: &'static str, size: f32, color: Hsla) -> impl IntoElement {
    svg()
        .path(path)
        .w(px(size))
        .h(px(size))
        .flex_none()
        .text_color(color)
}

// ---------------------------------------------------------------------------
// TaskStatusIcon — 14px ring; done=accent+check, canceled=x, started=half fill,
// deferred=dashed ring, else plain ring.
// ---------------------------------------------------------------------------

pub fn status_ring(status: Status) -> impl IntoElement {
    let ring = |border: Hsla| {
        div()
            .w(px(14.))
            .h(px(14.))
            .rounded_full()
            .border(px(1.5))
            .border_color(border)
            .flex_none()
            .grid()
            .items_center()
            .justify_center()
    };
    match status {
        Status::Done => ring(c(ACCENT))
            .bg(c(ACCENT))
            .child(icon("icons/status-check.svg", 9., c(ACCENT_FG)))
            .into_any_element(),
        Status::Canceled => ring(c(MUTED_FG))
            .child(icon("icons/status-x.svg", 9., c(MUTED_FG)))
            .into_any_element(),
        Status::Started => canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let cx = bounds.center().x;
                let cy = bounds.center().y;
                let r: f32 = 5.4;
                let mut pb = PathBuilder::fill();
                pb.move_to(point(cx, cy - px(r)));
                let n = 20;
                for i in 1..=n {
                    let a = -90.0f32.to_radians() + (i as f32 / n as f32) * 180.0f32.to_radians();
                    pb.line_to(point(cx + px(r * a.cos()), cy + px(r * a.sin())));
                }
                pb.close();
                let path = pb.build().unwrap();
                window.paint_path(path, c(WARN));
                // ring border
                let n2 = 40;
                let ro: f32 = 6.25;
                let ri: f32 = 4.75;
                for i in 0..n2 {
                    let a0 = (i as f32 / n2 as f32) * 360.0f32.to_radians();
                    let a1 = ((i + 1) as f32 / n2 as f32) * 360.0f32.to_radians();
                    let mut seg = PathBuilder::fill();
                    seg.move_to(point(cx + px(ro * a0.cos()), cy + px(ro * a0.sin())));
                    seg.line_to(point(cx + px(ro * a1.cos()), cy + px(ro * a1.sin())));
                    seg.line_to(point(cx + px(ri * a1.cos()), cy + px(ri * a1.sin())));
                    seg.line_to(point(cx + px(ri * a0.cos()), cy + px(ri * a0.sin())));
                    seg.close();
                    window.paint_path(seg.build().unwrap(), c(WARN));
                }
            },
        )
        .w(px(14.))
        .h(px(14.))
        .flex_none()
        .into_any_element(),
        Status::Deferred => canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let cx = bounds.center().x;
                let cy = bounds.center().y;
                let ro: f32 = 6.25;
                let ri: f32 = 4.75;
                let dashes = 8;
                for i in 0..dashes {
                    let a0 = (i as f32 / dashes as f32) * 360.0f32.to_radians() + 0.10;
                    let a1 = ((i + 1) as f32 / dashes as f32) * 360.0f32.to_radians() - 0.10;
                    let mut seg = PathBuilder::fill();
                    seg.move_to(point(cx + px(ro * a0.cos()), cy + px(ro * a0.sin())));
                    seg.line_to(point(cx + px(ro * a1.cos()), cy + px(ro * a1.sin())));
                    seg.line_to(point(cx + px(ri * a1.cos()), cy + px(ri * a1.sin())));
                    seg.line_to(point(cx + px(ri * a0.cos()), cy + px(ri * a0.sin())));
                    seg.close();
                    window.paint_path(seg.build().unwrap(), rgba(MUTED_FG, 0.6));
                }
            },
        )
        .w(px(14.))
        .h(px(14.))
        .flex_none()
        .into_any_element(),
        _ => ring(c(MUTED_FG)).into_any_element(),
    }
}

// ---------------------------------------------------------------------------
// PriorityBars — 4 bars, heights 4/7/10/13, w=2 rx=1 in a 16px box.
// muted-foreground fill; unfilled bars at 25% opacity.
// ---------------------------------------------------------------------------

pub fn priority_bars(priority: u8) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let heights = [4.0f32, 7.0, 10.0, 13.0];
            for (i, h) in heights.iter().enumerate() {
                let r: f32 = 1.0;
                let w: f32 = 2.0;
                let x0 = bounds.origin.x + px(1.5 + i as f32 * 3.5);
                let y0 = bounds.origin.y + px(15.0 - h);
                let mut pb = PathBuilder::fill();
                pb.move_to(point(x0 + px(r), y0));
                pb.line_to(point(x0 + px(w - r), y0));
                pb.arc_to(
                    point(px(r), px(r)),
                    px(0.),
                    false,
                    true,
                    point(x0 + px(w), y0 + px(r)),
                );
                pb.line_to(point(x0 + px(w), y0 + px(*h - r)));
                pb.arc_to(
                    point(px(r), px(r)),
                    px(0.),
                    false,
                    true,
                    point(x0 + px(w - r), y0 + px(*h)),
                );
                pb.line_to(point(x0 + px(r), y0 + px(*h)));
                pb.arc_to(
                    point(px(r), px(r)),
                    px(0.),
                    false,
                    true,
                    point(x0, y0 + px(*h - r)),
                );
                pb.line_to(point(x0, y0 + px(r)));
                pb.arc_to(
                    point(px(r), px(r)),
                    px(0.),
                    false,
                    true,
                    point(x0 + px(r), y0),
                );
                pb.close();
                let filled = (i as u8) < priority;
                window.paint_path(
                    pb.build().unwrap(),
                    rgba(MUTED_FG, if filled { 1.0 } else { 0.25 }),
                );
            }
        },
    )
    .w(px(16.))
    .h(px(16.))
    .flex_none()
}

