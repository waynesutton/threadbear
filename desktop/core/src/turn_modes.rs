// Shift-Tab in the message box (PlanFirstToggle.swift): Plan first on or
// off, as in Claude Code. Plan first is the thread's own permission mode
// (the agent plans and waits for Go ahead on its plan card); turning it off
// puts back the access it had. Ask mode, which Shift-Tab once stepped
// through as well, is gone from the runner (2ee83b55), so it's gone here.

/// The access Shift-Tab gives a box whose access is `permission`, with
/// `put_back` the access it had before going into Plan first. With none
/// noted (it went in before a restart, or the runner put it there) it goes
/// back to Ask before edits: never more access than it may have had
/// (Wayne, 7 Nov 2026: falling back to Full access raised it).
pub fn next(permission: &str, put_back: Option<&str>) -> String {
    if permission == "plan" {
        put_back.filter(|m| *m != "plan").unwrap_or("ask").to_owned()
    } else {
        "plan".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shift_tab_turns_plan_first_on_then_puts_the_access_back() {
        assert_eq!(next("full", None), "plan");
        assert_eq!(next("plan", Some("full")), "full", "the access it had comes back");
        assert_eq!(next("auto", None), "plan");
    }

    #[test]
    fn plan_first_with_nothing_noted_goes_back_to_ask_before_edits() {
        assert_eq!(next("plan", None), "ask");
        assert_eq!(next("plan", Some("plan")), "ask");
    }
}
