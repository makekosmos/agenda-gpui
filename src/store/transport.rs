use serde::Deserialize;
use serde_json::{json, Value};
use std::{path::PathBuf, time::Duration};

#[derive(Deserialize)]
struct Lock {
    format_version: u32,
    api_version: Version,
    http_port: u16,
    auth_token: String,
}

#[derive(Deserialize)]
struct Version {
    major: u32,
}

pub struct Engine {
    pub data_dir: Option<PathBuf>,
    agent: ureq::Agent,
}

impl Default for Engine {
    fn default() -> Self {
        Self {
            data_dir: None,
            agent: ureq::AgentBuilder::new()
                .timeout(Duration::from_secs(15))
                .redirects(0)
                .build(),
        }
    }
}

impl Engine {
    pub fn rpc(&self, operation: &str, mut params: Value) -> Result<Value, String> {
        let directory = self.data_dir.clone().map(Ok).unwrap_or_else(data_dir)?;
        // Re-read discovery on every request: Engine may have restarted.
        let bytes = std::fs::read(directory.join("engine.lock.json")).map_err(|_| {
            format!(
                "Engine не запущен. Запустите {} и обновите список.",
                crate::brand::NAME
            )
        })?;
        let lock: Lock =
            serde_json::from_slice(&bytes).map_err(|_| "Некорректный файл состояния Engine")?;
        if lock.format_version != 1
            || lock.api_version.major != 1
            || lock.http_port == 0
            || lock.auth_token.len() != 64
            || !lock.auth_token.bytes().all(|v| v.is_ascii_hexdigit())
        {
            return Err(format!(
                "Несовместимое состояние Engine. Обновите {}.",
                crate::brand::NAME
            ));
        }
        if !params.is_object() {
            return Err("Некорректный запрос Engine".into());
        }
        params["operation"] = json!(operation);
        params["_req_id"] = json!(uuid::Uuid::new_v4().to_string());
        let response = self
            .agent
            .post(&format!("http://127.0.0.1:{}/v1/rpc", lock.http_port))
            .set("Authorization", &format!("Bearer {}", lock.auth_token))
            .set("X-Kosmos-Api-Version", "1.0.0")
            .set("X-Kosmos-Client-Class", "agenda-gpui")
            .set("X-Kosmos-Client-Version", env!("CARGO_PKG_VERSION"))
            .set("X-Kosmos-Client-Pid", &std::process::id().to_string())
            .send_json(params)
            .map_err(|_| {
                "Нет подтверждения от Engine. Обновите список перед повтором.".to_string()
            })?;
        let value: Value = response
            .into_json()
            .map_err(|_| "Некорректный ответ Engine")?;
        if value["ok"] != true {
            return Err("Engine отклонил операцию. Изменение не подтверждено.".into());
        }
        value
            .get("data")
            .cloned()
            .ok_or("Engine не вернул результат".into())
    }
}

/// Per-OS config dir: `%APPDATA%` (Windows), `~/Library/Application Support`
/// (macOS), `$XDG_CONFIG_HOME` / `~/.config` (Linux).
#[cfg(target_os = "windows")]
fn config_base(get: &dyn Fn(&str) -> Option<PathBuf>) -> Option<PathBuf> {
    get("APPDATA")
}
#[cfg(target_os = "macos")]
fn config_base(get: &dyn Fn(&str) -> Option<PathBuf>) -> Option<PathBuf> {
    get("HOME").map(|v| v.join("Library/Application Support"))
}
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn config_base(get: &dyn Fn(&str) -> Option<PathBuf>) -> Option<PathBuf> {
    get("XDG_CONFIG_HOME").or_else(|| get("HOME").map(|v| v.join(".config")))
}

/// Engine data-dir candidates in priority order: `MUNDUS_DATA_DIR` →
/// `KOSMOS_DATA_DIR` (legacy) → `<config>/Mundus` → `<config>/Kosmos` (legacy).
// MIGRATION(KOS-267): drop the two legacy candidates after 2026-11-01.
fn candidates(get: &dyn Fn(&str) -> Option<PathBuf>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    for key in [
        crate::brand::DATA_DIR_ENV,
        crate::brand::LEGACY_DATA_DIR_ENV,
    ] {
        if let Some(path) = get(key).filter(|v| !v.as_os_str().is_empty()) {
            dirs.push(path);
        }
    }
    if let Some(base) = config_base(get) {
        dirs.push(base.join(crate::brand::DATA_DIR_NAME));
        dirs.push(base.join(crate::brand::LEGACY_DATA_DIR_NAME));
    }
    dirs
}

