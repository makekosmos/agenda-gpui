//! Embed a Windows application manifest so the loader activates Common
//! Controls v6 (required for TaskDialogIndirect from comctl32).
//! Cross-compile path: x86_64-w64-mingw32-windres → COFF .o → rustc link-arg.

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let target = env::var("TARGET").unwrap_or_default();
    if !target.contains("windows") {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let windows_dir = manifest_dir.join("windows");
    let manifest = windows_dir.join("app.manifest");
    let rc = windows_dir.join("app.rc");
    println!("cargo:rerun-if-changed={}", manifest.display());
    println!("cargo:rerun-if-changed={}", rc.display());

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let obj = out_dir.join("app_manifest.o");

    let windres = if target.starts_with("x86_64-pc-windows-gnu")
        || target.starts_with("x86_64-w64-windows-gnu")
    {
        "x86_64-w64-mingw32-windres"
    } else if target.starts_with("i686-") {
        "i686-w64-mingw32-windres"
    } else {
        "windres"
    };

    let status = Command::new(windres)
        .current_dir(&windows_dir)
        .arg("--input")
        .arg("app.rc")
        .arg("--output")
        .arg(&obj)
        .arg("--output-format=coff")
        .status()
        .unwrap_or_else(|e| panic!("failed to spawn {windres}: {e}"));
    if !status.success() {
        panic!("{windres} failed with {status}");
    }

    println!("cargo:rustc-link-arg={}", obj.display());
}
