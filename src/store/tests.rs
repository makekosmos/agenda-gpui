use super::*;
use crate::model::{is_due_today, is_inbox, new_todo, today_key, Status};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    process::Command as Process,
};

#[test]
fn mapping_preserves_unrepresented_fields() {
    let original = json!({"id":"real-id","typeId":mapping::TASK_TYPE,"typeVersion":"1.0.0",
        "title":"From Vue","createdAt":"2026-09-20T10:00:00Z","updatedAt":"2026-09-20T10:00:00Z",
        "contentJson":{"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Rich notes","marks":[{"type":"bold"}]}]}]},
        "propsJson":{"status":"inProgress","priority":"high","scheduledAt":today_key(),
            "checklist":[{"id":"item-1","title":"Keep this","isCompleted":true}],
            "extensions":{"priority":3,"linked_todo_ids":["other-task"],"foreign":{"secret":"preserved"},
                "project_id":"real-project","tag_ids":["real-tag"],"significance":7,"fuel_cost":12.5,"price":150.75}},"deletedAt":null});
    let before = mapping::read(&original).unwrap();
    assert_eq!(before.status, Status::Started);
    assert_eq!(before.fuel_cost, Some(12.5));
    assert_eq!(before.price, Some(150.75));
    assert_eq!(before.notes.as_deref(), Some("Rich notes"));
    assert_eq!(before.project_id.as_deref(), Some("real-project"));
    assert!(is_due_today(&before, &today_key()));
    let mut edited = before.clone();
    edited.title = "Renamed".into();
    let written = mapping::write(original.clone(), Some(&before), &edited).unwrap();
    assert_eq!(written["contentJson"], original["contentJson"]);
    assert_eq!(written["propsJson"], original["propsJson"]);
    assert_eq!(mapping::read(&written).unwrap(), edited);
    edited.status = Status::Deferred;
    let deferred = mapping::write(written, Some(&before), &edited).unwrap();
    assert_eq!(mapping::read(&deferred).unwrap().status, Status::Deferred);
    edited.status = Status::Todo;
    let resumed = mapping::write(deferred, None, &edited).unwrap();
    assert_eq!(mapping::read(&resumed).unwrap().status, Status::Todo);
}

// Called in separate OS processes below: no shared in-memory task state.
#[test]
#[ignore]
fn restart_client() {
    let Ok(mode) = std::env::var("AGENDA_TEST_CLIENT") else {
        return;
    };
    let engine = Engine::default();
    let tasks = engine.load().unwrap().todos;
    match mode.as_str() {
        "create" => {
            assert!(tasks.is_empty());
            let mut todo = new_todo("restart-test", "Before restart");
            todo.created_at = "2026-09-20T10:00:00Z".into();
            engine.save(None, &todo).unwrap();
        }
        "edit" => {
            assert_eq!(tasks.len(), 1);
            let before = &tasks[0];
            assert_eq!(before.title, "Before restart");
            assert!(is_inbox(before));
            let mut todo = before.clone();
            todo.title = "After restart".into();
            todo.notes = Some("Saved notes".into());
            todo.status = Status::Todo;
            todo.scheduled_date = Some(today_key());
            engine.save(Some(before), &todo).unwrap();
        }
        "complete" => {
            let before = &tasks[0];
            assert_eq!(before.title, "After restart");
            assert_eq!(before.notes.as_deref(), Some("Saved notes"));
            assert!(is_due_today(before, &today_key()));
            let mut todo = before.clone();
            todo.status = Status::Done;
            todo.is_completed = true;
            todo.completed_at = Some("2026-09-20T12:00:00Z".into());
            engine.save(Some(before), &todo).unwrap();
        }
        "trash" => {
            let before = &tasks[0];
            assert!(before.is_completed);
            let mut todo = before.clone();
            todo.is_trashed = true;
            engine.save(Some(before), &todo).unwrap();
        }
        "restore" => {
            let before = &tasks[0];
            assert!(before.is_trashed);
            let mut todo = before.clone();
            todo.is_trashed = false;
            engine.save(Some(before), &todo).unwrap();
        }
        "verify" => {
            assert!(tasks[0].is_completed);
            assert!(!tasks[0].is_trashed);
        }
        "rejected" => {
            let before = &tasks[0];
            let mut todo = before.clone();
            todo.title = "REJECT".into();
            assert!(engine.save(Some(before), &todo).is_err());
            assert_eq!(engine.load().unwrap().todos[0].title, "After restart");
        }
        "conflict" => {
            let before = &tasks[0];
            let mut other = before.clone();
            other.title = "Edited in Vue".into();
            engine.save(Some(before), &other).unwrap();
            let mut stale = before.clone();
            stale.title = "Stale edit".into();
            assert!(engine.save(Some(before), &stale).is_err());
            assert_eq!(engine.load().unwrap().todos[0].title, "Edited in Vue");
        }
        "recurrence" => {
            let mut before = tasks[0].clone();
            before.status = Status::Todo;
            before.is_completed = false;
            before.completed_at = None;
            engine.save(Some(&tasks[0]), &before).unwrap();
            let mut done = before.clone();
            done.status = Status::Done;
            done.is_completed = true;
            done.completed_at = Some("2026-09-21T12:00:00Z".into());
            let mut next = before.clone();
            next.id = "restart-next".into();
            next.title = "REJECT".into();
            assert!(engine
                .save_with_next(Some(&before), &done, Some(&next))
                .is_err());
            assert_eq!(engine.load().unwrap().todos, vec![before.clone()]);
            next.title = "Next occurrence".into();
            engine
                .save_with_next(Some(&before), &done, Some(&next))
                .unwrap();
            let persisted = engine.load().unwrap().todos;
            assert!(persisted.contains(&done));
            assert!(persisted.contains(&next));
        }
        _ => panic!("unknown test mode"),
    }
}

