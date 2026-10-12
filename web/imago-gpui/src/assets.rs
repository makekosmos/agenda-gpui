//! wasm: upstream imago-gpui embeds its icon table via `include_bytes!`; the
//! same set is bundled on the page as `/assets/icons/*.svg` and served by
//! gpui-kit-assets' fetch-backed `Assets` (default endpoint = same origin).
use gpui::{AssetSource, Result, SharedString};
use std::borrow::Cow;

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        gpui::assets::Assets::default().load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        gpui::assets::Assets::default().list(path)
    }
}
