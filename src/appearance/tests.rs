use super::*;
fn sample() -> serde_json::Value {
    json!({"settings":{"schema_version":1,"mode":"system","light_theme":"default","dark_theme":"claude","accent_source":"custom","accent_color":"#123ABC","follow_apps":true,"material":"frosted","font_family":"Inter","font_size":13,"revision":3},"capabilities":{"materials":["default","frosted"],"wallpaper_accent":true},"wallpaper_accent":"#DEAD00"})
}
#[test]
fn parses_and_resolves_modes_accents_material() {
    let r = Response::parse(sample()).unwrap();
    assert_eq!(r.theme_index(false), 0);
    assert_eq!(crate::palettes::THEMES[r.theme_index(true)].key, "claude");
    assert_eq!(r.accent(), Some(0x123abc));
    assert_eq!(r.material(), 1);
}
#[test]
fn missing_font_uses_resolved_system_family() {
    let installed = vec!["Inter".into(), "Arial".into()];
    assert_eq!(
        choose_font_family("arial", &installed, Some("Arial")),
        "Arial"
    );
    assert_eq!(
        choose_font_family("Unknown", &installed, Some("Arial")),
        "Arial"
    );
    assert_eq!(
        choose_font_family("system", &installed, Some("Arial")),
        "Arial"
    );
    assert_eq!(choose_font_family("Unknown", &installed, None), "Inter");
}
#[test]
fn omitted_optional_values_use_contract_defaults() {
    let mut value = sample();
    let settings = value["settings"].as_object_mut().unwrap();
    settings.remove("font_size");
    settings.remove("accent_color");
    value.as_object_mut().unwrap().remove("wallpaper_accent");
    let parsed = Response::parse(value).unwrap();
    assert_eq!(parsed.settings.font_size, 13.0);
    assert_eq!(parsed.accent(), None);
}
#[test]
fn rejects_invalid_contract_and_falls_back_for_missing_wallpaper_accent() {
    let mut v = sample();
    v["settings"]["font_size"] = json!(19);
    assert!(Response::parse(v).is_err());
    let mut v = sample();
    v["settings"]["font_size"] = json!(12.5);
    assert_eq!(Response::parse(v).unwrap().settings.font_size, 12.5);
    let mut v = sample();
    v["settings"]["light_theme"] = json!("unknown");
    assert!(Response::parse(v).is_err());
    let mut v = sample();
    v["settings"]["accent_color"] = json!("red");
    assert!(Response::parse(v).is_err());
    let mut v = sample();
    v["settings"]["accent_source"] = json!("wallpaper");
    v["wallpaper_accent"] = serde_json::Value::Null;
    assert_eq!(Response::parse(v).unwrap().accent(), None);
}

#[test]
fn default_material_is_opaque_and_wallpaper_accent_arrives_later() {
    let mut value = sample();
    value["settings"]["material"] = json!("default");
    value["settings"]["accent_source"] = json!("wallpaper");
    value["wallpaper_accent"] = serde_json::Value::Null;
    let pending = Response::parse(value.clone()).unwrap();
    assert_eq!(pending.material(), 0);
    assert_eq!(pending.accent(), None);
    value["wallpaper_accent"] = json!("#DEAD00");
    let sampled = Response::parse(value).unwrap();
    assert_ne!(pending, sampled);
    assert_eq!(sampled.accent(), Some(0xdead00));
}

#[test]
fn poller_publishes_only_good_responses_and_survives_failed_fetch() {
    use std::sync::atomic::AtomicUsize;
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let poller = Poller::start_with(Duration::from_millis(2), move || {
        let n = count.fetch_add(1, Ordering::Relaxed);
        if n == 1 {
            None
        } else {
            let mut value = sample();
            value["settings"]["revision"] = json!(n);
            Response::parse(value).ok()
        }
    });
    let first = poller.replies.recv_timeout(Duration::from_secs(1)).unwrap();
    let second = poller.replies.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(first.settings.revision, 0);
    assert_eq!(second.settings.revision, 2);
}
