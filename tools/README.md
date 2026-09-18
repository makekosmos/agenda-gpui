# Windows cross-build tools

`tools/build-windows.sh` cross-compiles `AgendaGpui.exe` for
`x86_64-pc-windows-gnu` from Linux. gpui's HLSL shaders must be compiled at
build time (release embeds DXBC blobs; see `vendor/gpui-0.2.2-patched`), so the
build needs an fxc-compatible compiler via `GPUI_FXC_PATH`.

## Option A — real Windows SDK fxc.exe under Wine (preferred)

Produces byte-identical upstream shader blobs (HLSL Compiler 10.1, SM 4.x):

1. Extract `fxc.exe` from a Windows SDK (e.g. `10.0.26100.0/x64/fxc.exe`) into a
   Wine prefix.
2. Wrap it in a script that translates `Z:\` paths and invokes
   `wine fxc.exe "$@"` — see `/home/box/bin/fxc` on the build box for the exact
   wrapper used for the shipped binary.
3. `GPUI_FXC_PATH=/home/box/bin/fxc tools/build-windows.sh`

## Option B — `tools/fxc` shim + Microsoft dxc (Linux)

`tools/fxc` accepts the same `/T /E /Fh /Vn` arguments gpui's build.rs passes
and produces real DXBC blobs plus an fxc-format C header.

1. Download the official Linux DXC release
   (e.g. `linux_dxc_2026_07_29.x86_x64.tar.gz` from
   https://github.com/microsoft/DirectXShaderCompiler/releases).
2. Unpack so `tools/dxc/bin/dxc` exists (or set `DXC_PATH`, or put `dxc` on
   PATH). The `tools/dxc/` directory is gitignored — binaries are not vendored.
3. `tools/build-windows.sh` (auto-picks `tools/fxc` when no other fxc exists).

dxc emits DXIL bytecode (SM 6.x promoted from the 4.x profiles) rather than
fxc's legacy DXBC — it requires a D3D11 runtime with DXIL support
(Windows 10 1809+ / 11), so Option A is preferred and is what the shipped
artifact was built with.
