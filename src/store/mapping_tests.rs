use super::*;
use agenda_core::Status;

/// core.task 0.1.0 keeps the completion instant in `state.at` and reminders in
/// `reminders[].at`; both must be real RFC 3339 instants, never bare days.
#[test]
fn write_keeps_canonical_instant_when_new_value_is_a_bare_day() {
    let original = json!({"id":"t1","typeId":mapping::TASK_TYPE,"typeVersion":"0.1.0","title":"Task",
        "createdAt":"2026-09-20T10:00:00Z","updatedAt":"2026-09-20T10:00:00Z","deletedAt":null,
        "propsJson":{"state":{"kind":"done","at":"2026-09-20T12:00:00Z"},
            "planning":{"kind":"anytime","period":"any","date":null},
            "extensions":{"status":"done"}}});
    let before = mapping::read(&original, &[]).unwrap();
    assert_eq!(before.completed_at.as_deref(), Some("2026-09-20T12:00:00Z"));
    let mut edited = before.clone();
    edited.completed_at = Some("2026-09-21".into()); // malformed day-only value
    edited.reminder_date = Some("2026-09-22".into());
    let written = mapping::write_at(
        original.clone(),
        Some(&before),
        &edited,
        &chrono::Utc::now().to_rfc3339(),
        &LocalDay::now(),
    )
    .unwrap();
    assert_eq!(
        written["propsJson"]["state"],
        json!({"kind":"done","at":"2026-09-20T12:00:00Z"})
    );
    assert_eq!(written["propsJson"]["reminders"], json!([])); // malformed value not written
    edited.status = Status::Todo; // reopening and clearing the instant writes null
    edited.is_completed = false;
    edited.completed_at = None;
    let cleared = mapping::write_at(
        written,
        Some(&before),
        &edited,
        &chrono::Utc::now().to_rfc3339(),
        &LocalDay::now(),
    )
    .unwrap();
    assert_eq!(
        cleared["propsJson"]["state"],
        json!({"kind":"open","at":null})
    );
}

#[test]
fn project_round_trips_as_canonical_object() {
    let project = Project {
        id: "p1".into(),
        title: "Ремонт".into(),
        status: 1,
        deadline: Some("2026-12-01".into()),
        sort_order: 3,
        area_id: Some("a1".into()),
    };
    let object =
        agenda_core::projects::project_object(&Value::Null, &project, "2026-10-05T00:00:00Z");
    assert_eq!(object["typeId"], "com.kosmos.project");
    assert_eq!(object["propsJson"]["status"], "someday");
    assert_eq!(object["propsJson"]["dueAt"], "2026-12-01");
    assert_eq!(agenda_core::projects::read_project(&object), project);
}
