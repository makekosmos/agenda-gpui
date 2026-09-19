#![allow(clippy::disallowed_methods, reason = "build scripts are exempt")]

fn main() {
    println!("cargo::rustc-check-cfg=cfg(gles)");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    if target_os == "windows" {
        #[cfg(feature = "windows-manifest")]
        embed_resource();
    }
}

// agenda-gpui patch: the `windows-manifest` feature is force-enabled by
// gpui-pre-platform, so it cannot be turned off from the outside. The app
// embeds its own windows/app.manifest (Common Controls v6 + PerMonitorV2,
// superset of gpui.manifest.xml) via its own build.rs; embedding a second
// RT_MANIFEST id=1 fails to link (CVTRES CVT1100 / LNK1123). Keep the
// feature valid but make the embed a no-op.
#[cfg(feature = "windows-manifest")]
fn embed_resource() {}
