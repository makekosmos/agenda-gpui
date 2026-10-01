use super::*;
use crate::model::{day_key, new_todo, today_key, RecurrenceRule, Status, Todo};

// ---------------------------------------------------------------------------
// Canonical `com.kosmos.task` schema conformance (KOS-295).
//
// The Engine validates `propsJson` against the type's JSON-schema subset
// (cortex ark-core `canonical_types::validation`). Agenda can't depend on
// ark-core ("apps → Engine only"), so this is a faithful port of the subset
// Engine enforces — type / required / properties / additionalProperties /
// enum / minimum / maximum / minLength / uniqueItems. `format` is checked for
// shape in the schema itself but NOT enforced on values, matching Engine.
// ---------------------------------------------------------------------------

/// `schema` of `com.kosmos.task` 1.0.0 — byte copy of cortex
/// `canonical_types/definitions/part02.rs`.
const TASK_SCHEMA: &str = r#"{
  "type": "object",
  "additionalProperties": false,
  "properties": {
    "status": {"type": "string", "enum": ["inbox","todo","inProgress","done","canceled"]},
    "priority": {"type": "string", "enum": ["none","low","medium","high","urgent"]},
    "scheduledAt": {"type": ["string","null"], "format": "date-time"},
    "dueAt": {"type": ["string","null"], "format": "date-time"},
    "reminderAt": {"type": ["string","null"], "format": "date-time"},
    "completedAt": {"type": ["string","null"], "format": "date-time"},
    "canceledAt": {"type": ["string","null"], "format": "date-time"},
    "recurrence": {
      "type": ["object","null"],
      "additionalProperties": false,
      "properties": {
        "frequency": {"type": "string", "enum": ["daily","weekly","monthly","yearly"]},
        "interval": {"type": "integer", "minimum": 1},
        "recurrenceType": {"type": "string", "enum": ["fixed","afterCompletion"]},
        "daysOfWeek": {"type": ["array","null"], "items": {"type": "integer", "minimum": 0, "maximum": 6}, "uniqueItems": true},
        "endDate": {"type": ["string","null"], "format": "date-time"}
      },
      "required": ["frequency","interval","recurrenceType","daysOfWeek","endDate"]
    },
    "checklist": {
      "type": "array",
      "items": {
        "type": "object",
        "additionalProperties": false,
        "properties": {
          "id": {"type": "string", "minLength": 1},
          "title": {"type": "string"},
          "isCompleted": {"type": "boolean"}
        },
        "required": ["id","title","isCompleted"]
      }
    },
    "extensions": {"type": "object"}
  },
  "required": ["status","priority","scheduledAt","dueAt","reminderAt","completedAt","canceledAt","recurrence","checklist","extensions"]
}"#;

