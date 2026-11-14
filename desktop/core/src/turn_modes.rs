// Shift-Tab in the message box (TurnModes.swift, docs/protocol.md "Ask
// mode"): it steps a box through Plan first, Ask, then off. Plan first is
// the thread's own permission mode (the agent plans and waits for Go ahead
// on its plan card); stepping on from it puts back the access it had. Ask
// sends each message read-only, just to be answered. Off is as before.

/// Where a box is now, and what Shift-Tab makes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    /// The permission mode to set, if it changes.
    pub permission: Option<String>,
    /// Ask mode on after the step.
    pub ask: bool,
}

/// The next step from `permission` (the box's access) with Ask `ask`, and
/// `put_back`, the access it had before going into Plan first. With none
/// noted (it went in before a restart, or the runner put it there) it goes
/// back to Ask before edits: never more access than it may have had
/// (Wayne, 7 Nov 2026: falling back to Full access raised it).
pub fn next(permission: &str, ask: bool, put_back: Option<&str>) -> Step {
    if permission == "plan" {
        Step { permission: Some(put_back.filter(|m| *m != "plan").unwrap_or("ask").to_owned()), ask: true }
    } else if ask {
        Step { permission: None, ask: false }
    } else {
        Step { permission: Some("plan".to_owned()), ask: false }
    }
}

/// Whether a message was sent in Ask mode, as its bubble says: "ask", or
/// an older app's per-message "plan", which the runner takes as Ask.
pub fn sent_in_ask(mode: Option<&str>) -> bool {
    matches!(mode, Some("ask" | "plan"))
}

pub const ASK_TITLE: &str = "Ask";
pub const ASK_TIP: &str = "Ask mode: nothing changes; it just answers. Shift-Tab to turn it off";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shift_tab_steps_through_plan_first_then_ask_then_off() {
        let s = next("full", false, None);
        assert_eq!(s, Step { permission: Some("plan".into()), ask: false });
        let s = next("plan", false, Some("full"));
        assert_eq!(s, Step { permission: Some("full".into()), ask: true }, "the access it had comes back");
        let s = next("full", true, None);
        assert_eq!(s, Step { permission: None, ask: false });
    }

    #[test]
    fn plan_first_with_nothing_noted_goes_back_to_ask_before_edits() {
        assert_eq!(next("plan", false, None).permission.as_deref(), Some("ask"));
        assert_eq!(next("plan", false, Some("plan")).permission.as_deref(), Some("ask"));
    }

    #[test]
    fn older_plan_messages_read_as_ask() {
        assert!(sent_in_ask(Some("ask")) && sent_in_ask(Some("plan")));
        assert!(!sent_in_ask(None) && !sent_in_ask(Some("other")));
    }
}
