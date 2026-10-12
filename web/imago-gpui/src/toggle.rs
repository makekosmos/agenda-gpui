//! Shared settings switch: Cortex geometry, state marks, travel and native feedback.
//!
//! Interaction, focus, disabled semantics and accessibility are delegated to
//! `gpui_base::Switch`. Glass is opt-in; colors otherwise follow Imago's theme.
use crate::theme::{self, mix, rgba};
use ::gpui::{prelude::*, *};
use gpui_component::Disableable;
use std::rc::Rc;
use web_time::Instant;

const WIDTH: f32 = 44.8;
const HEIGHT: f32 = 28.8;
const TRACK: f32 = 20.8;
const INSET: f32 = 1.6;
const THUMB: f32 = 24.0;

/// Optional RGB palette override for an app-local accent or surface.
///
/// Omit `.colors(...)` to resolve these values from Imago at render time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToggleColors {
    pub accent: u32,
    pub foreground: u32,
    pub surface: u32,
}

impl Default for ToggleColors {
    fn default() -> Self {
        Self {
            accent: theme::ACCENT(),
            foreground: theme::FG(),
            surface: theme::CARD(),
        }
    }
}

/// Controlled switch with Cortex's fixed 44.8 × 28.8 geometry.
///
/// The caller owns the checked state and updates it in the change callback.
type ChangeHandler = Rc<dyn Fn(bool, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct Toggle {
    id: &'static str,
    checked: bool,
    disabled: bool,
    label: Option<SharedString>,
    glass: bool,
    colors: Option<ToggleColors>,
    on_change: ChangeHandler,
}

pub fn toggle<T: 'static>(
    id: &'static str,
    checked: bool,
    cx: &mut Context<T>,
    on_click: impl Fn(&mut T, bool, &mut Context<T>) + 'static,
) -> Toggle {
    let listener = cx.listener(move |this, next: &bool, _, cx| {
        on_click(this, *next, cx);
        cx.notify();
    });
    Toggle::new(id, checked).on_change(move |next, window, cx| listener(&next, window, cx))
}

impl Toggle {
    /// Create a controlled switch; without a callback activation has no effect
    /// on the caller's state.
    pub fn new(id: &'static str, checked: bool) -> Self {
        Self {
            id,
            checked,
            disabled: false,
            label: None,
            glass: false,
            colors: None,
            on_change: Rc::new(|_, _, _| {}),
        }
    }

    /// Called once per enabled activation, after native selection feedback.
    pub fn on_change(mut self, on_change: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Rc::new(on_change);
        self
    }

    /// Opt into Cortex's glass surface recipe (no window material changes).
    pub fn glass(mut self, glass: bool) -> Self {
        self.glass = glass;
        self
    }