#[test]
fn persistence_across_client_processes() {
    let directory =
        std::env::temp_dir().join(format!("agenda-engine-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::fs::write(
        directory.join("engine.lock.json"),
        json!({"format_version":1,
        "api_version":{"major":1},"http_port":port,"auth_token":"a".repeat(64)})
        .to_string(),
    )
    .unwrap();
    let path = directory.join("fixture-store.json");
    std::fs::write(&path, "{}").unwrap();
    let server = std::thread::spawn(move || {
        for connection in listener.incoming() {
            let mut socket = connection.unwrap();
            let mut reader = BufReader::new(socket.try_clone().unwrap());
            let mut first = String::new();
            reader.read_line(&mut first).unwrap();
            if first.contains("/stop") {
                break;
            }
            assert!(first.starts_with("POST /v1/rpc "));
            let mut length = 0;
            let mut authenticated = false;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                if let Some(value) = line.to_lowercase().strip_prefix("content-length:") {
                    length = value.trim().parse().unwrap();
                }
                if line.trim() == format!("Authorization: Bearer {}", "a".repeat(64)) {
                    authenticated = true;
                }
            }
            assert!(authenticated);
            let mut bytes = vec![0; length];
            reader.read_exact(&mut bytes).unwrap();
            let request: Value = serde_json::from_slice(&bytes).unwrap();
            let mut objects: serde_json::Map<String, Value> =
                serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            let response = match request["operation"].as_str().unwrap() {
                "list_objects_by_type" => json!({"ok":true,"data":objects.values().filter(|v| v["typeId"] == request["type_id"]).collect::<Vec<_>>()}),
                "get_object" => json!({"ok":true,"data":objects.get(request["id"].as_str().unwrap())}),
                "upsert_object" if request["object"]["title"] == "REJECT" => json!({"ok":false,"error":"rejected"}),
                "upsert_object" => {
                    let object = &request["object"];
                    objects.insert(object["id"].as_str().unwrap().into(), object.clone());
                    std::fs::write(&path, serde_json::to_vec(&objects).unwrap()).unwrap();
                    json!({"ok":true,"data":true})
                }
                _ => panic!("unexpected operation"),
            }.to_string();
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", response.len(), response).unwrap();
        }
    });
    for mode in [
        "create",
        "edit",
        "complete",
        "trash",
        "restore",
        "verify",
        "rejected",
        "conflict",
        "recurrence",
    ] {
        let output = Process::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "store::tests::restart_client",
                "--nocapture",
            ])
            .env("KOSMOS_DATA_DIR", &directory)
            .env("AGENDA_TEST_CLIENT", mode)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{mode}: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let mut stop = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    stop.write_all(b"GET /stop HTTP/1.1\r\n\r\n").unwrap();
    server.join().unwrap();
    std::fs::remove_dir_all(directory).unwrap();
}
