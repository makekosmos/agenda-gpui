use super::*;
use crate::model::{new_todo, Status};

const NOW: &str = "2026-10-13T10:00:00.000Z";

fn stored(props: Value) -> Value {
    json!({"id":"t1","typeId":mapping::TASK_TYPE,"typeVersion":"0.1.0","title":"Task",
        "createdAt":"2026-09-20T10:00:00Z","updatedAt":"2026-09-20T10:00:00Z","deletedAt":null,
        "contentJson":{"type":"doc","content":[{"type":"paragraph"}]},"propsJson":props})
}

fn project_link(project: &str) -> mapping::Link {
    mapping::Link {
        id: format!("lnk:t1:project:{project}"),
        kind: mapping::PROJECT_LINK.into(),
        target: project.into(),
    }
}

#[test]
fn read_maps_core_task_state_planning_and_links() {
    let object = stored(json!({
        "state":{"kind":"done","at":"2026-09-20T12:00:00Z"},
        "planning":{"kind":"today","period":"evening","date":null},
        "deadline":"2026-09-22",
        "reminders":[{"kind":"floating","at":"2026-09-21T09:00:00Z","timeZone":null}],
        "priority":"urgent",
        "checklist":[{"id":"c1","text":"Шаг","done":true,"rank":"000000"}],
        "extensions":{"description":"заметка","tag_ids":["g1"],"sort_order":4}}));
    let todo = mapping::read(&object, &[project_link("p1")]).unwrap();
    assert_eq!(todo.status, Status::Done);
    assert!(todo.is_completed && todo.is_today && !todo.is_someday);
    assert_eq!(todo.completed_at.as_deref(), Some("2026-09-20T12:00:00Z"));
    assert_eq!(todo.project_id.as_deref(), Some("p1"));
    assert_eq!(todo.notes.as_deref(), Some("заметка"));
    assert_eq!(todo.sort_order, 4);
}

#[test]
fn read_planning_kinds_map_to_status_and_schedule() {
    let inbox = mapping::read(&stored(json!({"planning":{"kind":"inbox"}})), &[]).unwrap();
    assert_eq!(inbox.status, Status::Inbox);
    let dated = stored(json!({"planning":{"kind":"date","period":"any","date":"2026-09-25"}}));
    let dated = mapping::read(&dated, &[]).unwrap();
    assert_eq!(dated.scheduled_date.as_deref(), Some("2026-09-25"));
    assert_eq!(dated.status, Status::Todo);
    let deferred =
        stored(json!({"planning":{"kind":"someday"},"extensions":{"status":"deferred"}}));
    let deferred = mapping::read(&deferred, &[]).unwrap();
    assert_eq!(deferred.status, Status::Todo);
    assert!(deferred.is_someday);
}

#[test]
fn legacy_states_read_as_two_states_and_survive_edits() {
    let read = |state: &str, ext: Value| {
        let object = stored(json!({"state":{"kind":state,"at":null},
            "planning":{"kind":"anytime"},"extensions":ext}));
        (object.clone(), mapping::read(&object, &[]).unwrap())
    };
    // Canceled is closed: done, but no completion stamp for statistics.
    let (canceled, todo) = read("canceled", json!({}));
    assert_eq!(todo.status, Status::Done);
    assert!(todo.is_completed && todo.completed_at.is_none());
    let mut moved = todo.clone();
    moved.scheduled_date = Some("2026-10-30".into());
    let written = mapping::write_at(canceled, Some(&todo), &moved, NOW).unwrap();
    assert_eq!(written["propsJson"]["state"]["kind"], "canceled");
    // In progress is open; reopening keeps the stored kind until completed.
    let (started, todo) = read("inProgress", json!({}));
    assert_eq!(todo.status, Status::Todo);
    assert!(!todo.is_completed);
    let mut moved = todo.clone();
    moved.is_today = true;
    let written = mapping::write_at(started, Some(&todo), &moved, NOW).unwrap();
    assert_eq!(written["propsJson"]["state"]["kind"], "inProgress");
    // Old «deferred» status without a someday plan still lands in «Потом».
    let (_, todo) = read("open", json!({"status":"deferred"}));
    assert!(todo.is_someday && todo.status == Status::Todo);
}

#[test]
fn write_then_read_round_trips_every_status() {
    for status in [Status::Inbox, Status::Todo, Status::Done] {
        let mut todo = new_todo("t1", "Задача");
        todo.created_at = "2026-09-20T10:00:00Z".into();
        todo.status = status;
        todo.is_completed = status == Status::Done;
        todo.completed_at = (status == Status::Done).then(|| "2026-10-12T09:30:00Z".to_owned());
        todo.notes = Some("заметка".into());
        let object = mapping::write_at(Value::Null, None, &todo, NOW).unwrap();
        assert_eq!(object["typeVersion"], json!(mapping::TASK_VERSION));
        assert_eq!(mapping::read(&object, &[]).unwrap(), todo, "{status:?}");
    }
}

#[test]
fn scheduled_day_survives_a_today_plan() {
    let mut todo = new_todo("t1", "Задача");
    todo.created_at = "2026-09-20T10:00:00Z".into();
    todo.status = Status::Todo;
    todo.is_today = true;
    todo.scheduled_date = Some("2026-10-14".into());
    let object = mapping::write_at(Value::Null, None, &todo, NOW).unwrap();
    assert_eq!(object["propsJson"]["planning"]["kind"], "today");
    assert_eq!(
        object["propsJson"]["extensions"]["scheduled_date"],
        "2026-10-14"
    );
    assert_eq!(mapping::read(&object, &[]).unwrap(), todo);
}

#[test]
fn write_keeps_stored_instant_when_the_edit_is_a_bare_day() {
    let original = stored(json!({
        "state":{"kind":"done","at":"2026-09-20T12:00:00Z"},
        "planning":{"kind":"anytime","period":"any","date":null}}));
    let before = mapping::read(&original, &[]).unwrap();
    let mut edited = before.clone();
    edited.completed_at = Some("2026-09-21".into()); // malformed day-only value
    let written = mapping::write(original, Some(&before), &edited).unwrap();
    assert_eq!(written["propsJson"]["state"]["at"], "2026-09-20T12:00:00Z");
}

/// Agenda does not show these fields, so an edit must leave them as stored.
#[test]
fn write_keeps_fields_agenda_does_not_show() {
    let kept = json!({
        "deadline":"2026-09-22",
        "reminders":[{"kind":"floating","at":"2026-09-21T09:00:00Z","timeZone":null}],
        "priority":"high",
        "checklist":[{"id":"c1","text":"Купить","done":false,"rank":"a0"}],
        "recurrence":{"kind":"calendar","frequency":"weekly","interval":1,"daysOfWeek":[1]}});
    let mut props = kept.clone();
    props["planning"] = json!({"kind":"anytime","period":"evening","date":null});
    let original = stored(props);
    let before = mapping::read(&original, &[]).unwrap();
    let mut edited = before.clone();
    edited.title = "Другая".into();
    edited.is_today = true;
    let written = mapping::write(original, Some(&before), &edited).unwrap();
    for key in [
        "deadline",
        "reminders",
        "priority",
        "checklist",
        "recurrence",
    ] {
        assert_eq!(written["propsJson"][key], kept[key], "{key}");
    }
    assert_eq!(written["propsJson"]["planning"]["kind"], "today");
    assert_eq!(written["propsJson"]["planning"]["period"], "evening");
}
