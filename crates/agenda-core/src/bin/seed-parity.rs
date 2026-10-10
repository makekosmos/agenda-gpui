//! Golden dump for the Android read-parity gate (KOS-370): prints the seed
//! dataset as Engine objects plus the expected contents of every smart
//! list, project view and tag filter for a fixed `today`, all computed by
//! the same agenda-core rules desktop runs. Usage:
//!   cargo run -p agenda-core --bin seed-parity -- <YYYY-MM-DD> <utc-offset-secs>
use agenda_core::{
    filter_idx, mapping, project_page_idx, projects, seed, tag_page_idx, today_idx, LocalDay,
    SmartList,
};
use chrono::{FixedOffset, NaiveDate};
use serde_json::{json, Value};

fn ids(todos: &[agenda_core::Todo], idx: Vec<usize>) -> Vec<String> {
    idx.into_iter().map(|i| todos[i].id.clone()).collect()
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let today = NaiveDate::parse_from_str(
        &args
            .next()
            .ok_or("usage: seed-parity <YYYY-MM-DD> <offset-secs>")?,
        "%Y-%m-%d",
    )
    .map_err(|e| format!("bad today: {e}"))?;
    let offset: i32 = args
        .next()
        .ok_or("usage: seed-parity <YYYY-MM-DD> <offset-secs>")?
        .parse()
        .map_err(|e| format!("bad offset: {e}"))?;
    let day = LocalDay::new(
        today,
        FixedOffset::east_opt(offset).ok_or("offset out of range")?,
    );
    let now = format!("{}T12:00:00{:+03}:{:02}", day.today_key(), 0, 0);

    // Instant fields (`reminder_date`, `completed_at`) must carry an
    // explicit offset to survive the Engine's canonical normalization —
    // `iso_at` deliberately emits bare local stamps, so append the dump's
    // own offset like a real writer on this box would have.
    let suffix = format!(
        "{}{:02}:{:02}",
        if offset < 0 { "-" } else { "+" },
        offset.abs() / 3600,
        offset.abs() % 3600 / 60
    );
    let fix = |v: Option<String>| {
        v.map(|s| {
            if chrono::DateTime::parse_from_rfc3339(&s).is_ok() {
                s
            } else {
                format!("{s}{suffix}")
            }
        })
    };
    let mut todos = seed::seed_todos_at(&day);
    for t in &mut todos {
        t.reminder_date = fix(t.reminder_date.take());
        t.completed_at = fix(t.completed_at.take());
    }
    let todos = todos;
    let projs = seed::seed_projects_at(&day);
    let tags = seed::seed_tags();

    let mut objects: Vec<Value> = Vec::new();
    for t in &todos {
        objects.push(mapping::write_at(Value::Null, None, t, &now, &day)?);
    }
    for p in &projs {
        objects.push(projects::project_object(&Value::Null, p, &now));
    }
    objects.push(projects::references_object(&[], &tags, &now));

    let mut lists = serde_json::Map::new();
    for (name, list) in [
        ("inbox", SmartList::Inbox),
        ("week", SmartList::Week),
        ("nextWeek", SmartList::NextWeek),
        ("someday", SmartList::Someday),
        ("logbook", SmartList::Logbook),
        ("trash", SmartList::Trash),
    ] {
        lists.insert(
            name.into(),
            json!(ids(&todos, filter_idx(list, &todos, &day))),
        );
    }
    lists.insert("today".into(), json!(ids(&todos, today_idx(&todos, &day))));

    let mut proj_views = serde_json::Map::new();
    for p in &projs {
        proj_views.insert(
            p.id.clone(),
            json!(ids(&todos, project_page_idx(&p.id, &todos, &day))),
        );
    }
    let mut tag_views = serde_json::Map::new();
    for tag in &tags {
        tag_views.insert(
            tag.id.clone(),
            json!(ids(&todos, tag_page_idx(&tag.id, &todos))),
        );
    }

    let out = json!({
        "today": day.today_key(),
        "utcOffsetSeconds": offset,
        "objects": objects,
        "todos": todos,
        "projects": projs,
        "tags": tags,
        "expected": {
            "lists": lists,
            "projects": proj_views,
            "tags": tag_views,
        },
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&out).map_err(|e| e.to_string())?
    );
    Ok(())
}
