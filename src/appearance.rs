//! Consumer of Engine's per-app appearance settings. All state lives in the
//! Engine data dir — the app holds no local appearance file; `get`/`set` are
//! scoped to `agenda-gpui` via `app_id`.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;

use crate::store::Engine;

pub const APP_ID: &str = "agenda-gpui";

mod fonts;
#[cfg(test)]
use fonts::choose_font_family;
pub use fonts::resolve_font_family;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    System,
    Light,
    Dark,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AccentSource {
    Theme,
    Custom,
    Wallpaper,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Material {
    Default,
    Frosted,
    Opaque,
    Acrylic,
    Mica,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Settings {
    pub schema_version: u32,
    pub mode: Mode,
    pub light_theme: String,
    pub dark_theme: String,
    pub accent_source: AccentSource,
    #[serde(default)]
    pub accent_color: Option<String>,
    /// Only meaningful on the global (Manager) settings shape; scoped replies
    /// omit it and the field stays `false`.
    #[serde(default)]
    pub follow_apps: bool,
    pub material: Material,
    pub font_family: String,
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    pub revision: u64,
}

/// Engine's write policy for this app. `following` mirrors the global
/// `follow_apps` toggle: while it is on, `editable` is false and every scoped
/// write is rejected (the app's saved style is left untouched until follow
/// turns back off).
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct Policy {
    #[serde(default)]
    pub following: bool,
    #[serde(default = "default_editable")]
    pub editable: bool,
    #[serde(default)]
    pub can_set_follow_apps: bool,
}

fn default_editable() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Response {
    pub settings: Settings,
    #[serde(default)]
    pub policy: Policy,
    pub capabilities: Capabilities,
    #[serde(default)]
    pub wallpaper_accent: Option<String>,
    #[serde(default)]
    pub wallpaper_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Capabilities {
    pub materials: Vec<Material>,
    pub wallpaper_accent: bool,
}

fn default_font_size() -> f32 {
    13.0
}

fn hex_color(value: &str) -> Option<u32> {
    let bytes = value.as_bytes();
    (bytes.len() == 7 && bytes[0] == b'#' && bytes[1..].iter().all(u8::is_ascii_hexdigit))
        .then(|| u32::from_str_radix(&value[1..], 16).ok())
        .flatten()
}

impl Response {
    pub fn parse(value: &serde_json::Value) -> Result<Self, String> {
        let response: Self = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
        let s = &response.settings;
        if s.schema_version != 1
            || !s.font_size.is_finite()
            || !(11.0..=18.0).contains(&s.font_size)
            || s.font_family.trim().is_empty()
        {
            return Err("unsupported appearance settings".into());
        }
        for id in [&s.light_theme, &s.dark_theme] {
            if !crate::palettes::THEMES.iter().any(|def| def.key == id) {
                return Err(format!("unknown appearance theme: {id}"));
            }
        }
        if s.accent_color
            .as_deref()
            .is_some_and(|v| hex_color(v).is_none())
            || response
                .wallpaper_accent
                .as_deref()
                .is_some_and(|v| hex_color(v).is_none())
        {
            return Err("invalid appearance accent".into());
        }
        Ok(response)
    }

    pub fn theme_index(&self, dark: bool) -> usize {
        let id = if dark {
            &self.settings.dark_theme
        } else {
            &self.settings.light_theme
        };
        crate::palettes::THEMES
            .iter()
            .position(|def| def.key == id)
            .unwrap_or(0)
    }

    pub fn accent(&self) -> Option<u32> {
        match self.settings.accent_source {
            AccentSource::Theme => None,
            AccentSource::Custom => self.settings.accent_color.as_deref().and_then(hex_color),
            AccentSource::Wallpaper if self.capabilities.wallpaper_accent => {
                self.wallpaper_accent.as_deref().and_then(hex_color)
            }
            AccentSource::Wallpaper => None,
        }
    }

    pub fn material(&self) -> u8 {
        if !self
            .capabilities
            .materials
            .contains(&self.settings.material)
        {
            return 0;
        }
        match self.settings.material {
            Material::Acrylic | Material::Frosted => 1,
            Material::Mica => 2,
            Material::Default | Material::Opaque => 0,
        }
    }
}

/// The poller owns the authenticated transport and never runs on the UI thread.
/// Replies carry the raw scoped `appearance.get` payload so the shared editor
/// can ingest it unchanged.
pub struct Poller {
    pub replies: Receiver<Value>,
    stop: Arc<AtomicBool>,
}

impl Poller {
    pub fn start() -> Self {
        let engine = Engine::with_timeout(Duration::from_millis(700));
        Self::start_with(Duration::from_secs(2), move || {
            engine
                .rpc("appearance.get", json!({"app_id": APP_ID}))
                .ok()
                .filter(|value| Response::parse(value).is_ok())
        })
    }

    fn start_with(
        interval: Duration,
        mut fetch: impl FnMut() -> Option<Value> + Send + 'static,
    ) -> Self {
        let (sender, replies) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        std::thread::spawn(move || {
            while !thread_stop.load(Ordering::Relaxed) {
                if let Some(response) = fetch() {
                    if sender.send(response).is_err() {
                        break;
                    }
                }
                std::thread::sleep(interval);
            }
        });
        Self { replies, stop }
    }
}

impl Drop for Poller {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Coalescing scoped `appearance.set` writer: rapid edits fold into one
/// merged patch per flush, mirroring the old local writer's debounce. Replies
/// are ignored — the poller refreshes the authoritative snapshot.
pub struct Writer {
    sender: Sender<Value>,
    stop: Arc<AtomicBool>,
}

impl Writer {
    pub fn start() -> Self {
        let (sender, receiver) = mpsc::channel::<Value>();
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        std::thread::spawn(move || {
            let engine = Engine::with_timeout(Duration::from_millis(1500));
            while !thread_stop.load(Ordering::Relaxed) {
                let Ok(first) = receiver.recv_timeout(Duration::from_millis(250)) else {
                    continue;
                };
                let mut merged = first.as_object().cloned().unwrap_or_default();
                while let Ok(next) = receiver.try_recv() {
                    if let Some(object) = next.as_object() {
                        for (key, item) in object {
                            merged.insert(key.clone(), item.clone());
                        }
                    }
                }
                let params = json!({"app_id": APP_ID, "patch": Value::Object(merged)});
                if let Err(error) = engine.rpc("appearance.set", params) {
                    eprintln!("[appearance] scoped set failed: {error:?}");
                }
            }
        });
        Self { sender, stop }
    }

    pub fn set(&self, patch: Value) {
        let _ = self.sender.send(patch);
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests;