    /// Override palette inputs while retaining Imago's light/dark mode recipe.
    pub fn colors(mut self, colors: ToggleColors) -> Self {
        self.colors = Some(colors);
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
}
impl Disableable for Toggle {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

fn selection_step() {}

struct Travel {
    from: f32,
    target: f32,
    started: Instant,
}
impl Travel {
    fn value(&self, now: Instant) -> f32 {
        let t = (now.duration_since(self.started).as_secs_f32() / 0.18).min(1.0);
        self.from + (self.target - self.from) * (1.0 - (1.0 - t).powi(3))
    }
}

fn surface_tones(base: Hsla, thumb: bool, glass: bool) -> (Hsla, Hsla) {
    if !glass {
        return (base, base);
    }
    let (light, shade) = if thumb { (0.12, 0.07) } else { (0.07, 0.09) };
    (
        base.blend(white().opacity(light)),
        base.blend(black().opacity(shade)),
    )
}

impl RenderOnce for Toggle {
    fn render(self, window: &mut Window, _: &mut App) -> impl IntoElement {
        let now = Instant::now();
        let target = if self.checked { 1.0 } else { 0.0 };
        let position =
            window.with_global_id((ElementId::from(self.id), "travel").into(), |id, window| {
                window.with_element_state(id, |previous: Option<Travel>, _| {
                    let mut travel = previous.unwrap_or(Travel {
                        from: target,
                        target,
                        started: now,
                    });
                    if travel.target != target {
                        travel = Travel {
                            from: travel.value(now),
                            target,
                            started: now,
                        };
                    }
                    (travel.value(now), travel)
                })
            });
        if (position - target).abs() > 0.001 {
            window.request_animation_frame();
        }
        let dark = theme::is_dark();
        let colors = self.colors.unwrap_or_default();
        let surface = colors.surface;
        let track = if self.checked {
            if dark {
                mix(0, 0.14, colors.accent)
            } else {
                mix(0xffffff, 0.10, colors.accent)
            }
        } else {
            let opacity = match (dark, self.glass) {
                (true, true) => 0.22,
                (true, false) => 0.18,
                (false, true) => 0.12,
                (false, false) => 0.10,
            };
            mix(colors.foreground, opacity, surface)
        };
        let thumb = mix(
            0xffffff,
            match (dark, self.glass) {
                (true, true) => 0.94,
                (true, false) | (false, true) => 0.96,
                (false, false) => 1.0,
            },
            surface,
        );
        let (track_light, track_shade) = surface_tones(track, false, self.glass);
        let (thumb_light, thumb_shade) = surface_tones(thumb, true, self.glass);
        let mark_padding = (WIDTH - THUMB - INSET - 7.2) / 2.0;
        let visual = div()
            .relative()
            .w(px(WIDTH))
            .h(px(HEIGHT))
            .child(
                div()
                    .absolute()
                    .top(px((HEIGHT - TRACK) / 2.0))
                    .left_0()
                    .w(px(WIDTH))
                    .h(px(TRACK))
                    .rounded_full()
                    .bg(linear_gradient(
                        180.,
                        linear_color_stop(track_light, 0.),
                        linear_color_stop(track_shade, 1.),
                    ))
                    .border_1()
                    .border_color(if self.checked {
                        track.blend(white().opacity(if self.glass { 0.16 } else { 0.12 }))
                    } else {
                        track.blend(rgba(colors.foreground, 0.08))
                    })
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .px(px(mark_padding))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .size(px(7.2))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .opacity(position)
                                    .child(
                                        div()
                                            .w(px(1.2))
                                            .h(px(7.2))
                                            .rounded_full()
                                            .bg(white().opacity(0.96)),
                                    ),
                            )
                            .child(
                                div()
                                    .size(px(7.2))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .opacity(1.0 - position)
                                    .child(
                                        div()
                                            .size(px(6.4))
                                            .rounded_full()
                                            .border_1()
                                            .border_color(white().opacity(0.92)),
                                    ),
                            ),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .top(px((HEIGHT - (TRACK - 2.0 * INSET)) / 2.0))
                    .left(px(INSET + (WIDTH - THUMB - 2.0 * INSET) * position))
                    .w(px(THUMB))
                    .h(px(TRACK - 2.0 * INSET))
                    .rounded_full()
                    .bg(linear_gradient(
                        180.,
                        linear_color_stop(thumb_light, 0.),
                        linear_color_stop(thumb_shade, 1.),
                    ))
                    .border_1()
                    .border_color(thumb.blend(black().opacity(if dark { 0.10 } else { 0.08 })))
                    .when(self.glass && position > 0.001, |el| {
                        el.child(
                            div()
                                .absolute()
                                .top(px(1.6))
                                .left(px(7.2))
                                .w(px(9.6))
                                .h(px(1.))
                                .opacity(position)
                                .rounded_full()
                                .bg(thumb_light.blend(white().opacity(0.45))),
                        )
                    }),
            );
        gpui_base::Switch::new(self.id)
            .checked(self.checked)
            .disabled(self.disabled)
            .cursor_pointer()
            .styles(|styles| styles.disabled(|style| style.cursor_not_allowed()))
            .flex_none()
            .when_some(self.label, |el, label| el.accessibility_label(label))
            .on_change(move |next, _, window, cx| {
                selection_step();
                (self.on_change)(next, window, cx);
            })
            .child(visual)
    }
}
