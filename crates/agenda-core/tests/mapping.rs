// core.task 0.1.0 mapping (KOS-397): read/write round trips, the planning and
// status rules, instants kept on bare-day edits, checklist ids, the project
// link, and the recurrence fields the Engine owns.
use agenda_core::mapping::{self, Link, PROJECT_LINK, TASK_TYPE, TASK_VERSION};
use agenda_core::*;
use chrono::{FixedOffset, NaiveDate};
use serde_json::{json, Value};

const NOW: &str = "2026-10-13T10:00:00+03:00";

fn day() -> LocalDay {
    LocalDay::new(
        NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
        FixedOffset::east_opt(3 * 3600).unwrap(),
    )
}

fn stored(props: Value) -> Value {
    json!({"id":"t1","typeId":TASK_TYPE,"typeVersion":TASK_VERSION,"title":"Task",
        "createdAt":"2026-09-20T10:00:00Z","updatedAt":"2026-09-20T10:00:00Z","deletedAt":null,
        "contentJson":{"type":"doc","content":[{"type":"paragraph"}]},"propsJson":props})
}

fn project_link(target: &str) -> Link {
    Link {
        id: format!("lnk:t1:project:{target}"),
        kind: PROJECT_LINK.into(),
        target: target.into(),
    }
}

fn write(object: Value, before: Option<&Todo>, todo: &Todo) -> Value {
    mapping::write_at(object, before, todo, NOW, &day()).unwrap()
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
        let object = write(Value::Null, None, &todo);
        assert_eq!(object["typeId"], json!("core.task"));
        assert_eq!(object["typeVersion"], json!("0.1.0"));
        assert_eq!(mapping::read(&object, &[]).unwrap(), todo, "{status:?}");
    }
}

#[test]
fn fresh_object_carries_every_required_canonical_prop() {
    let todo = new_todo("t1", "Задача");
    let object = write(Value::Null, None, &todo);
    let props = &object["propsJson"];
    for key in [
        "state",
        "planning",
        "deadline",
        "reminders",
        "recurrence",
        "priority",
        "checklist",
        "extensions",
    ] {
        assert!(props.get(key).is_some(), "missing {key}");
    }
    assert_eq!(props["state"], json!({"kind":"open","at":null}));
    assert_eq!(props["planning"]["kind"], json!("inbox"));
}

#[test]
fn scheduled_day_survives_a_today_plan() {
    let mut todo = new_todo("t1", "Задача");
    todo.created_at = "2026-09-20T10:00:00Z".into();
    todo.status = Status::Todo;
    todo.is_today = true;
    todo.scheduled_date = Some("2026-10-14".into());
    let object = write(Value::Null, None, &todo);
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
    let written = write(original, Some(&before), &edited);
    assert_eq!(written["propsJson"]["state"]["at"], "2026-09-20T12:00:00Z");
    assert_eq!(
        written["propsJson"]["reminders"][0]["at"],
        "2026-09-21T09:00:00Z"
    );
    edited.reminder_date = None; // clearing still writes
    let cleared = write(written, Some(&before), &edited);
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
    let written = write(original, Some(&before), &edited);
    assert_eq!(
        written["propsJson"]["checklist"],
        json!([{"id":"c1","text":"Купить","done":true,"rank":"a0"}])
    );
    edited.checklist.clear();
    let emptied = write(written, Some(&before), &edited);
    assert_eq!(emptied["propsJson"]["checklist"], json!([]));
}

#[test]
fn project_is_a_link_not_a_prop() {
    let mut todo = new_todo("t1", "Задача");
    todo.project_id = Some("p1".into());
    let object = write(Value::Null, None, &todo);
    assert!(!object["propsJson"]
        .as_object()
        .unwrap()
        .contains_key("project"));
    assert!(object["propsJson"]["extensions"]
        .get("project_id")
        .is_none());
    assert_eq!(
        mapping::read(&object, &[project_link("p1")])
            .unwrap()
            .project_id
            .as_deref(),
        Some("p1")
    );
    assert_eq!(mapping::read(&object, &[]).unwrap().project_id, None);
}

#[test]
fn links_in_object_accept_both_spellings() {
    let object = json!({"links":[
        {"id":"l1","target_object_id":"p1","link_type":"project"},
        {"id":"l2","targetObjectId":"tag1","linkType":"tag"},
        {"id":"l3","target_object_id":"x"}]});
    let links = mapping::links_in_object(&object);
    assert_eq!(links.len(), 2);
    assert_eq!(links[0], project_link_with_id("l1", "p1"));
    assert_eq!(links[1].kind, "tag");
    assert!(mapping::links_in_object(&json!({})).is_empty());
}

fn project_link_with_id(id: &str, target: &str) -> Link {
    Link {
        id: id.into(),
        kind: PROJECT_LINK.into(),
        target: target.into(),
    }
}

#[test]
fn recurrence_end_and_time_zone_are_owned_by_the_engine() {
    let original = stored(json!({
        "recurrence":{"kind":"calendar","frequency":"weekly","interval":1,
            "daysOfWeek":[2],"timeZone":"Europe/Moscow",
            "end":{"kind":"count","date":null,"count":3}}}));
    let before = mapping::read(&original, &[]).unwrap();
    let mut edited = before.clone();
    edited.recurrence.as_mut().unwrap().interval = 2;
    edited.recurrence.as_mut().unwrap().recurrence_type = 1;
    let written = write(original, Some(&before), &edited);
    let rule = &written["propsJson"]["recurrence"];
    assert_eq!(rule["kind"], "afterCompletion");
    assert_eq!(rule["interval"], 2);
    assert_eq!(rule["timeZone"], "Europe/Moscow");
    assert_eq!(rule["end"], json!({"kind":"count","date":null,"count":3}));
}

#[test]
fn days_of_week_stay_within_iso_range() {
    let mut todo = new_todo("t1", "Задача");
    todo.recurrence = Some(RecurrenceRule {
        frequency: 1,
        interval: 1,
        recurrence_type: 0,
        days_of_week: vec![5, 0, 8, 1, 5],
    });
    let object = write(Value::Null, None, &todo);
    assert_eq!(
        object["propsJson"]["recurrence"]["daysOfWeek"],
        json!([1, 5])
    );
    assert_eq!(object["propsJson"]["recurrence"]["kind"], "calendar");
    assert_eq!(object["propsJson"]["recurrence"]["frequency"], "weekly");
}

#[test]
fn legacy_com_kosmos_task_objects_are_not_read_as_tasks() {
    let legacy = json!({"id":"t1","typeId":"com.kosmos.task","typeVersion":"1.1.0",
        "title":"Old","propsJson":{}});
    assert!(mapping::read(&legacy, &[]).is_err());
}
