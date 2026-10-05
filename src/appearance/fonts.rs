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

pub(super) fn choose_font_family(
    requested: &str,
    installed: &[String],
    system: Option<&str>,
) -> String {
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
