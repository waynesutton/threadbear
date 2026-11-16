// The Projects page laid out as you set it (a port of the Swift app's
// CustomizedProjectsPage and ProjectsLookEditor): with Settings > General >
// Look's Customize the Projects page on, a picture of your own (the page
// ships with none, so there's no bear), a background photo, the title and
// search and the cards each at the top, middle or bottom of the window or
// hidden, and what each card shows. Hiding the title and search shows every
// project. The picture and background sections are the other panels' own
// (looks.rs).
use crate::app::Threadbear;
use crate::looks::{self, layout_row, panel_section, switch_row, Page, Slot};
use gpui_kit::prelude::*;
use gpui_kit::*;
use threadbear_core::looks::{CardPart, PictureLook as _, Place, ProjectsBlock, ProjectsLook};

const CUSTOMIZE_TIP: &str = "Change the picture, the background, where each part of this page sits, and what each card shows";

impl Threadbear {
    /// The page with Customize on: `header` is the title, summary, search
    /// and New project, `cards` the grid (or the line saying why it's empty),
    /// as the standard page draws them.
    pub(crate) fn render_customized_projects(&mut self, look: ProjectsLook, header: AnyElement, cards: AnyElement, cx: &mut Context<Self>) -> AnyElement {
        let pic = look.pic().clone();
        let backdrop = pic.background.as_deref().and_then(|bg| self.render_backdrop(Slot::Page(Page::Projects), bg, pic.background_dim, pic.background_focus, cx));
        let mut zones: [Vec<AnyElement>; 3] = Default::default();
        let (mut header, mut cards) = (Some(header), Some(cards));
        for block in look.layout() {
            let zone = match look.place(block) {
                Place::Top => 0,
                Place::Middle => 1,
                Place::Bottom => 2,
            };
            let part = match block {
                ProjectsBlock::Picture => self.render_look_picture(pic.picture, pic.picture_file.as_deref(), pic.picture_size.side(), None, cx),
                ProjectsBlock::Header => header.take(),
                ProjectsBlock::Cards => cards.take(),
            };
            if let Some(part) = part {
                zones[zone].push(part);
            }
        }
        let editor = (self.looks.editing == Some(Page::Projects)).then(|| self.render_projects_editor(cx));
        // Top, middle and bottom are real places in the window on this page.
        self.render_customized_page(Page::Projects, backdrop, zones, true, 1160., CUSTOMIZE_TIP, vec![], editor, cx)
    }

    fn render_projects_editor(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let look: ProjectsLook = self.look();
        let mut sections = vec![self.render_picture_section::<ProjectsLook>("", cx), self.render_background_section::<ProjectsLook>(cx)];

        let rows = ProjectsBlock::ALL
            .iter()
            .map(|&block| {
                let tip = match block {
                    ProjectsBlock::Picture => "Show or hide your picture. Choose one under Picture first",
                    ProjectsBlock::Header => "Show or hide the title, search and New project. Hidden, the cards show every project",
                    ProjectsBlock::Cards => "Show or hide the project cards",
                };
                layout_row(format!("look-projects-layout-{}", block.raw()), block.title(), !look.hidden.contains(&block), tip.to_owned(), look.place(block), cx, move |this, on, cx| this.edit_look::<ProjectsLook>(cx, |l| l.set_shown(on, block)), move |this, place, cx| this.edit_look::<ProjectsLook>(cx, |l| l.set_place(place, block)))
            })
            .collect();
        sections.push(panel_section("Layout", None, rows, cx));

        let cards_hidden = look.hidden.contains(&ProjectsBlock::Cards);
        let rows = CardPart::ALL
            .iter()
            .map(|&part| {
                let tip = match part {
                    CardPart::Art => "The wide tile with the project's icon. Off, cards are just text",
                    CardPart::Threads => "How many open threads each project has",
                    CardPart::Status => "What its threads are doing, or when it was last active",
                    CardPart::State => "The first lines of where the project is up to",
                };
                switch_row(format!("look-projects-card-{}", part.raw()), part.title(), look.shows_card(part), tip, cards_hidden, cx, move |this, on, cx| this.edit_look::<ProjectsLook>(cx, |l| l.set_card_shown(on, part)))
            })
            .collect();
        sections.push(panel_section("Cards", None, rows, cx));

        sections.push(panel_section(
            "",
            None,
            vec![looks::kit_button("look-projects-reset", "Reset to the standard page", None, "Put the layout and every card part back as Threadbear ships them, and take away your pictures", look.is_standard(), false, cx.listener(|this, _, _, cx| this.save_look(ProjectsLook::default(), cx)))],
            cx,
        ));
        self.render_look_editor(Page::Projects, sections, cx)
    }
}
