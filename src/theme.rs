// Re-exported from imago-gpui (KOS-132): the shared Kosmos GPUI visual core
// was seeded from this file, so the API is a strict superset — `pal()`,
// `BG()`/`FG()`/…, `c`/`rgba`/`mix`/`lerp`, tag colors and easings are
// byte-identical. `imago_gpui::theme::apply` additionally installs the
// palette as the gpui-component theme.
pub use imago_gpui::theme::*;
