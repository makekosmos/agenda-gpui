# agenda-gpui

Экспериментальный native GPUI-клон Agenda (KOS-124). Не замена Vue/Electron Agenda.

## Требования

- Rust stable
- `hk` для Git hooks: `cargo install hk --locked`, затем `hk install` в корне репозитория
- Гейты pre-push: `cargo nextest`, `cargo shear`, `cargo clippy`, `cargo deny` (ставятся через `cargo install` по необходимости)
- Linux: обычный `cargo run`
- Windows release: нужен Windows SDK `fxc.exe` в `PATH` или `GPUI_FXC_PATH` (вшитые DXBC-шейдеры). Иначе падение на DirectWrite/шейдерах.

Весь гейт можно прогнать вручную: `hk check --all` или `hk run pre-push`.

## Сборка

```bash
git clone git@github.com:makekosmos/agenda-gpui.git
cd agenda-gpui
cargo run
cargo run --release
```

Windows cross (Linux):

```bash
export GPUI_FXC_PATH=/path/to/fxc   # wine-wrapper ok
cargo build --release --target x86_64-pc-windows-gnu
```

## Структура

- `src/` — UI
- `assets/` — шрифты и иконки
- `vendor/gpui-pre-*-patched/` — локальные патчи gpui-pre 0.3.5 для window hit-test
- `windows/` — манифест Common Controls v6

Вынесено из `makekosmos/agenda` ветка `kos-124` / PR #32.
