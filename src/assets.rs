use std::borrow::Cow;

pub fn font_bytes() -> Vec<Cow<'static, [u8]>> {
    vec![
        Cow::Borrowed(include_bytes!("../assets/fonts/Inter.ttf") as &[u8]),
        Cow::Borrowed(include_bytes!("../assets/fonts/IBMPlexMono-Regular.ttf") as &[u8]),
        Cow::Borrowed(include_bytes!("../assets/fonts/IBMPlexMono-Medium.ttf") as &[u8]),
        Cow::Borrowed(include_bytes!("../assets/fonts/IBMPlexMono-SemiBold.ttf") as &[u8]),
    ]
}
