// The Scheduled page's Customize panel and its parts at real window positions.
use crate::app::Threadbear;
use crate::home_page::{Art, home_title, home_status_line, home_stat_tile_on};
use crate::looks::{self, Page, Slot, layout_row, panel_section, switch_row};
use crate::style::px;
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::prelude::*;
use gpui_kit::*;
use threadbear_core::chrono::Local;
use threadbear_core::looks::{ScheduledLook, ScheduledBlock, PictureLook, Place};
use threadbear_core::schedules as rules;

const NUMBERS: [(&str, &str); 6] = [("on", "On"), ("dueSoon", "Due in a day"), ("repeating", "Repeating"), ("byThreads", "Set by threads"), ("paused", "Paused"), ("runs", "Runs so far")];
impl Threadbear {
    pub(crate) fn render_customized_scheduled(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let look: ScheduledLook = self.look();
        let pic = look.pic();
        let backdrop = pic.background.as_deref().and_then(|f| self.render_backdrop(Slot::Page(Page::Scheduled), f, pic.background_dim, pic.background_focus, cx));
        let on_picture = backdrop.is_some();
        let mut zones: [Vec<AnyElement>; 3] = Default::default();
        for block in look.layout() {
            let zone = match look.place(block) { Place::Top => 0, Place::Middle => 1, Place::Bottom => 2 };
            let part = match block {
                ScheduledBlock::Picture => self.render_look_picture(pic.picture, pic.picture_file.as_deref(), pic.picture_size.side(), Some(Art::Scheduled), cx),
                ScheduledBlock::About => {
                    let local = Local::now();
                    let next = rules::next(&self.scheduled.list);
                    let status = next.map_or_else(|| "Nothing coming up".to_owned(), |s| format!("Next: {}, {}", rules::title(s), rules::when(s, &local)));
                    Some(v_flex().gap(px(12.)).max_w(px(620.)).child(home_title("Scheduled", "Messages that send themselves later, once or on repeat.", crate::scheduled::SCHEDULED_MORE, cx)).child(home_status_line(&status, next.is_some(), cx)).into_any_element())
                }
                ScheduledBlock::Numbers => {
                    let stats = if self.schedules_supported { rules::stats(&self.scheduled.list, crate::now_ms()) } else { vec![] };
                    let shown: Vec<_> = stats.iter().zip(NUMBERS).filter(|(_, (key, _))| !look.hidden_numbers.contains(*key)).map(|(s, _)| s).collect();
                    (!shown.is_empty()).then(|| h_flex().flex_wrap().max_w(px(620.)).gap(px(14.)).children(shown.into_iter().enumerate().map(|(i, s)| home_stat_tile_on(i, s, on_picture, cx))).into_any_element())
                }
            };
            if let Some(part) = part { zones[zone].push(part); }
        }
        let editor = (self.looks.editing == Some(Page::Scheduled)).then(|| self.render_scheduled_look_editor(cx));
        self.render_customized_page(Page::Scheduled, backdrop, zones, true, 1000., "Change the picture, background, and where each part of the Scheduled page sits", vec![], editor, cx)
    }

    fn render_scheduled_look_editor(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let look: ScheduledLook = self.look();
        let mut sections = vec![self.render_picture_section::<ScheduledLook>("Show the scheduled bear", cx), self.render_background_section::<ScheduledLook>(cx)];
        let rows = ScheduledBlock::ALL.iter().map(|&block| layout_row(format!("scheduled-layout-{}", block.raw()), block.title(), !look.hidden.contains(&block), format!("Show or hide {} on the page", block.title().to_lowercase()), look.place(block), cx,
            move |this, on, cx| this.edit_look::<ScheduledLook>(cx, |l| l.set_shown(on, block)),
            move |this, place, cx| this.edit_look::<ScheduledLook>(cx, |l| l.set_place(place, block)))).collect();
        sections.push(panel_section("Layout", None, rows, cx));
        let rows = NUMBERS.iter().map(|&(key, label)| switch_row(format!("scheduled-number-{key}"), label, !look.hidden_numbers.contains(key), format!("Show or hide {}", label.to_lowercase()), look.hidden.contains(&ScheduledBlock::Numbers), cx,
            move |this, on, cx| this.edit_look::<ScheduledLook>(cx, |l| { if on { l.hidden_numbers.remove(key); } else { l.hidden_numbers.insert(key.to_owned()); } }))).collect();
        sections.push(panel_section("Numbers", None, rows, cx));
        sections.push(panel_section("", None, vec![looks::kit_button("scheduled-look-reset", "Reset to the standard page", None, "Put the Scheduled page back as Threadbear ships it", look.is_standard(), false, cx.listener(|this, _, _, cx| this.save_look(ScheduledLook::default(), cx)))], cx));
        self.render_look_editor(Page::Scheduled, sections, cx)
    }
}
