# Cross-compile patch (KOS-124)

Upstream gpui 0.2.2 `build.rs` gates `windows::build()` (fxc shader compile) on
`#[cfg(target_os = "windows")]` of the **host**. That skips fxc when
cross-compiling from Linux even if `CARGO_CFG_TARGET_OS=windows`.

Changes vs crates.io 0.2.2:
1. Always call `windows::build()` when target OS is windows (runtime env check).
2. Gate `embed_resource` on host `target_os = "windows"` (build-dep is host-gated).

Remove this path dep once upstream fixes host/target cfg for shader compile.
