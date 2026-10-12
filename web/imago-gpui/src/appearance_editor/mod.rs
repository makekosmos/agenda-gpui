//! Compact appearance editor for the wasm build. Same host contract as
//! imago-gpui's `AppearanceEditor` (`new`/`configure`/`status`/`ingest`/
//! `reject`, `on_patch` callback), but a reduced control set: mode tiles,
//! a theme list and accent swatches. There is no Engine round-trip on wasm —
//! the host folds each patch into its in-memory snapshot.
use crate::palettes::THEMES;
use crate::settings::UiStyle;
use crate::theme::{c, rgba};
use ::gpui::{prelude::*, *};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::rc::Rc;

/// Engine-shaped settings snapshot, same field set as upstream.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct AppearanceSettings {
    schema_version: u32,
    mode: String,
    light_theme: String,
    dark_theme: String,
    accent_source: String,
    accent_color: Option<String>,
    follow_apps: bool,
    material: String,
    font_family: String,
    font_size: f32,
    revision: u64,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            schema_version: 1,
            mode: "dark".into(),
            light_theme: "default".into(),
            dark_theme: "default".into(),
            accent_source: "theme".into(),
            accent_color: None,
            follow_apps: false,
            material: "opaque".into(),
            font_family: "Inter".into(),
            font_size: 13.,
            revision: 0,
        }
    }
}

type OnPatch = Rc<dyn for<'a> Fn(Value, &mut Context<'a, AppearanceEditor>)>;

const ACCENTS: &[Option<&str>] = &[
    None,
    Some("#548bd0"),
    Some("#3db07c"),
    Some("#d9975c"),
    Some("#e66f64"),
    Some("#a074d0"),
    Some("#d981ac"),
];

pub struct AppearanceEditor {
    on_patch: OnPatch,
    style: UiStyle,
    editable: bool,
    status: Option<SharedString>,
    ready: bool,
    accepted: AppearanceSettings,
    settings: AppearanceSettings,
}

impl AppearanceEditor {
    pub fn new(
        _cx: &mut Context<Self>,
        on_patch: impl Fn(Value, &mut Context<Self>) + 'static,
    ) -> Self {
        Self {
            on_patch: Rc::new(on_patch),
            style: UiStyle::default(),
            editable: true,
            status: None,
            ready: false,
            accepted: AppearanceSettings::default(),
            settings: AppearanceSettings::default(),
        }
    }

    pub fn configure(&mut self, style: UiStyle, _show_follow_apps: bool, editable: bool) {
        self.style = style;
        self.editable = editable;
    }

    pub fn status(&mut self, status: impl Into<SharedString>) {
        let status = status.into();
        self.status = (!status.is_empty()).then_some(status);
    }

    /// Host snapshot shaped like `appearance.get`: `{"settings": {...}, …}`.
    pub fn ingest(&mut self, response: &Value) -> bool {
        let Ok(settings) =
            serde_json::from_value::<AppearanceSettings>(response["settings"].clone())
        else {
            return false;
        };
        if settings.schema_version != 1
            || !settings.font_size.is_finite()
            || !(11. ..=18.).contains(&settings.font_size)
        {
            return false;
        }
        self.accepted = settings.clone();
        self.settings = settings;
        self.ready = true;
        true
    }

    pub fn reject(&mut self) {
        self.settings = self.accepted.clone();
    }

    fn patch(&mut self, params: Value, cx: &mut Context<Self>) {
        if !(self.ready && self.editable) {
            return;
        }
        if let Some(patch) = params.as_object() {
            if let Ok(mut value) = serde_json::to_value(&self.settings) {
                if let Some(settings) = value.as_object_mut() {
                    for (key, item) in patch {
                        settings.insert(key.clone(), item.clone());
                    }
                }
                if let Ok(next) = serde_json::from_value::<AppearanceSettings>(value) {
                    self.settings = next;
                    let on_patch = self.on_patch.clone();
                    on_patch(params, cx);
                    cx.notify();
                }
            }
        }
    }
}

mod rows;
impl Render for AppearanceEditor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let style = self.style;
        let mut col = div().flex().flex_col().gap_4();
        if let Some(status) = &self.status {
            col = col.child(
                div()
                    .px_3()
                    .py_2()
                    .rounded(px(8.))
                    .bg(rgba(style.surface, 0.8))
                    .text_size(style.text_px(12.5))
                    .text_color(c(style.muted))
                    .child(status.clone()),
            );
        }
        col.child(
            style
                .section("Режим", "Светлое, тёмное или по системе оформление.")
                .child(self.mode_row(cx)),
        )
        .child(
            style
                .section("Тема", "Палитра применяется сразу — без перезапуска.")
                .child(self.theme_rows(cx)),
        )
        .child(style.section("Акцент", "").child(self.accent_row(cx)))
    }
}