fn validate_schema(schema: &Value, value: &Value, pointer: &str, open: bool) -> Result<(), String> {
    let object = schema.as_object().unwrap();
    if let Some(types) = object.get("type") {
        let matches_type = |name: &str| match name {
            "object" => value.is_object(),
            "array" => value.is_array(),
            "string" => value.is_string(),
            "number" => value.is_number(),
            "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
            "boolean" => value.is_boolean(),
            "null" => value.is_null(),
            _ => false,
        };
        let ok = types.as_str().map(matches_type).unwrap_or_else(|| {
            types
                .as_array()
                .map(|a| a.iter().filter_map(Value::as_str).any(matches_type))
                .unwrap_or(false)
        });
        if !ok {
            return Err(format!("{pointer}: type"));
        }
        if value.is_null() {
            return Ok(());
        }
    }
    if let Some(values) = object.get("enum").and_then(Value::as_array) {
        if !values.iter().any(|candidate| candidate == value) {
            return Err(format!("{pointer}: enum"));
        }
    }
    if let Some(min) = object.get("minimum").and_then(Value::as_f64) {
        if value.as_f64().is_some_and(|n| n < min) {
            return Err(format!("{pointer}: minimum"));
        }
    }
    if let Some(max) = object.get("maximum").and_then(Value::as_f64) {
        if value.as_f64().is_some_and(|n| n > max) {
            return Err(format!("{pointer}: maximum"));
        }
    }
    if let Some(min) = object.get("minLength").and_then(Value::as_u64) {
        if value
            .as_str()
            .is_some_and(|s| s.chars().count() < min as usize)
        {
            return Err(format!("{pointer}: minLength"));
        }
    }
    if let Some(required) = object.get("required").and_then(Value::as_array) {
        let map = value.as_object().ok_or(format!("{pointer}: type"))?;
        for name in required.iter().filter_map(Value::as_str) {
            if !map.contains_key(name) {
                return Err(format!("{pointer}/{name}: required"));
            }
        }
    }
    if let Some(map) = value.as_object() {
        if let Some(properties) = object.get("properties").and_then(Value::as_object) {
            for (name, sub) in properties {
                if let Some(field) = map.get(name) {
                    validate_schema(
                        sub,
                        field,
                        &format!("{pointer}/{name}"),
                        name == "extensions",
                    )?;
                }
            }
            if !open && object.get("additionalProperties").and_then(Value::as_bool) == Some(false) {
                for name in map.keys() {
                    if !properties.contains_key(name) {
                        return Err(format!("{pointer}/{name}: additionalProperties"));
                    }
                }
            }
        }
    }
    if let Some(items) = object.get("items") {
        if let Some(values) = value.as_array() {
            for (index, item) in values.iter().enumerate() {
                validate_schema(items, item, &format!("{pointer}/{index}"), false)?;
            }
            if object.get("uniqueItems").and_then(Value::as_bool) == Some(true) {
                for i in 0..values.len() {
                    if values[..i].iter().any(|prior| prior == &values[i]) {
                        return Err(format!("{pointer}: uniqueItems"));
                    }
                }
            }
        }
    }
    Ok(())
}

/// What Engine checks on `upsert_object` for `com.kosmos.task` 1.0.0:
/// propsJson against the schema, contentJson as rich text (root `doc`).
fn engine_accepts(object: &Value) -> Result<(), String> {
    let schema: Value = serde_json::from_str(TASK_SCHEMA).unwrap();
    validate_schema(&schema, &object["propsJson"], "", false)?;
    if object["contentJson"]["type"] != "doc" {
        return Err("contentJson: rootType".into());
    }
    Ok(())
}

/// Every quick-entry / QA scenario's `propsJson` must pass Engine validation.
/// Regression for KOS-295: `recurrence` carried a `dayOfMonth` key that the
/// closed schema rejects, and any rejected upsert used to wedge storage.
#[test]
fn mapping_write_passes_task_schema() {
    let base = || new_todo("t", "Задача");
    let cases: Vec<(&str, Todo)> = vec![
        ("inbox", base()),
        ("today", {
            let mut t = base();
            t.scheduled_date = Some(today_key());
            t.status = Status::Todo;
            t
        }),
        ("project", {
            let mut t = base();
            t.project_id = Some("p1".into());
            t.status = Status::Todo;
            t
        }),
        ("notes", {
            let mut t = base();
            t.notes = Some("заметка".into());
            t
        }),
        ("deadline", {
            let mut t = base();
            t.deadline = Some(day_key(3));
            t
        }),
        ("recurrence", {
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
        ("significance-unset", {
            let mut t = base();
            t.significance = None;
            t
        }),
    ];
    for (name, todo) in cases {
        let object = mapping::write(Value::Null, None, &todo).unwrap();
        engine_accepts(&object).unwrap_or_else(|e| panic!("{name}: {e}"));
        // Edits to an existing object must stay valid too.
        let before = mapping::read(&object).unwrap();
        let mut edited = before.clone();
        edited.title = format!("{name} edited");
        let rewritten = mapping::write(object, Some(&before), &edited).unwrap();
        engine_accepts(&rewritten).unwrap_or_else(|e| panic!("{name} edit: {e}"));
    }
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
        ("Engine RPC timed out", ErrorKind::Timeout),
        ("unavailable", ErrorKind::Unavailable),
        ("some future engine error", ErrorKind::Unavailable),
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
    ] {
        let e = EngineError {
            kind,
            detail: "x".into(),
        };
        assert!(!e.message().is_empty(), "{kind:?} needs user text");
    }
}
