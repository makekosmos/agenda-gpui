//! Read-only consumer of Engine's shared appearance settings.
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::time::Duration;

use crate::store::Engine;
mod local;
pub use local::{path as local_path, LocalAppearance, LocalWriter};

/// Resolve a requested family once when it changes. Engine may name a font
/// installed on another machine; keep GPUI on an installed system fallback.
pub fn resolve_font_family(requested: &str, cx: &mut gpui::App) -> String {
    if requested == "Inter" {
        return "Inter".into();
    }
    let text_system = cx.text_system();
    let names = text_system.all_font_names();
    if names.is_empty() {
        return "Inter".into();
    }
    if !matches!(
        requested.to_ascii_lowercase().as_str(),
        "system" | ".systemuifont"
    ) {
        if let Some(name) = names
            .iter()
            .find(|name| name.eq_ignore_ascii_case(requested))
        {
            return name.clone();
        }
    }
    let system = text_system
        .get_font_for_id(text_system.resolve_font(&gpui::font(".SystemUIFont")))
        .map(|font| font.family.to_string());
    choose_font_family(requested, &names, system.as_deref())
}

fn choose_font_family(requested: &str, installed: &[String], system: Option<&str>) -> String {
    if !matches!(
        requested.to_ascii_lowercase().as_str(),
        "system" | ".systemuifont"
    ) {
        if let Some(name) = installed
            .iter()
            .find(|name| name.eq_ignore_ascii_case(requested))
        {
            return name.clone();
        }
    }
    system
        .and_then(|family| {
            installed
                .iter()
                .find(|name| name.eq_ignore_ascii_case(family))
        })
        .or_else(|| {
            installed
                .iter()
                .find(|name| name.eq_ignore_ascii_case("Inter"))
        })
        .cloned()
        .unwrap_or_else(|| "Inter".into())
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    System,
    Light,
    Dark,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccentSource {
    Theme,
    Custom,
    Wallpaper,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Material {
    Default,
    Frosted,
    Opaque,
    Acrylic,
    Mica,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Settings {
    pub schema_version: u32,
    pub mode: Mode,
    pub light_theme: String,
    pub dark_theme: String,
    pub accent_source: AccentSource,
    #[serde(default)]
    pub accent_color: Option<String>,
    pub follow_apps: bool,
    pub material: Material,
    pub font_family: String,
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Response {
    pub settings: Settings,
    pub capabilities: Capabilities,
    #[serde(default)]
    pub wallpaper_accent: Option<String>,
}

fn default_font_size() -> f32 {
    13.0
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Capabilities {
    pub materials: Vec<Material>,
    pub wallpaper_accent: bool,
}

fn hex_color(value: &str) -> Option<u32> {
    let bytes = value.as_bytes();
    (bytes.len() == 7 && bytes[0] == b'#' && bytes[1..].iter().all(u8::is_ascii_hexdigit))
        .then(|| u32::from_str_radix(&value[1..], 16).ok())
        .flatten()
}

impl Response {
    pub fn parse(value: serde_json::Value) -> Result<Self, String> {
        let response: Self = serde_json::from_value(value).map_err(|e| e.to_string())?;
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
pub struct Poller {
    pub replies: Receiver<Response>,
    stop: Arc<AtomicBool>,
}

impl Poller {
    pub fn start() -> Self {
        let engine = Engine::with_timeout(Duration::from_millis(700));
        Self::start_with(Duration::from_secs(2), move || {
            engine
                .rpc("appearance.get", json!({}))
                .ok()
                .and_then(|value| Response::parse(value).ok())
        })
    }

    fn start_with(
        interval: Duration,
        mut fetch: impl FnMut() -> Option<Response> + Send + 'static,
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

#[cfg(test)]
mod tests;
