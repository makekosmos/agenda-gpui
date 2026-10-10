use super::*;
use agenda_core::{mapping, new_todo, parse_key, LocalDay, RecurrenceRule, Status, Todo};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Engine contract fixtures (KOS-297).
//
// These are the exact objects `mapping::write_at` sends to `upsert_object`.
// They are committed under `fixtures/engine/` and consumed by the ark-core
// contract test in cortex (`phase3_agenda_contract`), which validates them
// against the real `canonical_ingress`. The two repos cannot share code, so
// the fixture files are the contract: change the write path here → regenerate
// (`AGENDA_EMIT_FIXTURES=1 cargo test engine_payload_fixtures`), then copy the files
// into `cortex/core/crates/ark-core/tests/fixtures/agenda/`.
// ---------------------------------------------------------------------------

const NOW: &str = "2026-05-15T10:00:00.000Z";

/// Fixtures pin the clock: the offset only steers `date_only` on RFC 3339
/// inputs, and the canned todos carry bare dates — UTC is fine.
fn day() -> LocalDay {
    LocalDay::new(
        chrono::NaiveDate::from_ymd_opt(2026, 5, 15).unwrap(),
        chrono::FixedOffset::east_opt(0).unwrap(),
    )
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("engine")
}

fn cases() -> Vec<(&'static str, Todo)> {
    let base = || {
        let mut t = new_todo("t", "Задача");
        t.created_at = "2026-05-14T09:00:00.000Z".into();
        t
    };
    vec![
        ("task-inbox", base()),
        ("task-date", {
            let mut t = base();
            t.scheduled_date = Some("2026-05-20".into());
            t.status = Status::Todo;
            t
        }),
        ("task-project", {
            let mut t = base();
            t.project_id = Some("p1".into());
            t.status = Status::Todo;
            t
        }),
        ("task-note", {
            let mut t = base();
            t.notes = Some("заметка".into());
            t
        }),
        ("task-deadline", {
            let mut t = base();
            t.deadline = Some("2026-05-22".into());
            t
        }),
        ("task-recurrence", {
            let mut t = base();
            t.status = Status::Todo;
            t.recurrence = Some(RecurrenceRule {
                frequency: 0,
                interval: 1,
                recurrence_type: 0,
                days_of_week: vec![1, 3],
            });
            t
        }),
        ("task-completed", {
            let mut t = base();
            t.status = Status::Done;
            t.is_completed = true;
            t.completed_at = Some("2026-05-15T09:30:00.000Z".into());
            t
        }),
        ("task-canceled", {
            let mut t = base();
            t.status = Status::Canceled;
            t.is_cancelled = true;
            t
        }),
        ("task-significance-unset", {
            let mut t = base();
            t.significance = None;
            t
        }),
    ]
}

fn emit(name: &str, object: &Value) {
    let produced = format!("{}\n", serde_json::to_string_pretty(object).unwrap());
    let path = fixture_dir().join(format!("{name}.json"));
    if std::env::var_os("AGENDA_EMIT_FIXTURES").is_some() {
        std::fs::create_dir_all(fixture_dir()).unwrap();
        std::fs::write(&path, &produced).unwrap();
        return;
    }
    let committed = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "missing fixture {}; run with AGENDA_EMIT_FIXTURES=1",
            path.display()
        )
    });
    assert_eq!(
        produced, committed,
        "{name}: write output drifted from the committed fixture — regenerate with \
         `AGENDA_EMIT_FIXTURES=1 cargo test engine_payload_fixtures`, then copy \
         fixtures/engine/*.json into cortex \
         core/crates/ark-core/tests/fixtures/agenda/ in the same change"
    );
}

/// Every quick-entry / QA scenario payload must be deterministic, schema-shaped,
/// and byte-identical to the committed contract fixture.
#[test]
fn engine_payload_fixtures_match_write_output() {
    for (name, todo) in cases() {
        let object = mapping::write_at(Value::Null, None, &todo, NOW, &day()).unwrap();
        assert_eq!(
            object["typeVersion"],
            json!(mapping::TASK_VERSION),
            "{name}"
        );
        // The day fields must be bare dates whenever present.
        for key in ["scheduledAt", "dueAt"] {
            if let Some(value) = object["propsJson"][key].as_str() {
                assert!(
                    parse_key(value).is_some() && value.len() == 10,
                    "{name}: {key} is not a bare date: {value}"
                );
            }
        }
        emit(name, &object);
        // Edits to an existing object produce the same contract shape; the
        // KOS-295 regression (Engine-owned `dayOfMonth` leaking into
        // `recurrence`) was an edit-path write.
        let before = mapping::read(&object).unwrap();
        let mut edited = before.clone();
        edited.title = format!("{name} edited");
        let rewritten = mapping::write_at(object, Some(&before), &edited, NOW, &day()).unwrap();
        emit(&format!("{name}-edited"), &rewritten);
    }
}
