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
- Engine ищется по `engine.lock.json` в `MUNDUS_DATA_DIR`; без неё — в папке
  конфигурации Mundus (`src/brand.rs`). Старые `KOSMOS_DATA_DIR` и `Kosmos`
  пока читаются как запасной путь: первый найденный lock-файл побеждает.
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
`main`: `cargo fmt --check`, `cargo clippy -- -D warnings`, тесты, гейт размера
файлов и release-сборка на Windows, Linux и macOS; плюс проверка правил
версий (`python scripts/test_release.py`, `scripts/test_release_notes.py`).
Ночью (00:00 МСК) тот же workflow выпускает релиз.

Локально те же проверки гонит `hk` (`hk.pkl`):

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
  список `GRANDFATHERED` только сокращается.
- `gpui` (псевдоним `gpui-kit`), `gpui-component`, `gpui-base` закреплены
  точными версиями и должны совпадать с cortex/manager-gpui, memoria-gpui и
  dictation: две версии gpui в одной сборке — ошибка типов. Поднимай вместе.
  `imago-gpui`, `mundus-gpui-kit` и `[patch.crates-io]` закреплены по rev
  репозитория `makekosmos/imago`.
- Цвета и шрифты — только через токены темы (`theme.rs`, `palettes.rs`).
  Сохраняй клавиатурный доступ, фокус и доступные имена элементов.
- Подписки, таймеры и слушатели снимай при уничтожении окна или view.
- Строки интерфейса — русские; смотри соседние экраны.

## Релиз

Выпускает workflow по расписанию, тестируя ровно тот коммит, который
публикует: патч-версия растёт сама, версию в `Cargo.toml` вручную не меняй, тег
не ставь и релиз не публикуй без просьбы (`scripts/release.py`,
`scripts/publish-version.sh`).
