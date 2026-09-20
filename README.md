# agenda-gpui

Экспериментальный native GPUI-клон Agenda (KOS-124). Не замена Vue/Electron Agenda.

## Требования

- Rust stable
- `hk` для Git hooks: `cargo install hk --locked`, затем `hk install` в корне репозитория
- Гейты pre-push: `cargo nextest`, `cargo shear`, `cargo clippy`, `cargo deny` (ставятся через `cargo install` по необходимости)
- Linux: обычный `cargo run`
- Windows release: нужен Windows SDK `fxc.exe` в `PATH` или `GPUI_FXC_PATH` (вшитые DXBC-шейдеры). Иначе падение на DirectWrite/шейдерах.

Весь гейт можно прогнать вручную: `hk check --all` или `hk run pre-push`.

## CI и ночные релизы

GitHub Actions проверяет форматирование, правила версий, Clippy (Windows),
тесты и release-сборки при push в `main` и в pull request.
Архивы сборок доступны в Artifacts каждого успешного запуска в течение 7 дней.

Каждый день в **00:00 МСК** (`21:00 UTC`) workflow `Build and release` выпускает
новую версию, если дерево исходников изменилось с последнего опубликованного
релиза. Очередь GitHub может задержать запуск. Ручной запуск: Actions →
Build and release → Run workflow на основной ветке.

- Первый релиз использует текущую версию из `Cargo.toml`.
- Если версия осталась прежней, увеличивается patch: `0.1.0` → `0.1.1`.
- Для своего номера измени `[package].version` в `Cargo.toml`, например на
  `0.2.0` или `1.0.0`, и обнови `Cargo.lock` командой `cargo check`.
  Номер должен быть выше последнего релиза; поддерживаются версии `X.Y.Z`.
- Все платформы собирают один commit с одной версией. После успешных проверок
  бот коммитит `Cargo.toml` и `Cargo.lock`, создаёт тег `vX.Y.Z` и публикует
  GitHub Release с архивами и `SHA256SUMS.txt`. Окно «О приложении» показывает
  версию из Cargo. Локальную ветку после релиза нужно обновить через `git pull`.
- Если основная ветка изменилась во время сборки, публикация останавливается:
  запусти workflow заново на новой вершине ветки. При сбое загрузки релиза
  повтори неудачный job до добавления новых коммитов; готовый черновик можно
  дозагрузить повторным запуском, опубликованные релизы не перезаписываются.

Платформы: **Windows x86_64** (ZIP с EXE), **Linux x86_64** (tar.gz,
сборка на Ubuntu 24.04) и **macOS Apple Silicon** (tar.gz с `Agenda.app`).
Linux-архив не включает системные библиотеки. macOS-приложение имеет локальную
ad-hoc подпись, без Apple notarization; Windows EXE также без издательской подписи.

Workflow использует стандартный `GITHUB_TOKEN`, отдельный PAT не нужен.
Правила основной ветки должны разрешать release-job писать версию и теги;
если push запрещён правилами репозитория, job остановится без force-push.
Расписание начнёт работать после попадания workflow в основную ветку.
GitHub отключает расписания публичных репозиториев после 60 дней без активности;
при необходимости включи workflow снова в Actions.

Проверка логики версий локально: `python scripts/test_release.py` (Python 3.11+).

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
