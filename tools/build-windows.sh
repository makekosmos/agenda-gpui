#!/usr/bin/env bash
# build-windows.sh — cross-build AgendaGpui.exe for x86_64-pc-windows-gnu.
#
# The vendored gpui (vendor/gpui-0.2.2-patched) runs its HLSL compile step on
# any host when the target is Windows, so it just needs a working fxc via
# GPUI_FXC_PATH. Preference order:
#   1. $GPUI_FXC_PATH already set (e.g. a real SDK fxc.exe under Wine),
#   2. /home/box/bin/fxc (this box's Wine-wrapped SDK fxc 10.1),
#   3. tools/fxc — repo shim compiling real DXBC blobs with Microsoft's dxc
#      (official Linux binary, unpacked into tools/dxc/).
#
# Usage: tools/build-windows.sh [dist-dir]
set -euo pipefail

cd "$(dirname "$0")/.."
TARGET=x86_64-pc-windows-gnu
DIST="${1:-/home/box/devin-runs/kos-124/dist}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [[ -z "${GPUI_FXC_PATH:-}" ]]; then
    if [[ -x /home/box/bin/fxc ]]; then
        GPUI_FXC_PATH=/home/box/bin/fxc
    else
        GPUI_FXC_PATH="$SCRIPT_DIR/fxc"
    fi
fi
export GPUI_FXC_PATH
echo "== GPUI_FXC_PATH=$GPUI_FXC_PATH =="

cargo build --release --target "$TARGET"

mkdir -p "$DIST"
cp "target/$TARGET/release/agenda-gpui.exe" "$DIST/AgendaGpui.exe"
echo "== wrote $DIST/AgendaGpui.exe =="
