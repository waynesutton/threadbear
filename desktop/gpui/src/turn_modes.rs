// Shift-Tab in the message box (PlanFirstToggle.swift): Plan first on or
// off. Plan first is the thread's permission mode, set as the access menu
// sets it, and turning it off puts back the access the box had. The rule is
// threadbear_core::turn_modes. Settings > General can give Shift-Tab back
// to the text box.
//
// What to put back is kept per box (a thread's id, or "new" for the
// new-thread page) while the app runs, as the Swift app's `planPutBack` is.
use crate::composer::Composer;
use gpui_kit::*;
use std::cell::RefCell;
use std::collections::HashMap;
use threadbear_core::turn_modes as rules;

thread_local! {
    /// Each box's access before it went into Plan first, to put back.
    static PUT_BACK: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
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

    /// Shift-Tab (`togglePlanFirst`): Plan first on or off. False when
    /// Settings has it off, so the key does what it did.
    pub(crate) fn cycle_turn_mode(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.app.upgrade().is_none_or(|app| app.read(cx).settings.turn_modes_enabled()) {
            return false;
        }
        let key = self.mode_key();
        let Some(permission) = self.chip_data(cx).summary.map(|t| if t.permission_mode.is_empty() { "full".to_owned() } else { t.permission_mode }) else { return false };
        let put_back = PUT_BACK.with(|p| p.borrow().get(&key).cloned());
        let mode: &'static str = match rules::next(&permission, put_back.as_deref()).as_str() {
            "plan" => "plan",
            "full" => "full",
            "auto" => "auto",
            _ => "ask",
        };
        self.set_permission(mode, cx);
        cx.notify();
        true
    }
}
