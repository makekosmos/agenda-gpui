//! Engine error classes shared by the transport and the app shell.

/// Why an Engine call failed. `kind` drives the user-facing text; `detail`
/// is the raw `error` code the Engine sent on the wire (or a local failure
/// tag) — it is written to the app log and never shown to the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineError {
    pub kind: ErrorKind,
    pub detail: String,
}

impl EngineError {
    /// A `{ok:false, error}` reply (or an HTTP error body) from Engine.
    pub(crate) fn engine(code: &str) -> Self {
        Self {
            kind: ErrorKind::from_engine_code(code),
            detail: code.to_owned(),
        }
    }

    pub(crate) fn local(kind: ErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }

    /// User-facing Russian text for the error class.
    pub fn message(&self) -> String {
        let name = crate::brand::NAME;
        match self.kind {
            ErrorKind::NotRunning => {
                format!("{name} не запущен. Запустите {name} и обновите список.")
            }
            ErrorKind::NotCompatible => {
                format!("Состояние Engine несовместимо. Обновите {name}.")
            }
            ErrorKind::Transport => {
                "Нет подтверждения от Engine. Обновите список и проверьте, что изменение сохранилось."
                    .into()
            }
            ErrorKind::InvalidRequest => {
                format!(
                    "Engine отклонил операцию: данные не прошли проверку. Обновите {name} и сообщите о проблеме."
                )
            }
            ErrorKind::Forbidden => {
                format!("Engine запретил операцию для Agenda. Обновите {name}.")
            }
            ErrorKind::Conflict => {
                "Данные изменились в другом приложении. Список обновлён — повторите изменение."
                    .into()
            }
            ErrorKind::NotFound => "Запись не найдена в Engine. Обновите список.".into(),
            ErrorKind::Timeout => "Engine не ответил вовремя. Повторите попытку.".into(),
            ErrorKind::Unavailable => {
                "Engine временно недоступен. Повторите попытку.".into()
            }
        }
    }
}

/// Engine rejection classes — the `error` codes on `/v1/rpc` replies
/// (`invalid-request`, `forbidden`, `conflict`, `not-found`, `timeout`,
/// `unavailable`; see cortex `public_app_error`) — plus the local failures
/// that can precede a reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// engine.lock.json is missing/unreadable — Engine is not running.
    NotRunning,
    /// The lock file exists but its format/api version is incompatible.
    NotCompatible,
    /// The request left the app but no confirmed reply came back.
    Transport,
    InvalidRequest,
    Forbidden,
    Conflict,
    NotFound,
    Timeout,
    Unavailable,
}

impl ErrorKind {
    /// The wire carries the raw code (`canonical_ingress:invalid_request:*`,
    /// `object_conflict:*`, plain class names); classify by its stable tokens.
    pub fn from_engine_code(code: &str) -> Self {
        let code = code.strip_prefix("package worker: ").unwrap_or(code);
        if code.contains("conflict") {
            Self::Conflict
        } else if code.contains("invalid-request") || code.contains("invalid_request") {
            Self::InvalidRequest
        } else if code.contains("forbidden") {
            Self::Forbidden
        } else if code.contains("not-found") || code.contains("not found") {
            Self::NotFound
        } else if code.contains("timeout") || code.contains("timed out") {
            Self::Timeout
        } else {
            Self::Unavailable
        }
    }
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The display form is for logs: kind + the raw engine code.
        write!(f, "{:?}: {}", self.kind, self.detail)
    }
}
