use super::*;
use agenda_core::{is_due_this_week, LocalDay, Status};
/// `mapping::write` equivalent for tests: the app no longer has a
/// clock-reading wrapper, so tests stamp `now`/`LocalDay` here.
fn write(object: Value, before: Option<&Todo>, todo: &Todo) -> Result<Value, String> {
    mapping::write_at(
        object,
        before,
        todo,
        &chrono::Utc::now().to_rfc3339(),
        &LocalDay::now(),
    )
}

use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    process::Command as Process,
};

#[test]
fn mapping_preserves_unrepresented_fields() {
    let original = json!({"id":"real-id","typeId":mapping::TASK_TYPE,"typeVersion":"0.1.0",
        "title":"From Vue","createdAt":"2026-09-20T10:00:00Z","updatedAt":"2026-09-20T10:00:00Z",
        "contentJson":{"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Rich notes","marks":[{"type":"bold"}]}]}]},
        "propsJson":{"state":{"kind":"inProgress","at":null},
            "planning":{"kind":"date","period":"evening","date":LocalDay::now().today_key()},
            "deadline":null,"reminders":[],"recurrence":null,"priority":"high",
            "checklist":[{"id":"item-1","text":"Keep this","done":true,"rank":"a0"}],
            "extensions":{"status":"todo","linked_todo_ids":["other-task"],"foreign":{"secret":"preserved"},
                "tag_ids":["real-tag"],"significance":7,"fuel_cost":12.5,"price":150.75}},"deletedAt":null});
    let links = [mapping::Link {
        id: "lnk:real-id:project:real-project".into(),
        kind: mapping::PROJECT_LINK.into(),
        target: "real-project".into(),
    }];
    let before = mapping::read(&original, &links).unwrap();
    assert_eq!(before.status, Status::Started);
    assert_eq!(before.fuel_cost, Some(12.5));
    assert_eq!(before.price, Some(150.75));
    assert_eq!(before.notes.as_deref(), Some("Rich notes"));
    assert_eq!(before.project_id.as_deref(), Some("real-project"));
    let day = LocalDay::now();
    assert!(is_due_this_week(&before, &day.week_bounds().unwrap(), &day));
    let mut edited = before.clone();
    edited.title = "Renamed".into();
    let written = write(original.clone(), Some(&before), &edited).unwrap();
    assert_eq!(written["contentJson"], original["contentJson"]);
    assert_eq!(written["propsJson"], original["propsJson"]);
    assert_eq!(mapping::read(&written, &links).unwrap(), edited);
    edited.status = Status::Deferred;
    let deferred = write(written, Some(&before), &edited).unwrap();
    assert_eq!(
        mapping::read(&deferred, &links).unwrap().status,
        Status::Deferred
    );
    edited.status = Status::Todo;
    let resumed = write(deferred, None, &edited).unwrap();
    assert_eq!(
        mapping::read(&resumed, &links).unwrap().status,
        Status::Todo
    );
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
                // Links are kept beside objects; only links carry `linkType`.
                "list_object_links" => json!({"ok":true,"data":objects.values().filter(|v| v.get("linkType").is_some()).collect::<Vec<_>>()}),
                "upsert_object_link" => {
                    let link = &request["object_link"];
                    objects.insert(link["id"].as_str().unwrap().into(), link.clone());
                    std::fs::write(&path, serde_json::to_vec(&objects).unwrap()).unwrap();
                    json!({"ok":true,"data":true})
                }
                "delete_object_link" => {
                    objects.remove(request["id"].as_str().unwrap());
                    std::fs::write(&path, serde_json::to_vec(&objects).unwrap()).unwrap();
                    json!({"ok":true,"data":true})
                }
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
        "project",
    ] {
        let output = Process::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "store::restart_tests::restart_client",
                "--nocapture",
            ])
            .env("MUNDUS_DATA_DIR", &directory)
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

/// Error classification keeps the real Engine code and maps to a class.
#[test]
fn engine_error_codes_map_to_kinds() {
    for (raw, kind) in [
        (
            "canonical_ingress:invalid_request:canonical_field:/recurrence/dayOfMonth",
            ErrorKind::InvalidRequest,
        ),
        ("invalid-request", ErrorKind::InvalidRequest),
        ("forbidden", ErrorKind::Forbidden),
        ("object_conflict:stale_snapshot", ErrorKind::Conflict),
        ("conflict", ErrorKind::Conflict),
        ("not-found", ErrorKind::NotFound),
        ("timeout", ErrorKind::Timeout),
        ("Engine RPC timed out", ErrorKind::Unknown),
        ("unavailable", ErrorKind::Unavailable),
        ("some future engine error", ErrorKind::Unknown),
    ] {
        assert_eq!(ErrorKind::from_engine_code(raw), kind, "{raw}");
    }
    for kind in [
        ErrorKind::NotRunning,
        ErrorKind::NotCompatible,
        ErrorKind::Transport,
        ErrorKind::InvalidRequest,
        ErrorKind::Forbidden,
        ErrorKind::Conflict,
        ErrorKind::NotFound,
        ErrorKind::Timeout,
        ErrorKind::Unavailable,
        ErrorKind::Malformed,
        ErrorKind::Unknown,
    ] {
        let e = EngineError {
            kind,
            detail: "x".into(),
        };
        assert!(!e.message().is_empty(), "{kind:?} needs user text");
    }
}
