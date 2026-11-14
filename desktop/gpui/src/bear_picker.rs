// The composer's @ list. Names are plain text when picked, for the runner
// to route; the list belongs only to the local runner.
use crate::composer::Composer;
use crate::style::{self, px};
use crate::tips::Tip as _;
use gpui_kit::component::{v_flex, h_flex, ActiveTheme as _};
use gpui_kit::prelude::*;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::*;
use threadbear_core::bears::{self, BearEntry};
use threadbear_core::slash::PaletteKey;

impl Composer {
    fn bear_offer(&self, window: &Window, cx: &App) -> Option<(std::ops::Range<usize>, Vec<BearEntry>)> {
        let app = self.app.upgrade()?;
        let app = app.read(cx);
        let local = match &self.thread_id {
            Some(id) => app.threads.get(id).is_some_and(|t| !t.is_remote()),
            None => app.new_thread.picks.runner.is_none(),
        };
        let input = self.input.read(cx);
        let selection = input.selected_range();
        if !selection.is_empty() { return None; }
        let text = input.value().to_string();
        let range = bears::token(&text, selection.end)?;
        if !self.bear_gate.may_offer(range.start, input.focus_handle(cx).is_focused(window), local) { return None; }
        let found = bears::offer(&text, &range, &app.bear_list.bears);
        (!found.is_empty()).then_some((range, found))
    }

    pub(crate) fn bear_text_changed(&mut self, cx: &mut Context<Self>) {
        let input = self.input.read(cx);
        let text = input.value().to_string();
        let token = bears::token(&text, input.selected_range().end);
        self.bear_gate.moved(token.as_ref().map(|r| r.start));
        self.bear_selected = 0;
        let local = self.app.upgrade().is_some_and(|app| {
            let app = app.read(cx);
            match &self.thread_id {
                Some(id) => app.threads.get(id).is_some_and(|t| !t.is_remote()),
                None => app.new_thread.picks.runner.is_none(),
            }
        });
        if token.is_some() && local {
            if let Some(app) = self.app.upgrade() {
                app.update(cx, |app, cx| app.refresh_bears(cx));
            }
        }
    }

    pub(crate) fn bear_key(&mut self, key: PaletteKey, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some((range, found)) = self.bear_offer(window, cx) else { return false; };
        let modifiers = window.modifiers();
        if modifiers.shift || modifiers.control || modifiers.alt || modifiers.platform {
            self.bear_gate.dismissed_at = Some(range.start);
            cx.notify();
            return false;
        }
        match key {
            PaletteKey::Up => self.bear_selected = bears::step(self.bear_selected, -1, found.len()),
            PaletteKey::Down => self.bear_selected = bears::step(self.bear_selected, 1, found.len()),
            PaletteKey::Escape => self.bear_gate.dismissed_at = Some(range.start),
            PaletteKey::Tab | PaletteKey::Enter => {
                let bear = found[self.bear_selected.min(found.len() - 1)].clone();
                self.pick_bear(range, bear, window, cx);
            }
            // Slash-palette navigation keys remain the text box's here.
            #[allow(unreachable_patterns)]
            _ => return false,
        }
        cx.notify();
        true
    }

    fn pick_bear(&mut self, range: std::ops::Range<usize>, bear: BearEntry, window: &mut Window, cx: &mut Context<Self>) {
        self.input.update(cx, |s, cx| {
            s.set_selected_range(range, cx);
            s.replace(bears::mention(&bear), window, cx);
            s.focus(window, cx);
            crate::web_panels::keyboard_to_app(window);
        });
        self.bear_selected = 0;
        cx.notify();
    }

    pub(crate) fn render_bear_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let input = self.input.read(cx);
        let token = bears::token(&input.value().to_string(), input.selected_range().end);
        self.bear_gate.moved(token.as_ref().map(|r| r.start));
        let (range, found) = self.bear_offer(window, cx)?;
        self.bear_selected = self.bear_selected.min(found.len() - 1);
        let mut list = v_flex().w(px(320.)).p(px(6.)).rounded(px(10.)).bg(cx.theme().background).border_1().border_color(style::hairline(cx)).shadow_lg().occlude();
        for (index, bear) in found.into_iter().enumerate() {
            let selected = self.bear_selected == index;
            let pick_range = range.clone();
            let tile = crate::bears::tipped_tile(SharedString::from(format!("bear-picker-tile-{}", bear.id)), &bear.tile(), 28.);
            let row = h_flex().id(SharedString::from(format!("bear-picker-{}", bear.id))).test_support().role(Role::Button).aria_label(format!("Mention {}", bear.name)).tip(format!("Mention {}", bear.name))
                .gap(px(8.)).p(px(6.)).rounded(px(5.)).cursor_pointer()
                .when(selected, |s| s.bg(style::sidebar_hover(cx)))
                .child(tile)
                .child(v_flex().min_w_0().child(div().text_size(px(style::size::LABEL)).child(format!("@{}", bear.name)))
                    .when_some(bear.does.clone(), |s, does| s.child(div().truncate().text_size(px(style::size::DETAIL)).text_color(style::secondary_text(cx)).child(does))))
                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    window.prevent_default();
                    this.pick_bear(pick_range.clone(), bear.clone(), window, cx);
                }));
            list = list.child(row);
        }
        Some(deferred(div().absolute().left_0().bottom_full().mb(px(6.)).child(list)).into_any_element())
    }
}
