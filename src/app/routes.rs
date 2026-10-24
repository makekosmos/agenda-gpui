use super::Agenda;
use crate::model::Project;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Route {
    Inbox,
    Week,
    NextWeek,
    Someday,
    Statistics,
    Logbook,
    Trash,
    Settings,
    About,
    Project(String),
}

impl Agenda {
    pub(crate) fn navigate(&mut self, route: Route) {
        self.route = route;
    }

    pub(crate) fn project(&self, id: &str) -> Option<&Project> {
        self.projects.iter().find(|project| project.id == id)
    }

    pub(crate) fn page_title(&self) -> (String, &'static str) {
        match &self.route {
            Route::Inbox => ("Входящие".into(), "inbox"),
            Route::Week => ("Эта неделя".into(), "week"),
            Route::NextWeek => ("Следующая неделя".into(), "nextWeek"),
            Route::Someday => ("Потом".into(), "someday"),
            Route::Statistics => ("Активность".into(), "statistics"),
            Route::Logbook => ("Архив".into(), "logbook"),
            Route::Trash => ("Корзина".into(), "trash"),
            Route::Settings => ("Настройки".into(), "settings"),
            Route::About => ("О приложении".into(), "about"),
            Route::Project(id) => (
                self.project(id)
                    .map(|project| project.title.clone())
                    .unwrap_or_else(|| "Проект".into()),
                "project",
            ),
        }
    }
}
