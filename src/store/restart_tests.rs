use super::*;
use crate::model::{is_due_this_week, is_inbox, new_todo, today_key, week_bounds, Status};

// Run as a child process by `tests::persistence_across_client_processes`: no shared in-memory task state.
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
            assert!(is_due_this_week(
                before,
                &week_bounds(&today_key()).unwrap()
            ));
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
        "project" => {
            // Projects are objects, and a task's project is an object link that moves with it.
            let project = Project {
                id: "restart-project".into(),
                title: "Ремонт".into(),
                status: 0,
                deadline: None,
                sort_order: 0,
                area_id: None,
            };
            engine.save_project(&project).unwrap();
            let mut todo = new_todo("restart-linked", "Linked");
            todo.created_at = "2026-09-20T10:00:00Z".into();
            todo.project_id = Some(project.id.clone());
            engine.save(None, &todo).unwrap();
            let linked = engine.load().unwrap().todos;
            let before = linked.iter().find(|t| t.id == todo.id).unwrap().clone();
            assert_eq!(before.project_id.as_deref(), Some("restart-project"));
            let mut moved = before.clone();
            moved.project_id = None;
            engine.save(Some(&before), &moved).unwrap();
            let loaded = engine.load().unwrap().todos;
            assert_eq!(
                loaded.iter().find(|t| t.id == todo.id).unwrap().project_id,
                None
            );
            // A project that is not an object yet is refused before the task is written.
            let mut unknown = moved.clone();
            unknown.project_id = Some("missing-project".into());
            assert!(engine.save(Some(&moved), &unknown).is_err());
            assert_eq!(
                engine
                    .load()
                    .unwrap()
                    .todos
                    .iter()
                    .find(|t| t.id == todo.id)
                    .unwrap()
                    .project_id,
                None
            );
        }
        _ => panic!("unknown test mode"),
    }
}
