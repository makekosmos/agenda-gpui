//! Dev seed data (src/dev/seed.ts + src/dev/pseudoCalendar.ts) — the
//! `AGENDA_DEMO`/`AGENDA_DEVTASKS` fixtures. Shared by the desktop app and
//! the Android parity gate, so every clock read takes a [`LocalDay`].
use crate::{new_todo, ChecklistItem, LocalDay, Project, RecurrenceRule, Status, Tag, Todo};

fn todo(id: impl Into<String>, title: &str) -> Todo {
    new_todo(id, title)
}

pub fn seed_todos() -> Vec<Todo> {
    seed_todos_at(&LocalDay::now())
}

pub fn seed_todos_at(day: &LocalDay) -> Vec<Todo> {
    let day = *day;
    let mut v: Vec<Todo> = vec![
        todo("dev-inbox-1", "Перечитать бриф по лендингу"),
        {
            let mut t = todo("dev-inbox-2", "Записаться к стоматологу");
            t.priority = 1;
            t
        },
        todo("dev-inbox-3", "Идея: виджет погоды в сайдбаре"),
        {
            let mut t = todo("dev-today-1", "Созвон по дизайну");
            t.status = Status::Todo;
            t.is_today = true;
            t.priority = 3;
            t.tag_ids = vec!["dev-tag-urgent".into(), "dev-tag-focus".into()];
            t.fuel_cost = Some(35.0);
            t
        },
        {
            let mut t = todo("dev-today-2", "Отправить инвойс клиенту");
            t.status = Status::Todo;
            t.scheduled_date = Some(day.day_key(0));
            t.deadline = Some(day.day_key(0));
            t.billable = true;
            t.price = Some(15000.0);
            t.project_id = Some("dev-proj-release".into());
            t
        },
        {
            let mut t = todo("dev-today-evening", "Почитать главу книги");
            t.status = Status::Todo;
            t.is_today = true;
            t.is_evening = true;
            t.area_id = Some("dev-area-life".into());
            t
        },
        {
            let mut t = todo("dev-overdue-1", "Вернуть правки по макетам");
            t.status = Status::Todo;
            t.scheduled_date = Some(day.day_key(-1));
            t.priority = 3;
            t.tag_ids = vec!["dev-tag-urgent".into()];
            t.fuel_cost = Some(20.0);
            t
        },
        {
            let mut t = todo("dev-checklist-1", "Подготовить демо для команды");
            t.status = Status::Todo;
            t.scheduled_date = Some(day.day_key(1));
            t.project_id = Some("dev-proj-release".into());
            t.heading_id = Some("dev-head-prep".into());
            t.checklist = vec![
                ChecklistItem { is_completed: true },
                ChecklistItem {
                    is_completed: false,
                },
                ChecklistItem {
                    is_completed: false,
                },
            ];
            t
        },
        {
            let mut t = todo("dev-week-2", "Ревью PR по навигации");
            t.status = Status::Started;
            t.scheduled_date = Some(day.day_key(2));
            t.project_id = Some("dev-proj-release".into());
            t.heading_id = Some("dev-head-polish".into());
            t.significance = Some(7);
            t.fuel_cost = Some(15.0);
            t
        },
        {
            let mut t = todo("dev-week-3", "Оплатить интернет");
            t.status = Status::Todo;
            t.deadline = Some(day.day_key(3));
            t.reminder_date = Some(day.iso_at(3, 9, 0));
            t.project_id = Some("dev-proj-home".into());
            t
        },
        {
            let mut t = todo("dev-week-4", "Купить подарок Маше");
            t.status = Status::Todo;
            t.scheduled_date = Some(day.day_key(4));
            t.area_id = Some("dev-area-life".into());
            t.tag_ids = vec!["dev-tag-home".into()];
            t
        },
        {
            let mut t = todo("dev-recurring-1", "Поливать цветы");
            t.status = Status::Todo;
            t.scheduled_date = Some(day.day_key(0));
            t.area_id = Some("dev-area-life".into());
            t.recurrence = Some(RecurrenceRule {
                frequency: 1,
                interval: 1,
                recurrence_type: 0,
                days_of_week: vec![1, 3, 5],
            });
            t
        },
        {
            let mut t = todo("dev-next-week-1", "Запланировать отпуск");
            t.status = Status::Todo;
            t.scheduled_date = Some(day.day_key(8));
            t.priority = 2;
            t
        },
        {
            let mut t = todo("dev-someday-1", "Собрать домашний кинотеатр");
            t.status = Status::Deferred;
            t.is_someday = true;
            t.project_id = Some("dev-proj-someday".into());
            t
        },
        {
            let mut t = todo("dev-done-today", "Утренняя планёрка");
            t.status = Status::Done;
            t.is_completed = true;
            t.completed_at = Some(day.iso_at(0, 9, 30));
            t.scheduled_date = Some(day.day_key(0));
            t
        },
        {
            let mut t = todo("dev-done-yesterday", "Позвонить в банк");
            t.status = Status::Done;
            t.is_completed = true;
            t.completed_at = Some(day.iso_at(-1, 14, 0));
            t
        },
        {
            let mut t = todo("dev-canceled-1", "Встреча с брокером");
            t.status = Status::Canceled;
            t.is_cancelled = true;
            t.completed_at = Some(day.iso_at(0, 11, 0));
            t.scheduled_date = Some(day.day_key(1));
            t
        },
        {
            let mut t = todo("dev-trash-1", "Черновик: старый план переезда");
            t.is_trashed = true;
            t
        },
    ];
    v[0].notes = Some("Обратить внимание на секцию про ценообразование.".into());
    let created = day.iso_at(0, 9, 0);
    for (i, t) in v.iter_mut().enumerate() {
        t.created_at = created.clone();
        t.sort_order = i as i32;
    }
    v
}

mod gen;
pub use gen::gen_random_todos;

pub fn seed_projects() -> Vec<Project> {
    seed_projects_at(&LocalDay::now())
}

pub fn seed_projects_at(day: &LocalDay) -> Vec<Project> {
    let day = *day;
    vec![
        Project {
            id: "dev-proj-release".into(),
            title: "Релиз Agenda 1.0".into(),
            status: 0,
            deadline: Some(day.day_key(10)),
            sort_order: 0,
            area_id: Some("dev-area-work".into()),
        },
        Project {
            id: "dev-proj-home".into(),
            title: "Дом и быт".into(),
            status: 0,
            deadline: None,
            sort_order: 1,
            area_id: Some("dev-area-life".into()),
        },
        Project {
            id: "dev-proj-someday".into(),
            title: "Путешествие в Японию".into(),
            status: 1,
            deadline: None,
            sort_order: 2,
            area_id: Some("dev-area-life".into()),
        },
        // status 2: completed project — desktop shows these in the Logbook
        // «Проекты» section, so the parity gate needs one in the seed.
        Project {
            id: "dev-proj-archive".into(),
            title: "Старый сайт".into(),
            status: 2,
            deadline: None,
            sort_order: 3,
            area_id: Some("dev-area-work".into()),
        },
    ]
}

pub fn seed_tags() -> Vec<Tag> {
    vec![
        Tag {
            id: "dev-tag-urgent".into(),
            title: "срочно".into(),
            color: "red".into(),
        },
        Tag {
            id: "dev-tag-focus".into(),
            title: "фокус".into(),
            color: "blue".into(),
        },
        Tag {
            id: "dev-tag-home".into(),
            title: "дом".into(),
            color: "green".into(),
        },
    ]
}
