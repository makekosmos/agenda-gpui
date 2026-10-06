# AGENTS.md — agenda-gpui

Нативное GPUI-приложение Agenda (задачи и календарь) для Mundus. Один бинарный
крейт `agenda-gpui`. Главная платформа — Windows; Linux и macOS собираются и
проверяются в CI. Приложение ставится Engine из GitHub Releases этого репо.

## Карта

- `src/main.rs`, `src/app.rs`, `src/app/` — оболочка приложения и окна.
- `src/pages/` — экраны; `src/widgets.rs`, `src/chrome.rs`, `src/text_field.rs` —
  общий UI.
- `src/model/`, `src/store/` — состояние и транспорт к Engine
  (`src/store/transport.rs`: читает `engine.lock.json`, шлёт `POST /v1/rpc`).
- `src/theme.rs`, `src/palettes.rs`, `src/appearance/` — тема; общее визуальное
  ядро лежит в `imago-gpui` (репозиторий `makekosmos/imago`).
- `src/*_tests.rs` — тесты `#[gpui::test]` (UI, a11y, регрессии);
  `snapshots/` — эталон дерева доступности; `fixtures/engine/` — стенд
  контракта Engine.
- `src/bin/check-source-size.rs` — гейт размера файлов; `windows/`, `build.rs` —
  иконка, манифест, VERSIONINFO.

## Границы

- Все данные идут через операции Engine. Приложение никогда не открывает
  SQLite само (`cortex/docs/write-boundary.md`) и не хранит копию задач.
- Engine ищется по `engine.lock.json`; порядок кандидатов: `MUNDUS_DATA_DIR`,
  `KOSMOS_DATA_DIR`, `<config>/Mundus`, `<config>/Kosmos` — побеждает первая
  папка с lock-файлом (`src/store/transport.rs`, `src/brand.rs`). Старые
  `KOSMOS_*`/`Kosmos` — переходный запас под `MIGRATION(KOS-267)`.
- Переменные окружения для разработки: `AGENDA_DEMO=1` (демо без Engine,
  ничего не сохраняется), `AGENDA_DEVTASKS=N`, `AGENDA_ROUTE`,
  `AGENDA_OVERLAY`, `AGENDA_FPS`, `AGENDA_VSYNC`, `AGENDA_OFFSCREEN`,
  `AGENDA_EMIT_FIXTURES`. Без Engine и без `AGENDA_DEMO` приложение показывает
  ошибку подключения, а не тестовые карточки.

## Запуск

```text
cargo run                                   # против живого Engine
MUNDUS_DATA_DIR=<dir> cargo run             # другая папка данных
AGENDA_DEMO=1 cargo run                     # без Engine
```

Engine берётся из `makekosmos/cortex`: `pnpm run dev -- --engine-only` в корне
cortex (общая папка данных — `--data-dir DIR`, тот же `DIR` отдай приложению).
На Linux нужен `ld.lld` в `PATH` (`.cargo/config.toml`). Релизная сборка на
Windows требует `fxc.exe` из Windows SDK в `PATH` или `GPUI_FXC_PATH`:
шейдеры вшиваются при сборке.

## Проверки

CI (`.github/workflows/build.yml`) запускается на каждый PR и на push в
`main`. На Windows, Linux и macOS: `cargo fmt --check`, `cargo test --release`
и release-сборка; clippy и гейт размера файлов — только на Windows; плюс
проверка правил версий (`python scripts/test_release.py`,
`scripts/test_release_notes.py`). `cargo shear` и `cargo deny` в CI не
запускаются, только локально через `hk`. Ночью (00:00 МСК = 21:00 UTC) тот же
workflow выпускает релиз, если исходники изменились.

Локальный гейт — `hk` (`hk.pkl`); pre-commit у него лёгкий (`cargo check` и
`cargo fmt --check`), полный гейт идёт на pre-push:

```text
cargo install hk --locked && hk install     # один раз на копию
hk run pre-push                             # или hk check --all
```

Гейт: `cargo nextest run --workspace --all-features`, `cargo shear`,
`cargo clippy --workspace --all-targets --all-features -- -D warnings`,
`cargo deny check advisories bans sources`, `check-source-size`.
`cargo-nextest`, `cargo-shear`, `cargo-deny` ставь через `cargo install`.
Не обходи хуки через `--no-verify`.

Снимок `snapshots/a11y-inbox.txt` — дерево Windows/Linux; на macOS тест
сравнивает с ним с поправкой на отсутствие кнопок окна. Меняя дерево
доступности, обнови снимок.

## Правила кода

- Мёртвый код удаляй сразу, вместе с тестами только на него. Не используй
  `#[allow(dead_code)]` и не добавляй `-A …` в clippy (`hk.pkl`, `build.yml`):
- Размер файла — не больше 300 строк (`src/bin/check-source-size.rs`);
  список `GRANDFATHERED` (`app.rs`, `chrome.rs`, `model.rs`, `pages.rs`) только
  сокращается: новый код клади в подмодули.
- `gpui` (псевдоним `gpui-kit`) и `gpui-component` закреплены точными
  версиями и должны совпадать с cortex/manager-gpui, memoria-gpui и dictation
  (там же закреплён и `gpui-base`; он здесь приходит транзитивно): две версии
  gpui в одной сборке — ошибка типов. Поднимай вместе. `imago-gpui` и
  `mundus-gpui-kit` закреплены по одному rev репозитория `makekosmos/imago`,
  `[patch.crates-io]` (`gpui-pre*`) — по другому; их меняют отдельно.
- В `[profile.release]` `debug-assertions` должен оставаться `false`:
  иначе gpui-pre-windows не скомпилирует шейдеры (комментарий в `Cargo.toml`).
- Цвета и шрифты — только через токены темы (`theme.rs`, `palettes.rs`).
  Сохраняй клавиатурный доступ, фокус и доступные имена элементов.
- Подписки, таймеры и слушатели снимай при уничтожении окна или view.
- Строки интерфейса — русские; смотри соседние экраны.

## Релиз

Выпускает workflow по расписанию, тестируя ровно тот коммит, который
публикует: патч-версия растёт сама, версию в `Cargo.toml` вручную не меняй, тег
не ставь и релиз не публикуй без просьбы (`scripts/release.py`,
`scripts/publish-version.sh`).
