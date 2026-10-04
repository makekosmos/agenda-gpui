# agenda-gpui: agent instructions

## Scope and entry points

Experimental native GPUI clone of the Agenda task/calendar app (KOS-124).
Single binary crate; data access goes through the local Mundus Engine API —
the app never opens the ARK database itself (write boundary: cortex
`docs/write-boundary.md`).

- `src/main.rs`, `src/app/` — GPUI app shell and windows.
- `src/pages/` — views; `src/widgets.rs`, `src/chrome.rs` — shared UI.
- `src/model/`, `src/store/` — state and Engine transport
  (`src/store/transport.rs` reads `engine.lock.json`, POSTs `/v1/rpc`).
- `src/theme.rs`, `src/palettes.rs`, `src/appearance/` — theme/appearance;
  visual core is shared through `imago-gpui`.
- `src/*_tests.rs`, `src/ui_tests.rs`, `src/a11y_tests.rs` — `#[gpui::test]`
  UI/a11y/regression suites behind gpui's `test-support` feature.
- `src/bin/check-source-size.rs` — file-size gate. `snapshots/` — test
  snapshots. `windows/` — icon/manifest for embed-resource.
- `hk.pkl`: the local gate (pre-commit + pre-push). `.github/workflows/`:
  CI + nightly release — keep them working; this is a public repo.

## Setup and verification

```powershell
cargo install hk --locked; hk install   # once per checkout
hk run pre-push                          # or: hk check --all
```

Gate = `cargo nextest run --workspace --all-features`, `cargo shear`,
`cargo clippy`, `cargo deny`, `check-source-size` (see `hk.pkl` for the
exact commands and allow-lists). Install `cargo-nextest`, `cargo-shear`,
`cargo-deny` on demand.

Windows release builds need the SDK `fxc.exe` in `PATH` or `GPUI_FXC_PATH`
(embedded DXBC shaders; DirectWrite fails otherwise).

## Contracts to preserve

- `gpui` (aliased `gpui-kit`), `gpui-component`, `gpui-base` are exact-pinned
  and must stay identical to `memoria-gpui` and `cortex/manager-gpui` — two
  gpui versions in one build are type errors. Bump them together.
- `imago-gpui` is pinned by git rev to `makekosmos/imago`;
  `[patch.crates-io]` `gpui-pre{,-windows}` follows the shared patched gpui
  revision (KOS-328; local patches for WM_NCHITTEST, frame instrumentation,
  a11y TestWindow — see `crates/gpui-pre/PATCH.md` in imago).
- Engine discovery: `engine.lock.json` under `MUNDUS_DATA_DIR`
  (`src/brand.rs`); first candidate dir containing the lock file wins.
- All persistence goes through Engine ops; never write SQLite directly.
- Preserve keyboard access, focus behavior, accessible names, and theme
  tokens — no hardcoded hex/fonts outside `theme.rs`/`palettes.rs`.
- Clean up listeners, timers and subscriptions on disposal.

## Completion

- One logical change per commit; no drive-by refactors.
- `cargo fmt` before committing; run targeted tests for what you touched,
  then `hk run pre-push` before pushing.
- Report exact commands and PASS / FAIL / NOT_RUN with reasons; for UI
  changes state whether you verified visually.
- Do not bump `[package].version`, tag, or publish releases unless
  authorized — the nightly Build workflow auto-releases changed trees
  (`scripts/release.py`, `publish-version.sh`).
