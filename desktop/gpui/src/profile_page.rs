// The public profile page's side of the app (docs/protocol.md, "Profile
// page"): the Librarian page's Customize panel edits it, through the runner.
use crate::app::Threadbear;
use gpui_kit::prelude::*;
use gpui_kit::*;
use threadbear_core::looks::LibrarianLook;

impl Threadbear {
    /// The profile page's address, only while it's published.
    pub(crate) fn published_profile_url(&self) -> Option<String> {
        None
    }

    /// The Librarian page's look changed: the page made from it goes again.
    pub(crate) fn profile_look_changed(&mut self, _cx: &mut Context<Self>) {}

    /// The panel's Profile page sections.
    pub(crate) fn render_profile_sections(&mut self, _look: &LibrarianLook, _window: &mut Window, _cx: &mut Context<Self>) -> Vec<AnyElement> {
        vec![]
    }
}
