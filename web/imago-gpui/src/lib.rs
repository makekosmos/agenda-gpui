//! wasm stand-in for `imago-gpui` — same module names and public API so the
//! app's `imago_gpui::…` paths compile unchanged. `theme`/`palettes`/
//! `settings`/`about`/`toggle` are copied from imago rev c7ae794 (that code
//! was itself seeded from this app's own sources and is wasm-clean apart
//! from `Instant`, which here comes from web-time). `appearance_editor` is a
//! compact demo editor — the Engine round-trip does not exist on wasm.
//! `assets` delegates to the fetch-backed `gpui::assets::Assets`, serving
//! `icons/*.svg` from the bundle's `/assets/` directory.
#![allow(non_snake_case)] // Imago's palette accessors are uppercase.

pub mod about;
pub mod appearance_editor;
pub mod assets;
pub mod palettes;
pub mod settings;
pub mod theme;
pub mod toggle;

pub use palettes::{ThemeDef, THEMES};
pub use theme::Palette;
