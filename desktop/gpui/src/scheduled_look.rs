// The Scheduled page laid out as you set it (a port of the Swift app's
// CustomizedScheduledPage and ScheduledLookEditor): with Settings > General >
// Look's Customize the Scheduled page on, its picture (the bear, your own or
// none), a background photo, and the title and status and the numbers, each
// at the top, middle or bottom of the window, or hidden. Customize at the
// top right opens the panel beside the page, which changes it as you go.
// The picture and background sections are the Librarian panel's own
// (looks.rs), as in the Swift app.
use crate::app::Threadbear;
use crate::home_page::{home_stat_tile_on, Art};
use crate::looks::{self, layout_row, panel_section, switch_row, Page, Slot};
use crate::style::px;
use gpui_kit::component::{h_flex, StyledExt as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use threadbear_core::home::HomeStat;
use threadbear_core::looks::{PictureLook as _, Place, ScheduledBlock, ScheduledLook};

/// The numbers' keys, in the order core's `schedules::stats` makes them, as
/// the Swift app's ScheduledHome names them in a saved look.
pub(crate) const NUMBERS: [(&str, &str); 6] = [("on", "On"), ("dueSoon", "Due in a day"), ("repeating", "Repeating"), ("byThreads", "Set by threads"), ("paused", "Paused"), ("runs", "Runs so far")];

const CUSTOMIZE_TIP: &str = "Change the picture, the background, and where each part of this page sits";

impl Threadbear {
    /// The page with Customize on. `about` is the title and status as the
    /// standard page draws them.
    pub(crate) fn render_customized_scheduled(&mut self, about: AnyElement, stats: &[HomeStat], cx: &mut Context<Self>) -> AnyElement {
        let look: ScheduledLook = self.look();
        let pic = look.pic().clone();
        let backdrop = pic.background.as_deref().and_then(|bg| self.render_backdrop(Slot::Page(Page::Scheduled), bg, pic.background_dim, pic.background_focus, cx));
        let on_picture = backdrop.is_some();
        let mut zones: [Vec<AnyElement>; 3] = Default::default();
        let mut about = Some(about);
        for block in look.layout() {
            let zone = match look.place(block) {
                Place::Top => 0,
                Place::Middle => 1,
                Place::Bottom => 2,
            };
            let part = match block {
                ScheduledBlock::Picture => self.render_look_picture(pic.picture, pic.picture_file.as_deref(), pic.picture_size.side(), Some(Art::Scheduled), cx),
                ScheduledBlock::About => about.take().map(|a| div().w_full().max_w(px(620.)).child(a).into_any_element()),
                ScheduledBlock::Numbers => {
                    let shown: Vec<&HomeStat> = stats.iter().zip(NUMBERS).filter(|(_, (k, _))| !look.hidden_numbers.contains(*k)).map(|(s, _)| s).collect();
                    (!shown.is_empty()).then(|| h_flex().flex_wrap().max_w(px(620.)).gap(px(14.)).children(shown.into_iter().enumerate().map(|(i, s)| home_stat_tile_on(i, s, on_picture, cx))).into_any_element())
                }
            };
            if let Some(part) = part {
                zones[zone].push(part);
            }
        }
        let editor = (self.looks.editing == Some(Page::Scheduled)).then(|| self.render_scheduled_editor(cx));
        // Top, middle and bottom are real places in the window on this page.
        self.render_customized_page(Page::Scheduled, backdrop, zones, true, 1000., CUSTOMIZE_TIP, vec![], editor, cx)
    }

    fn render_scheduled_editor(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let look: ScheduledLook = self.look();
        let mut sections = vec![self.render_picture_section::<ScheduledLook>("Show the bear winding its alarm clock", cx), self.render_background_section::<ScheduledLook>(cx)];

        let rows = ScheduledBlock::ALL
            .iter()
            .map(|&block| {
                let tip = format!("Show or hide {} on the page", block.title().to_lowercase());
                layout_row(format!("look-scheduled-layout-{}", block.raw()), block.title(), !look.hidden.contains(&block), tip, look.place(block), cx, move |this, on, cx| this.edit_look::<ScheduledLook>(cx, |l| l.set_shown(on, block)), move |this, place, cx| this.edit_look::<ScheduledLook>(cx, |l| l.set_place(place, block)))
            })
            .collect();
        sections.push(panel_section("Layout", None, rows, cx));

        let numbers_hidden = look.hidden.contains(&ScheduledBlock::Numbers);
        let rows = NUMBERS
            .iter()
            .map(|&(key, title)| {
                switch_row(format!("look-scheduled-number-{key}"), title, !look.hidden_numbers.contains(key), format!("Show or hide {}", title.to_lowercase()), numbers_hidden, cx, move |this, on, cx| {
                    this.edit_look::<ScheduledLook>(cx, |l| {
                        if on {
                            l.hidden_numbers.remove(key);
                        } else {
                            l.hidden_numbers.insert(key.to_owned());
                        }
                    })
                })
            })
            .collect();
        sections.push(panel_section("Numbers", None, rows, cx));

        sections.push(panel_section(
            "",
            None,
            vec![looks::kit_button("look-scheduled-reset", "Reset to the standard page", None, "Put the bear, the layout and every part back as Threadbear ships them, and take away your pictures", look.is_standard(), false, cx.listener(|this, _, _, cx| this.save_look(ScheduledLook::default(), cx)))],
            cx,
        ));
        self.render_look_editor(Page::Scheduled, sections, cx)
    }
}
