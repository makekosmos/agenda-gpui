#!/usr/bin/env bash
# Build the browser demo bundle: agenda-gpui compiled to wasm32 + wasm-bindgen
# JS glue + index.html + the icon set the fetch-backed AssetSource downloads.
#
# Requires: rustup toolchain `nightly` with rust-src and the wasm32-unknown-unknown
# target (gpui-pre-web's multithreaded scheduler uses `#![feature]` and atomics,
# so std must be rebuilt via -Zbuild-std), plus a `wasm-bindgen` CLI matching the
# wasm-bindgen crate in Cargo.lock.
#
# Usage: scripts/build-web.sh [out-dir]   (default: web/dist)
set -euo pipefail
cd "$(dirname "$0")/.."

OUT=${1:-web/dist}
# +atomics codegen alone is not enough — wasm-ld must emit a *shared* memory
# (requires an explicit maximum) or wasm_thread workers and the Atomics
# mailbox never see each other. Since rust PR 156174 the TLS/threading
# symbols wasm-bindgen rewrites are no longer implicit exports — ask for
# them by name.
RUSTFLAGS="-C target-feature=+atomics,+bulk-memory,+mutable-globals -C link-arg=--shared-memory -C link-arg=--max-memory=4294967296 -C link-arg=--import-memory -C link-arg=--export=__heap_base -C link-arg=--export=__wasm_init_tls -C link-arg=--export=__tls_size -C link-arg=--export=__tls_align -C link-arg=--export=__tls_base"
export RUSTFLAGS

cargo +nightly build --release --target wasm32-unknown-unknown \
    -Zbuild-std=std,panic_abort

WASM=target/wasm32-unknown-unknown/release/agenda-gpui.wasm

# std::time::{Instant,SystemTime}::now panic on wasm32-unknown-unknown and a
# few dependency paths (gpui-pre's frame instrumentation, gpui-base timing)
# call them — retarget those bodies at web_time's implementations.
python3 scripts/patch-wasm-time.py "$WASM"

# The wasm-bindgen CLI must match the crate version exactly.
WANT=$(grep -A1 'name = "wasm-bindgen"' Cargo.lock | grep 'version' | head -1 \
    | sed -E 's/.*"([^"]+)".*/\1/')
HAVE=$(wasm-bindgen --version 2>/dev/null | awk '{print $2}' || true)
if [ "$HAVE" != "$WANT" ]; then
    echo "wasm-bindgen CLI $HAVE != crate $WANT — install:" >&2
    echo "  cargo install wasm-bindgen-cli --version $WANT --locked" >&2
    exit 1
fi

rm -rf "$OUT"
mkdir -p "$OUT"
wasm-bindgen --target web --no-typescript "$WASM" --out-dir "$OUT"

# bindgen's threading transform rewrites call operands — the
# `call now; unreachable` sites only exist in the emitted module.
python3 scripts/fix-wasm-noreturn.py "$OUT/agenda-gpui_bg.wasm"

cp web/index.html "$OUT/index.html"

# Icon set for gpui-kit-assets' wasm AssetSource: it fetches
# `<endpoint>/assets/icons/<name>.svg` on demand; endpoint "" means same origin.
ASSETS_SRC=$(python3 - <<'PY'
import json, subprocess, os
m = json.loads(subprocess.check_output(
    ["cargo", "+nightly", "metadata", "--format-version", "1"]))
for p in m["packages"]:
    if p["name"] == "gpui-kit-assets":
        print(os.path.join(os.path.dirname(p["manifest_path"]), "assets"))
        break
PY
)
if [ -z "$ASSETS_SRC" ] || [ ! -d "$ASSETS_SRC" ]; then
    echo "gpui-kit-assets assets dir not found" >&2
    exit 1
fi
mkdir -p "$OUT/assets"
cp -R "$ASSETS_SRC/." "$OUT/assets/"

echo "Bundle written to $OUT:"
du -sh "$OUT" "$OUT"/*.wasm | sed 's/^/  /'
