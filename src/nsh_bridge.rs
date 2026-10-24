use std::{cell::RefCell, rc::Rc};

use gpui::{App, Entity, Subscription, Window};
use imago_nsh::{ComponentContext, NativeComponent};
use serde_json::{json, Value};

use crate::app::{Agenda, Route};
use crate::model::{filter_idx, SmartList};

pub(crate) struct AgendaBridge {
    context: ComponentContext,
    agenda: Entity<Agenda>,
    route: Rc<RefCell<String>>,
    project_id: Rc<RefCell<String>>,
    subscription: Option<Subscription>,
}

impl AgendaBridge {
    pub(crate) fn new(context: ComponentContext, agenda: Entity<Agenda>) -> Self {
        Self {
            context,
            agenda,
            route: Rc::new(RefCell::new(String::new())),
            project_id: Rc::new(RefCell::new("dev-proj-release".into())),
            subscription: None,
        }
    }

    fn publish(&self, cx: &App) {
        let agenda = self.agenda.read(cx);
        let route = route_key(&agenda.route);
        self.context.emit("state", snapshot(agenda, &route));
    }

    fn route(&self) -> Route {
        match self.route.borrow().as_str() {
            "week" => Route::Week,
            "next-week" => Route::NextWeek,
            "someday" => Route::Someday,
            "logbook" => Route::Logbook,
            "trash" => Route::Trash,
            "statistics" => Route::Statistics,
            "settings" => Route::Settings,
            "about" => Route::About,
            "project" => Route::Project(self.project_id.borrow().clone()),
            _ => Route::Inbox,
        }
    }
}

fn route_key(route: &Route) -> String {
    match route {
        Route::Inbox => "inbox".into(),
        Route::Week => "week".into(),
        Route::NextWeek => "next-week".into(),
        Route::Someday => "someday".into(),
        Route::Logbook => "logbook".into(),
        Route::Trash => "trash".into(),
        Route::Settings => "settings".into(),
        Route::Statistics => "statistics".into(),
        Route::About => "about".into(),
        Route::Project(_) => "project".into(),
    }
}

pub(crate) fn initial_snapshot(agenda: &Agenda) -> Value {
    snapshot(agenda, "inbox")
}

fn snapshot(agenda: &Agenda, route: &str) -> Value {
    let list = match route {
        "week" => Some(SmartList::Week),
        "next-week" => Some(SmartList::NextWeek),
        "someday" => Some(SmartList::Someday),
        "logbook" => Some(SmartList::Logbook),
        "trash" => Some(SmartList::Trash),
        "statistics" | "settings" | "about" | "project" => None,
        _ => Some(SmartList::Inbox),
    };
    let project_id = match &agenda.route {
        Route::Project(id) => Some(id.as_str()),
        _ => None,
    };
    let todos = if let Some(project_id) = project_id {
        agenda
            .todos
            .iter()
            .filter(|todo| {
                todo.project_id.as_deref() == Some(project_id)
                    && !todo.is_trashed
                    && !todo.is_completed
            })
            .collect::<Vec<_>>()
    } else {
        list.map(|list| {
            filter_idx(list, &agenda.todos)
                .into_iter()
                .map(|index| &agenda.todos[index])
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
    };
    let project = project_id.and_then(|id| agenda.projects.iter().find(|project| project.id == id));
    let archived = agenda
        .todos
        .iter()
        .filter(|todo| todo.is_completed && !todo.is_trashed)
        .count();
    let trashed = agenda.todos.iter().filter(|todo| todo.is_trashed).count();
    let theme_idx = agenda
        .appearance
        .as_ref()
        .map(|appearance| appearance.theme_index(agenda.theme_sel != 0))
        .unwrap_or(agenda.theme_idx);
    let theme = crate::palettes::THEMES
        .get(theme_idx)
        .unwrap_or(&crate::palettes::THEMES[0]);
    let palette = if agenda.theme_sel == 0 {
        &theme.light
    } else {
        &theme.dark
    };
    let color = |value: u32| format!("#{value:06x}");
    let accent = agenda
        .appearance
        .as_ref()
        .and_then(|appearance| appearance.accent())
        .unwrap_or(palette.accent);
    json!({
        "route": route,
        "title": agenda.page_title().0.to_string(),
        "todos": todos,
        "projects": &agenda.projects,
        "project": project,
        "archivedCount": archived,
        "trashCount": trashed,
        "todoCount": agenda.todos.len(),
        "version": env!("CARGO_PKG_VERSION"),
        "activity": agenda.nsh_activity(),
        "appearance": agenda.appearance,
        "appearanceEditable": agenda.appearance_editable(),
        "themes": crate::palettes::THEMES.iter().map(|theme| json!({"key": theme.key, "name": theme.name})).collect::<Vec<_>>(),
        "colors": {
            "background": color(palette.bg),
            "foreground": color(palette.fg),
            "muted": color(palette.muted_fg),
            "border": color(palette.border),
            "accent": color(accent),
            "card": color(palette.card),
        },
        "sidebarVisible": agenda.sidebar_visible,
    })
}

impl NativeComponent for AgendaBridge {
    fn render(&mut self, _window: &mut Window, cx: &mut App) -> Option<gpui::AnyElement> {
        if self.subscription.is_none() {
            let context = self.context.clone();
            self.subscription = Some(cx.observe(&self.agenda, move |agenda, cx| {
                let state = agenda.read(cx);
                let route = route_key(&state.route);
                context.emit("state", snapshot(state, &route));
            }));
            self.publish(cx);
        }
        None
    }

    fn set_prop(&mut self, name: &str, value: Option<&Value>, _window: &mut Window, cx: &mut App) {
        if name == "projectId" {
            *self.project_id.borrow_mut() =
                value.and_then(Value::as_str).unwrap_or_default().to_owned();
            return;
        }
        if name != "route" {
            return;
        }
        let route = value.and_then(Value::as_str).unwrap_or("inbox").to_owned();
        if *self.route.borrow() == route {
            return;
        }
        *self.route.borrow_mut() = route;
        let route = self.route();
        self.agenda.update(cx, |agenda, cx| {
            agenda.navigate(route);
            cx.notify();
        });
        self.publish(cx);
    }

    fn command(&mut self, name: &str, args: &Value, _window: &mut Window, cx: &mut App) {
        match name {
            "complete" => {
                if let Some(id) = args.get("id").and_then(Value::as_str) {
                    let id = id.to_owned();
                    self.agenda
                        .update(cx, |agenda, _| agenda.complete_todo(&id));
                    self.publish(cx);
                }
            }
            "create" => {
                if let Some(title) = args.get("title").and_then(Value::as_str) {
                    let title = title.to_owned();
                    self.agenda
                        .update(cx, |agenda, _| agenda.create_nsh_todo(&title));
                    self.publish(cx);
                }
            }
            "appearance.set" => {
                if let Some(patch) = args.as_object() {
                    self.agenda.update(cx, |agenda, cx| {
                        agenda.persist_appearance_patch(Value::Object(patch.clone()), cx);
                    });
                    self.publish(cx);
                }
            }
            "toggle-sidebar" => {
                self.agenda.update(cx, |agenda, cx| {
                    agenda.toggle_sidebar();
                    cx.notify();
                });
            }
            "reload" => {
                self.agenda.update(cx, |agenda, _| agenda.reload_storage());
            }
            _ => {}
        }
    }
}
