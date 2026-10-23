use super::*;
use gpui::{linear_color_stop, linear_gradient};
use gpui_component::chart::AreaChart;

const DOT: f32 = 8.0;
const DOT_GAP: f32 = 5.0;
const WEEK_DOTS: usize = 5;
/// btop default graph stops: `cpu_start` / `cpu_mid` / `cpu_end`.
const BTOP_GREEN: u32 = 0x8ec07c;
const BTOP_YELLOW: u32 = 0xd79921;
const BTOP_RED: u32 = 0xfb4934;

fn week_monday(date: NaiveDate) -> NaiveDate {
    date - Duration::days(date.weekday().num_days_from_monday() as i64)
}

fn lerp_rgb(a: u32, b: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let ch = |shift: u32| {
        let av = ((a >> shift) & 0xff) as f32;
        let bv = ((b >> shift) & 0xff) as f32;
        (av + (bv - av) * t).round() as u32
    };
    (ch(16) << 16) | (ch(8) << 8) | ch(0)
}

/// Color of a lit circle by its place in the stack, bottom = 0. Green at the
/// base, yellow through the middle, red on the top two — btop's cpu gradient.
fn btop_color(from_bottom: usize) -> u32 {
    if from_bottom >= WEEK_DOTS - 2 {
        return BTOP_RED;
    }
    let t = from_bottom as f32 / (WEEK_DOTS - 3) as f32;
    if t < 0.5 {
        lerp_rgb(BTOP_GREEN, BTOP_YELLOW, t / 0.5)
    } else {
        lerp_rgb(BTOP_YELLOW, BTOP_RED, (t - 0.5) / 0.5)
    }
}

/// One column per week, ten circles each, edges flush with `width`. Circles
/// fill from the bottom by completed tasks that week (one circle, one task,
/// capped at ten). Unlit circles stay dim so the column does not disappear.
#[derive(Clone)]
struct WeekPoint {
    label: SharedString,
    planned: f64,
    done: f64,
}

fn range_len(range: crate::app::ActivityRange) -> (usize, bool) {
    match range {
        crate::app::ActivityRange::Week => (7, false),
        crate::app::ActivityRange::Month => (30, false),
        crate::app::ActivityRange::Year => (52, true),
    }
}

/// Fixed window for the selected range: 7 days, 30 days, or 52 weeks.
/// The count does not depend on when the first task appeared.
fn range_points(
    todos: &[Todo],
    today: NaiveDate,
    range: crate::app::ActivityRange,
) -> Vec<WeekPoint> {
    let (len, weekly) = range_len(range);
    let step = if weekly { 7 } else { 1 };
    let end = if weekly { week_monday(today) } else { today };
    let first = end - Duration::days(step * (len - 1) as i64);
    let mut planned = vec![0f64; len];
    let mut done = vec![0f64; len];
    for todo in todos {
        if todo.is_trashed {
            continue;
        }
        let planned_day = date_only(&todo.scheduled_date)
            .and_then(|k| parse_key(&k))
            .or_else(|| todo.is_today.then_some(today));
        if let Some(day) = planned_day {
            if let Some(index) = bucket_index(day, first, step, len) {
                planned[index] += 1.0;
            }
        }
        if let Some(day) = completed_day(todo) {
            if let Some(index) = bucket_index(day, first, step, len) {
                done[index] += 1.0;
            }
        }
    }
    (0..len)
        .map(|i| {
            let start = first + Duration::days(step * i as i64);
            WeekPoint {
                label: format!(
                    "{} {}",
                    start.day(),
                    MONTH_SHORT[(start.month() - 1) as usize]
                )
                .into(),
                planned: planned[i],
                done: done[i],
            }
        })
        .collect()
}

fn bucket_index(day: NaiveDate, first: NaiveDate, step: i64, len: usize) -> Option<usize> {
    let origin = if step == 7 { week_monday(day) } else { day };
    let index = (origin - first).num_days() / step;
    (0..len as i64).contains(&index).then_some(index as usize)
}

fn fade_to_baseline(color: u32, alpha: f32) -> gpui::Background {
    linear_gradient(
        180.,
        linear_color_stop(rgba(color, alpha), 0.),
        linear_color_stop(rgba(color, 0.), 1.),
    )
}

fn activity_area(points: Vec<WeekPoint>) -> impl IntoElement {
    let ticks = (points.len() / 8).max(1);
    AreaChart::new(points)
        .id("activity-area")
        .x(|point: &WeekPoint| point.label.clone())
        .y(|point: &WeekPoint| point.planned)
        .stroke(rgba(BTOP_YELLOW, 0.95))
        .fill(fade_to_baseline(BTOP_YELLOW, 0.4))
        .name("Запланировано")
        .linear()
        .y(|point: &WeekPoint| point.done)
        .stroke(rgba(BTOP_GREEN, 0.95))
        .fill(fade_to_baseline(BTOP_GREEN, 0.45))
        .name("Выполнено")
        .linear()
        .grid(true)
        .tick_margin(ticks)
}

/// One column per bucket of the selected range, always the same count,
/// spread edge to edge. Five circles, filled from the bottom by completed tasks.
fn activity_chart(points: &[WeekPoint]) -> AnyElement {
    let mut cols = div().w_full().flex().justify_between().items_end();
    for (i, point) in points.iter().enumerate() {
        let count = point.done.max(0.0) as usize;
        let shown = count.min(WEEK_DOTS);
        let mut col = div().flex().flex_col().gap(px(DOT_GAP)).items_center();
        for row in 0..WEEK_DOTS {
            let from_bottom = WEEK_DOTS - 1 - row;
            let lit = from_bottom < shown;
            col = col.child(div().size(px(DOT)).flex_none().rounded_full().bg(if lit {
                rgba(btop_color(from_bottom), 0.95)
            } else {
                fg_mix(0.12)
            }));
        }
        let caption = format!("{} выполнено, {}", count, point.label);
        cols = cols.child(
            div()
                .id(SharedString::from(format!("act-{i}")))
                .flex_none()
                .tooltip(text_tooltip(caption))
                .child(col),
        );
    }
    cols.into_any_element()
}

impl Agenda {
    pub(crate) fn nsh_activity(&self) -> Vec<serde_json::Value> {
        range_points(&self.todos, Local::now().date_naive(), self.activity_range)
            .into_iter()
            .map(|point| serde_json::json!({"label": point.label.to_string(), "done": point.done}))
            .collect()
    }

    pub(crate) fn statistics_page(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> AnyElement {
        let today = Local::now().date_naive();
        let points = range_points(&self.todos, today, self.activity_range);
        let chart = activity_chart(&points);
        let area = activity_area(points);

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
                            .max_w(px(720.))
                            .mx_auto()
                            .px_7()
                            .pt_4()
                            .pb_8()
                            .flex()
                            .flex_col()
                            .gap(px(28.))
                            .child(chart)
                            .child(div().h(px(220.)).w_full().child(area)),
                    ),
            )
            .into_any_element()
    }
}
