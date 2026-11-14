// Shift-Tab in the message box (TurnModes.swift): it steps the box through
// Plan first, Ask, then off. Plan first is the thread's permission mode, set
// as the access menu sets it; Ask goes with each message sent from the box
// (`mode` on threads.send and threads.create), shown by a chip beside the
// access chip. The rules are threadbear_core::turn_modes. Settings > General
// can turn the modes off, and Shift-Tab then does what it did before.
//
// Kept per box (a thread's id, or "new" for the new-thread page) while the
// app runs, as the Swift app's `turnModes` and `planPutBack` are.
use crate::composer::Composer;
use crate::style::{self, px};
use crate::tips::Tip as _;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::Sizable as _;
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use threadbear_core::turn_modes::{self as rules, ASK_TIP, ASK_TITLE};

thread_local! {
    /// The boxes in Ask mode.
    static ASKING: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    /// Each box's access before it went into Plan first, to put back.
    static PUT_BACK: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
}

/// The box `key` is in Ask mode (a thread's id, or "new").
pub(crate) fn asking(key: &str) -> bool {
    ASKING.with(|a| a.borrow().contains(key))
}

fn set_asking(key: &str, on: bool) {
    ASKING.with(|a| {
        let mut a = a.borrow_mut();
        if on {
            a.insert(key.to_owned());
        } else {
            a.remove(key);
        }
    });
}

/// A new thread made from the new-thread page's box keeps its Ask mode.
pub(crate) fn moved_to(thread_id: &str) {
    if asking("new") {
        set_asking("new", false);
        set_asking(thread_id, true);
    }
}

/// Settings turned the modes off: every box back to how it was.
pub(crate) fn clear_all() {
    ASKING.with(|a| a.borrow_mut().clear());
}

/// The access menu or Shift-Tab changed a box's access: going into Plan
/// first notes what it had, to put back (`setPermission(for:)`).
pub(crate) fn permission_changing(key: &str, from: &str, to: &str) {
    PUT_BACK.with(|p| {
        let mut p = p.borrow_mut();
        if to == "plan" {
            if from != "plan" {
                p.insert(key.to_owned(), from.to_owned());
            }
        } else {
            p.remove(key);
        }
    });
}

impl Composer {
    /// The box's key: its thread, or "new".
    pub(crate) fn mode_key(&self) -> String {
        self.thread_id.clone().unwrap_or_else(|| "new".to_owned())
    }

    fn modes_on(&self, cx: &App) -> bool {
        self.app.upgrade().is_none_or(|app| app.read(cx).settings.turn_modes_enabled())
    }

    /// Ask mode, while the modes are on.
    pub(crate) fn ask_mode(&self, cx: &App) -> bool {
        self.modes_on(cx) && asking(&self.mode_key())
    }

    /// Shift-Tab (`cycleTurnMode`): Plan first, Ask, off. False when the
    /// modes are off, so the key does what it did.
    pub(crate) fn cycle_turn_mode(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.modes_on(cx) {
            return false;
        }
        let key = self.mode_key();
        let Some(permission) = self.chip_data(cx).summary.map(|t| if t.permission_mode.is_empty() { "full".to_owned() } else { t.permission_mode }) else { return false };
        let put_back = PUT_BACK.with(|p| p.borrow().get(&key).cloned());
        let step = rules::next(&permission, asking(&key), put_back.as_deref());
        set_asking(&key, step.ask);
        if let Some(mode) = step.permission {
            let mode: &'static str = match mode.as_str() {
                "plan" => "plan",
                "full" => "full",
                "auto" => "auto",
                _ => "ask",
            };
            self.set_permission(mode, cx);
        }
        cx.notify();
        true
    }

    /// `TurnModeChip`: Ask beside the access chip, in the accent, so it's
    /// plain the next message won't change anything. A click steps on.
    pub(crate) fn render_turn_mode_chip(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.ask_mode(cx) {
            return None;
        }
        let accent = style::accent(cx);
        Some(
            div()
                .id("composer-turn-mode")
                .test_support()
                .role(Role::Button)
                .aria_label("Ask mode")
                .flex_none()
                .flex()
                .items_center()
                .gap(px(4.))
                .px(px(7.))
                .py(px(3.))
                .rounded_full()
                .bg(accent.opacity(0.14))
                .text_color(accent)
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .cursor_pointer()
                .child(crate::icons::symbol("questionmark.circle").with_size(px(12.)))
                .child(ASK_TITLE)
                .tip(ASK_TIP)
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.cycle_turn_mode(cx);
                }))
                .into_any_element(),
        )
    }
}
