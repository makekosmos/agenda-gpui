use super::transport::resolve;
use std::path::PathBuf;

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
    let dir = std::env::temp_dir().join(format!("agenda-gpui-kos266-{tag}-{}", std::process::id()));
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
