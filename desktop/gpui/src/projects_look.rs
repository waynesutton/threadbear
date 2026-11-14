// Projects' Customize panel uses the shared picture, background and layout controls.
use crate::app::Threadbear;
use crate::looks::{self, Page, panel_section, layout_row, switch_row};
use gpui_kit::prelude::*;
use gpui_kit::*;
use threadbear_core::looks::{ProjectsLook, ProjectsBlock, CardPart};
impl Threadbear {
    pub(crate) fn render_projects_look_editor(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let look: ProjectsLook = self.look();
        let mut sections = vec![self.render_picture_section::<ProjectsLook>("", cx), self.render_background_section::<ProjectsLook>(cx)];
        let rows = ProjectsBlock::ALL.iter().map(|&block| layout_row(format!("projects-layout-{}", block.raw()), block.title(), !look.hidden.contains(&block), format!("Show or hide {} on the page", block.title().to_lowercase()), look.place(block), cx,
            move |this, on, cx| this.edit_look::<ProjectsLook>(cx, |l| l.set_shown(on, block)),
            move |this, place, cx| this.edit_look::<ProjectsLook>(cx, |l| l.set_place(place, block)))).collect();
        sections.push(panel_section("Layout", None, rows, cx));
        let rows = CardPart::ALL.iter().map(|&part| switch_row(format!("projects-card-{}", part.raw()), part.title(), look.shows_card(part), format!("Show or hide {} on each project card", part.title().to_lowercase()), look.hidden.contains(&ProjectsBlock::Cards), cx,
            move |this, on, cx| this.edit_look::<ProjectsLook>(cx, |l| l.set_card_shown(on, part)))).collect();
        sections.push(panel_section("Cards", None, rows, cx));
        sections.push(panel_section("", None, vec![looks::kit_button("projects-look-reset", "Reset to the standard page", None, "Put the layout and every card part back as Threadbear ships them, and take away your pictures", look.is_standard(), false, cx.listener(|this, _, _, cx| this.save_look(ProjectsLook::default(), cx)))], cx));
        self.render_look_editor(Page::Projects, sections, cx)
    }
}
