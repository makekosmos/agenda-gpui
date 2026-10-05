use super::*;

/// `reminderAt`/`completedAt` are `date-time` under com.kosmos.task 1.1.0.
#[test]
fn write_keeps_canonical_instant_when_new_value_is_a_bare_day() {
    let original = json!({"id":"t1","typeId":mapping::TASK_TYPE,"typeVersion":"1.1.0","title":"Task",
        "createdAt":"2026-09-20T10:00:00Z","updatedAt":"2026-09-20T10:00:00Z","deletedAt":null,
        "propsJson":{"status":"todo","completedAt":"2026-09-20T12:00:00Z",
            "extensions":{"completed_at":"2026-09-20T12:00:00Z"}}});
    let before = mapping::read(&original).unwrap();
    let mut edited = before.clone();
    edited.completed_at = Some("2026-09-21".into()); // malformed day-only value
    edited.reminder_date = Some("2026-09-22".into());
    let written = mapping::write(original.clone(), Some(&before), &edited).unwrap();
    assert_eq!(
        written["propsJson"]["completedAt"],
        json!("2026-09-20T12:00:00Z")
    );
    assert!(written["propsJson"]["reminderAt"].is_null()); // malformed value not written
    edited.completed_at = None; // clearing still writes null
    let cleared = mapping::write(written, Some(&before), &edited).unwrap();
    assert_eq!(cleared["propsJson"]["completedAt"], json!(null));
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
        crate::store::projects::project_object(Value::Null, &project, "2026-10-05T00:00:00Z");
    assert_eq!(object["typeId"], "com.kosmos.project");
    assert_eq!(object["propsJson"]["status"], "someday");
    assert_eq!(object["propsJson"]["dueAt"], "2026-12-01");
    assert_eq!(crate::store::projects::read_project(&object), project);
}
