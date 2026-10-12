//! Cortex's diagnostics plaque. Callers provide checks; no network IO here.
use crate::{
    settings::{section_group, UiStyle, INSET},
    theme::{c, rgba, DESTRUCTIVE, SUCCESS},
};
use ::gpui::{prelude::*, AnimationExt, *};
use std::{rc::Rc, time::Duration};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticState {
    Ok,
    Pending,
    Fail,
}

pub struct DiagnosticCheck {
    pub label: SharedString,
    pub state: DiagnosticState,
}
impl DiagnosticCheck {
    pub fn new(label: impl Into<SharedString>, state: DiagnosticState) -> Self {
        Self {
            label: label.into(),
            state,
        }
    }
}

type ToggleHandler = Rc<dyn Fn(&mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct Diagnostics {
    id: ElementId,
    checks: Vec<DiagnosticCheck>,
    expanded: bool,
    style: UiStyle,
    on_toggle: Option<ToggleHandler>,
}
impl Diagnostics {
    pub fn new(id: impl Into<ElementId>, checks: Vec<DiagnosticCheck>) -> Self {
        Self {
            id: id.into(),
            checks,
            expanded: false,
            style: UiStyle::default(),
            on_toggle: None,
        }
    }
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }
    pub fn style(mut self, style: UiStyle) -> Self {
        self.style = style;
        self
    }
    pub fn on_toggle(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }
}

fn status_icon(path: &'static str, color: u32) -> gpui_component::Icon {
    gpui_component::Icon::default()
        .path(path)
        .size(px(18.))
        .text_color(c(color))
}

impl RenderOnce for Diagnostics {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focus = window.with_global_id((self.id.clone(), "focus").into(), |id, window| {
            window.with_element_state(id, |previous: Option<FocusHandle>, _| {
                let focus = previous.unwrap_or_else(|| cx.focus_handle());
                (focus.clone(), focus)
            })
        });
        let selector = self.id.to_string();
        let style = self.style;
        let has_failures = self
            .checks
            .iter()
            .any(|check| check.state == DiagnosticState::Fail);
        let pending = self
            .checks
            .iter()
            .any(|check| check.state == DiagnosticState::Pending);
        let interactive = has_failures && self.on_toggle.is_some();
        let click = self.on_toggle.clone();
        let keyboard = self.on_toggle;
        let mut body =
            style
                .settings_card()
                .id(self.id)
                .debug_selector(move || selector.clone())
                .child(
                    div()
                        .id("about-diagnostics-toggle")
                        .debug_selector(|| "about-diagnostics-toggle".into())
                        .w_full()
                        .rounded_t(px(12.))
                        .when(!self.expanded, |d| d.rounded_b(px(12.)))
                        .flex()
                        .items_center()
                        .gap_3()
                        .px(px(INSET))
                        .py(px(10.))
                        .child(status_icon(
                            if has_failures {
                                "icons/circle-x.svg"
                            } else {
                                "icons/circle-check.svg"
                            },
                            if has_failures {
                                DESTRUCTIVE()
                            } else if pending {
                                style.muted
                            } else {
                                SUCCESS()
                            },
                        ))
                        .child(div().flex_1().text_size(style.text_px(13.)).child(
                            if has_failures {
                                "Есть ошибки"
                            } else if pending {
                                "Проверка…"
                            } else {
                                "Всё в порядке"
                            },
                        ))
                        .when(has_failures, |d| {
                            d.child(
                                gpui_component::Icon::default()
                                    .path(if self.expanded {
                                        "icons/chevron-up.svg"
                                    } else {
                                        "icons/chevron-down.svg"
                                    })
                                    .size(px(16.))
                                    .text_color(c(style.muted)),
                            )
                        })
                        .when(interactive, |d| {
                            d.cursor_pointer()
                                .hover(move |s| s.bg(rgba(style.foreground, 0.05)))
                                .role(Role::Button)
                                .aria_label("Список проверок диагностики")
                                .aria_expanded(self.expanded)
                                .track_focus(&focus)
                                .tab_index(0)
                                .focus_visible(move |s| s.bg(rgba(style.foreground, 0.05)))
                                .on_click(move |_, window, cx| {
                                    window.focus(&focus, cx);
                                    if let Some(handler) = &click {
                                        handler(window, cx);
                                    }
                                })
                                .on_key_down(move |event: &KeyDownEvent, window, cx| {
                                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                        if let Some(handler) = &keyboard {
                                            handler(window, cx);
                                        }
                                        cx.stop_propagation();
                                    }
                                })
                        }),
                );
        if has_failures && self.expanded {
            let mut checks = self.checks;
            checks.sort_by_key(|check| check.state != DiagnosticState::Fail);
            let full_h = checks.len() as f32 * 34. + 1.;
            let mut list = div()
                .id("about-diagnostics-list")
                .debug_selector(|| "about-diagnostics-list".into())
                .w_full()
                .flex()
                .flex_col()
                .border_t_1()
                .border_color(rgba(style.border, 0.6));
            for (index, check) in checks.into_iter().enumerate() {
                let (path, color) = match check.state {
                    DiagnosticState::Ok => ("icons/circle-check.svg", SUCCESS()),
                    DiagnosticState::Pending => ("icons/circle-check.svg", style.muted),
                    DiagnosticState::Fail => ("icons/circle-x.svg", DESTRUCTIVE()),
                };
                list = list.child(
                    div()
                        .w_full()
                        .flex()
                        .items_center()
                        .gap_3()
                        .px(px(INSET))
                        .py(px(8.))
                        .when(index > 0, |row| {
                            row.border_t_1().border_color(rgba(style.border, 0.6))
                        })
                        .child(status_icon(path, color))
                        .child(div().text_size(style.text_px(13.)).child(check.label)),
                );
            }
            body = body.child(
                div()
                    .w_full()
                    .overflow_hidden()
                    .with_animation(
                        "about-diagnostics-reveal",
                        Animation::new(Duration::from_millis(220))
                            .with_easing(|t| 1. - (1. - t).powi(3)),
                        move |element, progress| {
                            element
                                .h(px(full_h * progress))
                                .opacity(0.4 + 0.6 * progress)
                        },
                    )
                    .child(list),
            );
        }
        section_group()
            .child(div().text_size(style.text_px(13.)).child("Диагностика"))
            .child(body)
    }
}
