use super::*;
use crate::model::{new_todo, ChecklistItem, RecurrenceRule, Status};

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
    assert!(todo.is_completed && todo.is_today && todo.is_evening && !todo.is_someday);
    assert_eq!(todo.completed_at.as_deref(), Some("2026-09-20T12:00:00Z"));
    assert_eq!(todo.deadline.as_deref(), Some("2026-09-22"));
    assert_eq!(todo.reminder_date.as_deref(), Some("2026-09-21T09:00:00Z"));
    assert_eq!(todo.priority, 3);
    assert_eq!(todo.project_id.as_deref(), Some("p1"));
    assert_eq!(todo.tag_ids, vec!["g1".to_string()]);
    assert_eq!(todo.notes.as_deref(), Some("заметка"));
    assert_eq!(todo.checklist, vec![ChecklistItem { is_completed: true }]);
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
    assert_eq!(deferred.status, Status::Deferred);
    assert!(deferred.is_someday);
}

#[test]
fn write_then_read_round_trips_every_status() {
    for status in [
        Status::Inbox,
        Status::Todo,
        Status::Started,
        Status::Deferred,
        Status::Done,
        Status::Canceled,
    ] {
        let mut todo = new_todo("t1", "Задача");
        todo.created_at = "2026-09-20T10:00:00Z".into();
        todo.status = status;
        todo.is_completed = status == Status::Done;
        todo.is_cancelled = status == Status::Canceled;
        todo.completed_at = (status == Status::Done).then(|| "2026-10-12T09:30:00Z".to_owned());
        todo.priority = 2;
        todo.deadline = Some("2026-10-20".into());
        todo.reminder_date = Some("2026-10-19T08:00:00Z".into());
        todo.notes = Some("заметка".into());
        todo.tag_ids = vec!["g1".into()];
        todo.recurrence = Some(RecurrenceRule {
            frequency: 1,
            interval: 2,
            recurrence_type: 0,
            days_of_week: vec![1, 3],
        });
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
fn write_keeps_stored_instants_when_the_edit_is_a_bare_day() {
    let original = stored(json!({
        "state":{"kind":"done","at":"2026-09-20T12:00:00Z"},
        "planning":{"kind":"anytime","period":"any","date":null},
        "reminders":[{"kind":"floating","at":"2026-09-21T09:00:00Z","timeZone":null}]}));
    let before = mapping::read(&original, &[]).unwrap();
    let mut edited = before.clone();
    edited.completed_at = Some("2026-09-21".into()); // malformed day-only value
    edited.reminder_date = Some("2026-09-22".into());
    let written = mapping::write(original, Some(&before), &edited).unwrap();
    assert_eq!(written["propsJson"]["state"]["at"], "2026-09-20T12:00:00Z");
    assert_eq!(
        written["propsJson"]["reminders"][0]["at"],
        "2026-09-21T09:00:00Z"
    );
    edited.reminder_date = None; // clearing still writes
    let cleared = mapping::write(written, Some(&before), &edited).unwrap();
    assert_eq!(cleared["propsJson"]["reminders"], json!([]));
}

#[test]
fn checklist_keeps_engine_ids_and_only_toggles_done() {
    let original =
        stored(json!({"checklist":[{"id":"c1","text":"Купить","done":false,"rank":"a0"}]}));
    let before = mapping::read(&original, &[]).unwrap();
    assert_eq!(
        before.checklist,
        vec![ChecklistItem {
            is_completed: false
        }]
    );
    let mut edited = before.clone();
    edited.checklist[0].is_completed = true;
    let written = mapping::write(original, Some(&before), &edited).unwrap();
    assert_eq!(
        written["propsJson"]["checklist"],
        json!([{"id":"c1","text":"Купить","done":true,"rank":"a0"}])
    );
    edited.checklist.clear();
    let emptied = mapping::write(written, Some(&before), &edited).unwrap();
    assert_eq!(emptied["propsJson"]["checklist"], json!([]));
}
