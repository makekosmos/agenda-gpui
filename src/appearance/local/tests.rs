use super::*;

fn temp_path() -> PathBuf {
    std::env::temp_dir()
        .join(format!("agenda-local-appearance-{}", uuid::Uuid::new_v4()))
        .join("appearance.json")
}

#[test]
fn saves_and_loads_stable_local_choices_across_restart() {
    let path = temp_path();
    {
        let writer = LocalWriter::start(path.clone());
        writer.save((0, 1, 0));
        writer.save((2, 4, 2));
    } // Flush pending debounced value on shutdown.
    let saved = LocalAppearance::load(&path).unwrap();
    assert_eq!(saved.selection(), (2, 4, 2));
    assert_eq!(saved.theme, crate::palettes::THEMES[4].key);
    let contents = std::fs::read_to_string(&path).unwrap();
    assert!(contents.contains("\"schema_version\":1"));
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[test]
fn invalid_local_file_uses_defaults_without_overwriting_file() {
    let path = temp_path();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    for invalid in [
        "{broken",
        r#"{"schema_version":1,"mode":"dark","theme":"missing","material":"mica"}"#,
    ] {
        std::fs::write(&path, invalid).unwrap();
        assert_eq!(LocalAppearance::load(&path), None);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), invalid);
    }
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[test]
fn debounced_writer_keeps_latest_local_change() {
    let path = temp_path();
    let writer = LocalWriter::start(path.clone());
    writer.save((0, 0, 0));
    writer.save((1, 2, 1));
    std::thread::sleep(Duration::from_millis(350));
    assert_eq!(LocalAppearance::load(&path).unwrap().selection(), (1, 2, 1));
    writer.save((2, 3, 2));
    drop(writer);
    assert_eq!(LocalAppearance::load(&path).unwrap().selection(), (2, 3, 2));
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}
