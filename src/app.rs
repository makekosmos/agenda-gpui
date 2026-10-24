mod actions;
mod activity;
mod appearance;
mod engine;
mod routes;

pub(crate) use routes::Route;

use crate::appearance::{Poller, Response, Writer};
use crate::model::{Project, Todo};
use crate::store::Worker;
use gpui::Context;

pub(crate) struct Agenda {
    pub(crate) demo: bool,
    pub(crate) storage: Option<Worker>,
    pub(crate) storage_ready: bool,
    pub(crate) storage_busy: bool,
    pub(crate) storage_error: Option<String>,
    pub(crate) todos: Vec<Todo>,
    pub(crate) projects: Vec<Project>,
    pub(crate) route: Route,
    pub(crate) sidebar_visible: bool,
    pub(crate) appearance: Option<Response>,
    pub(crate) appearance_poller: Option<Poller>,
    pub(crate) appearance_writer: Option<Writer>,
    pub(crate) theme_sel: u8,
    pub(crate) theme_idx: usize,
}

impl Agenda {
    pub(crate) fn new(cx: &mut Context<Self>) -> Self {
        let demo = std::env::var("AGENDA_DEMO").as_deref() == Ok("1")
            || std::env::var("AGENDA_DEVTASKS")
                .ok()
                .and_then(|value| value.parse::<usize>().ok())
                .is_some()
            || cfg!(test);
        let appearance = demo.then(Self::demo_appearance);
        let mut agenda = Self {
            demo,
            storage: None,
            storage_ready: demo,
            storage_busy: false,
            storage_error: None,
            todos: if demo {
                crate::model::seed_todos()
            } else {
                Vec::new()
            },
            projects: if demo {
                crate::model::seed_projects()
            } else {
                Vec::new()
            },
            route: initial_route(),
            sidebar_visible: true,
            appearance,
            appearance_poller: None,
            appearance_writer: (!demo).then(Writer::start),
            theme_sel: 1,
            theme_idx: 0,
        };
        agenda.start_storage(cx);
        agenda.start_appearance(cx);
        agenda
    }
}

fn initial_route() -> Route {
    match std::env::var("AGENDA_ROUTE").ok().as_deref() {
        Some("week") => Route::Week,
        Some("next-week") => Route::NextWeek,
        Some("someday") => Route::Someday,
        Some("statistics") => Route::Statistics,
        Some("logbook") => Route::Logbook,
        Some("trash") => Route::Trash,
        Some("settings") => Route::Settings,
        Some("about") => Route::About,
        Some(route) if route.starts_with("project/") => Route::Project(route[8..].to_owned()),
        _ => Route::Inbox,
    }
}
