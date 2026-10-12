use super::{EngineError, ErrorKind};
#[cfg(not(target_family = "wasm"))]
use serde::Deserialize;
#[cfg(not(target_family = "wasm"))]
use serde_json::json;
use serde_json::Value;
#[cfg(not(target_family = "wasm"))]
use std::path::PathBuf;
#[cfg(not(target_family = "wasm"))]
use std::time::Duration;

#[cfg(not(target_family = "wasm"))]
#[derive(Deserialize)]
struct Lock {
    format_version: u32,
    api_version: Version,
    http_port: u16,
    auth_token: String,
}

#[cfg(not(target_family = "wasm"))]
#[derive(Deserialize)]
struct Version {
    major: u32,
}

pub struct Engine {
    /// Overrides discovery; read by the native `rpc`.
    #[cfg(not(target_family = "wasm"))]
    pub data_dir: Option<PathBuf>,
    #[cfg(not(target_family = "wasm"))]
    agent: ureq::Agent,
}

impl Default for Engine {
    fn default() -> Self {
        #[cfg(target_family = "wasm")]
        {
            Self {}
        }
        #[cfg(not(target_family = "wasm"))]
        {
            Self::with_timeout(Duration::from_secs(15))
        }
    }
}

impl Engine {
    #[cfg(not(target_family = "wasm"))]
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            data_dir: None,
            agent: ureq::AgentBuilder::new()
                .timeout(timeout)
                .redirects(0)
                .build(),
        }
    }

    /// wasm never reaches Engine — demo mode short-circuits before any call;
    /// this stub exists so the store surface compiles unchanged.
    #[cfg(target_family = "wasm")]
    pub fn with_timeout(_timeout: std::time::Duration) -> Self {
        Self {}
    }

    #[cfg(target_family = "wasm")]
    pub fn rpc(&self, _operation: &str, _params: Value) -> Result<Value, EngineError> {
        Err(EngineError::local(
            ErrorKind::NotRunning,
            "engine transport unavailable on wasm",
        ))
    }

    #[cfg(not(target_family = "wasm"))]
    pub fn rpc(&self, operation: &str, mut params: Value) -> Result<Value, EngineError> {
        let directory = self
            .data_dir
            .clone()
            .map(Ok)
            .unwrap_or_else(data_dir)
            .map_err(|detail| EngineError::local(ErrorKind::NotRunning, detail))?;
        // Re-read discovery on every request: Engine may have restarted.
        let bytes = std::fs::read(directory.join("engine.lock.json")).map_err(|e| {
            EngineError::local(
                ErrorKind::NotRunning,
                format!("engine.lock.json unreadable: {e}"),
            )
        })?;
        let lock: Lock = serde_json::from_slice(&bytes).map_err(|e| {
            EngineError::local(
                ErrorKind::NotCompatible,
                format!("engine.lock.json malformed: {e}"),
            )
        })?;
        if lock.format_version != 1
            || lock.api_version.major != 1
            || lock.http_port == 0
            || lock.auth_token.len() != 64
            || !lock.auth_token.bytes().all(|v| v.is_ascii_hexdigit())
        {
            return Err(EngineError::local(
                ErrorKind::NotCompatible,
                "engine.lock.json: incompatible format",
            ));
        }
        if !params.is_object() {
            return Err(EngineError::local(
                ErrorKind::InvalidRequest,
                "local: params not an object",
            ));
        }
        params["operation"] = json!(operation);
        params["_req_id"] = json!(uuid::Uuid::new_v4().to_string());
        let response = match self
            .agent
            .post(&format!("http://127.0.0.1:{}/v1/rpc", lock.http_port))
            .set("Authorization", &format!("Bearer {}", lock.auth_token))
            .set("X-Kosmos-Api-Version", "1.0.0")
            .set("X-Kosmos-Client-Class", "agenda-gpui")
            .set("X-Kosmos-Client-Version", env!("CARGO_PKG_VERSION"))
            .set("X-Kosmos-Client-Pid", &std::process::id().to_string())
            .send_json(params)
        {
            Ok(response) => response,
            // Engine reports handled failures as non-2xx `{ok:false,error}` —
            // ureq surfaces that body as Err(Status); keep its real code.
            Err(ureq::Error::Status(_, response)) => {
                let detail = response
                    .into_json::<Value>()
                    .ok()
                    .and_then(|v| v["error"].as_str().map(str::to_owned))
                    .unwrap_or_else(|| "http-status".into());
                return Err(EngineError::engine(&detail));
            }
            Err(error) => {
                // A refused connection means Engine died after the lock read.
                let kind = match &error {
                    ureq::Error::Transport(t) if t.kind() == ureq::ErrorKind::ConnectionFailed => {
                        ErrorKind::NotRunning
                    }
                    _ => ErrorKind::Transport,
                };
                return Err(EngineError::local(kind, error.to_string()));
            }
        };
        let value: Value = response.into_json().map_err(|e| {
            EngineError::local(ErrorKind::Transport, format!("response not json: {e}"))
        })?;
        if value["ok"] != true {
            let detail = value["error"].as_str().unwrap_or("unknown").to_owned();
            return Err(EngineError::engine(&detail));
        }
        value
            .get("data")
            .cloned()
            .ok_or_else(|| EngineError::local(ErrorKind::Transport, "response missing `data`"))
    }
}

/// Per-OS config dir: `%APPDATA%` (Windows), `~/Library/Application Support`
/// (macOS), `$XDG_CONFIG_HOME` / `~/.config` (Linux). Discovery is
/// native-only — the wasm stub `rpc` never consults the filesystem.
#[cfg(all(target_os = "windows", not(target_family = "wasm")))]
fn config_base(get: &dyn Fn(&str) -> Option<PathBuf>) -> Option<PathBuf> {
    get("APPDATA")
}
#[cfg(all(target_os = "macos", not(target_family = "wasm")))]
fn config_base(get: &dyn Fn(&str) -> Option<PathBuf>) -> Option<PathBuf> {
    get("HOME").map(|v| v.join("Library/Application Support"))
}
#[cfg(not(any(target_os = "windows", target_os = "macos", target_family = "wasm")))]
fn config_base(get: &dyn Fn(&str) -> Option<PathBuf>) -> Option<PathBuf> {
    get("XDG_CONFIG_HOME").or_else(|| get("HOME").map(|v| v.join(".config")))
}

/// Engine data-dir candidates in priority order: `MUNDUS_DATA_DIR` →
/// `KOSMOS_DATA_DIR` (legacy) → `<config>/Mundus` → `<config>/Kosmos` (legacy).
// MIGRATION(KOS-267): drop the two legacy candidates after 2026-11-01.
#[cfg(not(target_family = "wasm"))]
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
#[cfg(not(target_family = "wasm"))]
pub(super) fn resolve(get: &dyn Fn(&str) -> Option<PathBuf>) -> Result<PathBuf, String> {
    let dirs = candidates(get);
    dirs.iter()
        .find(|dir| dir.join("engine.lock.json").is_file())
        .cloned()
        .or_else(|| dirs.into_iter().next())
        .ok_or_else(|| format!("Не найдена папка данных {}", crate::brand::NAME))
}

#[cfg(not(target_family = "wasm"))]
fn data_dir() -> Result<PathBuf, String> {
    resolve(&|key| std::env::var_os(key).map(PathBuf::from))
}