/// First candidate holding `engine.lock.json` wins — a new app may still run
/// against a pre-rename Engine that only wrote the legacy dir. With no lock
/// anywhere the primary candidate is returned so the caller reports a stable
/// path (and the missing-lock error stays the same).
fn resolve(get: &dyn Fn(&str) -> Option<PathBuf>) -> Result<PathBuf, String> {
    let dirs = candidates(get);
    dirs.iter()
        .find(|dir| dir.join("engine.lock.json").is_file())
        .cloned()
        .or_else(|| dirs.into_iter().next())
        .ok_or_else(|| format!("Не найдена папка данных {}", crate::brand::NAME))
}

fn data_dir() -> Result<PathBuf, String> {
    resolve(&|key| std::env::var_os(key).map(PathBuf::from))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Env-var lookup over a fixed map — tests never touch the real process env.
    fn fake_env(pairs: Vec<(&'static str, PathBuf)>) -> impl Fn(&str) -> Option<PathBuf> {
        move |key| {
            pairs
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v.clone())
        }
    }

    fn tmpdir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("agenda-gpui-kos266-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_lock(dir: &std::path::Path) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("engine.lock.json"), "{}").unwrap();
    }

    /// Env var(s) that feed `config_base` on this OS, bound to `root`.
    #[cfg(target_os = "windows")]
    fn config_env(root: &std::path::Path) -> Vec<(&'static str, PathBuf)> {
        vec![("APPDATA", root.to_path_buf())]
    }
    #[cfg(target_os = "macos")]
    fn config_env(root: &std::path::Path) -> Vec<(&'static str, PathBuf)> {
        vec![("HOME", root.to_path_buf())]
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    fn config_env(root: &std::path::Path) -> Vec<(&'static str, PathBuf)> {
        vec![("XDG_CONFIG_HOME", root.to_path_buf())]
    }

    /// What `config_base` should return for `config_env(root)` on this OS.
    fn expected_config_base(root: &std::path::Path) -> PathBuf {
        #[cfg(target_os = "macos")]
        {
            root.join("Library/Application Support")
        }
        #[cfg(not(target_os = "macos"))]
        {
            root.to_path_buf()
        }
    }

    #[test]
    fn env_override_wins() {
        let dir = tmpdir("env");
        write_lock(&dir);
        let env = fake_env(vec![(crate::brand::DATA_DIR_ENV, dir.clone())]);
        assert_eq!(resolve(&env).unwrap(), dir);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn legacy_env_used_when_primary_has_no_lock() {
        let root = tmpdir("legacy-env");
        let primary = root.join("new");
        let legacy = root.join("old");
        std::fs::create_dir_all(&primary).unwrap();
        write_lock(&legacy);
        let env = fake_env(vec![
            (crate::brand::DATA_DIR_ENV, primary),
            (crate::brand::LEGACY_DATA_DIR_ENV, legacy.clone()),
        ]);
        assert_eq!(resolve(&env).unwrap(), legacy);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn config_dir_and_legacy_fallback() {
        let root = tmpdir("config");
        let base = expected_config_base(&root);
        let legacy = base.join(crate::brand::LEGACY_DATA_DIR_NAME);
        write_lock(&legacy);
        let env = fake_env(config_env(&root));
        assert_eq!(resolve(&env).unwrap(), legacy);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn mundus_dir_beats_legacy_dir() {
        let root = tmpdir("both");
        let base = expected_config_base(&root);
        let mundus = base.join(crate::brand::DATA_DIR_NAME);
        write_lock(&mundus);
        write_lock(&base.join(crate::brand::LEGACY_DATA_DIR_NAME));
        let env = fake_env(config_env(&root));
        assert_eq!(resolve(&env).unwrap(), mundus);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn no_lock_anywhere_returns_primary_candidate() {
        let root = tmpdir("nolock");
        let primary = expected_config_base(&root).join(crate::brand::DATA_DIR_NAME);
        let env = fake_env(config_env(&root));
        assert_eq!(resolve(&env).unwrap(), primary);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn empty_env_var_is_ignored() {
        let root = tmpdir("empty");
        let base = expected_config_base(&root);
        let mundus = base.join(crate::brand::DATA_DIR_NAME);
        write_lock(&mundus);
        let mut pairs = vec![(crate::brand::DATA_DIR_ENV, PathBuf::from(""))];
        pairs.extend(config_env(&root));
        let env = fake_env(pairs);
        assert_eq!(resolve(&env).unwrap(), mundus);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn no_candidates_is_an_error() {
        let env = fake_env(vec![]);
        let err = resolve(&env).unwrap_err();
        assert!(err.contains(crate::brand::NAME), "{err}");
    }
}
