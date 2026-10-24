use gpui::{div, Context, IntoElement, ParentElement, Render, Styled, Window};
use imago_nsh::AppView;

use crate::app::Agenda;

pub(crate) struct AgendaShell {
    pub(crate) agenda: gpui::Entity<Agenda>,
    pub(crate) nsh: gpui::Entity<AppView>,
    pub(crate) close_handler_installed: bool,
}

impl Render for AgendaShell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.close_handler_installed {
            self.close_handler_installed = true;
            let agenda = self.agenda.clone();
            window.on_window_should_close(cx, move |_, cx| {
                agenda.update(cx, |state, _| !state.storage_busy)
            });
        }
        div().size_full().child(self.nsh.clone())
    }
}
