use gpui::Context;
use serde_json::{json, Value};

use super::Agenda;
use crate::appearance::{Mode, Response, Writer};

impl Agenda {
    pub(crate) fn start_appearance(&mut self, cx: &mut Context<Self>) {
        if self.demo {
            return;
        }
        self.appearance_poller = Some(crate::appearance::Poller::start());
        cx.spawn(async move |this, cx| loop {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(200))
                .await;
            if this
                .update(cx, |this, cx| {
                    let latest = this.appearance_poller.as_ref().and_then(|poller| {
                        let mut last = None;
                        while let Ok(reply) = poller.replies.try_recv() {
                            last = Some(reply);
                        }
                        last
                    });
                    if let Some(reply) = latest {
                        if let Ok(response) = Response::parse(&reply) {
                            this.receive_appearance(response);
                            cx.notify();
                        }
                    }
                })
                .is_err()
            {
                break;
            }
        })
        .detach();
    }

    fn receive_appearance(&mut self, response: Response) {
        self.theme_sel = match response.settings.mode {
            Mode::Light => 0,
            Mode::Dark => 1,
            Mode::System => 2,
        };
        self.appearance = Some(response);
    }

    pub(crate) fn appearance_editable(&self) -> bool {
        self.appearance
            .as_ref()
            .is_some_and(|response| response.policy.editable)
    }

    pub(crate) fn persist_appearance_patch(&mut self, patch: Value, cx: &mut Context<Self>) {
        if !self.appearance_editable() {
            return;
        }
        if let Some(mut value) = self
            .appearance
            .as_ref()
            .and_then(|r| serde_json::to_value(r).ok())
        {
            if let (Some(settings), Some(patch)) = (
                value.get_mut("settings").and_then(Value::as_object_mut),
                patch.as_object(),
            ) {
                for (key, item) in patch {
                    settings.insert(key.clone(), item.clone());
                }
            }
            if let Ok(response) = Response::parse(&value) {
                self.receive_appearance(response);
            }
        }
        if !self.demo {
            self.appearance_writer
                .get_or_insert_with(Writer::start)
                .set(patch);
        }
        cx.notify();
    }

    pub(crate) fn demo_appearance() -> Response {
        Response::parse(&json!({
            "settings": {
                "schema_version": 1,
                "mode": "dark",
                "light_theme": "default",
                "dark_theme": "default",
                "accent_source": "theme",
                "accent_color": null,
                "material": "default",
                "font_family": "Inter",
                "font_size": 13,
                "revision": 0
            },
            "policy": { "following": false, "editable": true },
            "capabilities": { "materials": ["default", "frosted", "mica"], "wallpaper_accent": false },
            "wallpaper_accent": null
        }))
        .expect("demo appearance matches the Engine schema")
    }
}
