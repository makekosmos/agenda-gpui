// Embedded assets: icons + fonts served through gpui::AssetSource.
use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

macro_rules! embed {
    ($($path:literal),* $(,)?) => {
        fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
            match path {
                $( $path => Ok(Some(Cow::Borrowed(include_bytes!(concat!("../assets/", $path)) as &[u8]))), )*
                _ => Ok(None),
            }
        }
        fn list(&self, _path: &str) -> Result<Vec<SharedString>> {
            Ok(vec![$( SharedString::new_static($path) ),*])
        }
    };
}

pub struct Assets;

impl AssetSource for Assets {
    embed!(
        "icons/analytics-01.svg",
        "icons/arrow-left.svg",
        "icons/book-open.svg",
        "icons/calendar-01.svg",
        "icons/calendar-02.svg",
        "icons/check.svg",
        "icons/clock-01.svg",
        "icons/delete.svg",
        "icons/dollar.svg",
        "icons/folder.svg",
        "icons/folder-closed.svg",
        "icons/folder-open.svg",
        "icons/folder-open2.svg",
        "icons/help-circle.svg",
        "icons/inbox.svg",
        "icons/more-circle.svg",
        "icons/more-h.svg",
        "icons/plus.svg",
        "icons/repeat.svg",
        "icons/search.svg",
        "icons/settings.svg",
        "icons/sidebar-left.svg",
        "icons/sliders.svg",
        "icons/star.svg",
        "icons/status-check.svg",
        "icons/status-x.svg",
        "icons/tag.svg",
        "icons/task-01.svg",
    );
}

pub fn font_bytes() -> Vec<Cow<'static, [u8]>> {
    vec![
        Cow::Borrowed(include_bytes!("../assets/fonts/Inter.ttf") as &[u8]),
        Cow::Borrowed(include_bytes!("../assets/fonts/IBMPlexMono-Regular.ttf") as &[u8]),
        Cow::Borrowed(include_bytes!("../assets/fonts/IBMPlexMono-Medium.ttf") as &[u8]),
        Cow::Borrowed(include_bytes!("../assets/fonts/IBMPlexMono-SemiBold.ttf") as &[u8]),
    ]
}
