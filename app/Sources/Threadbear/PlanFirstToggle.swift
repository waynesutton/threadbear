import AppKit
import ThreadbearCore

/// Shift-Tab in the composer turns Plan first on or off, as in Claude Code.
/// Plan first is the thread's own permission mode (Mike's: the agent plans
/// and waits for Go ahead on its plan card, and the plan is also saved as
/// plans/<date>-<project>.md); turning it off puts back the mode it had.
/// Settings > General can give Shift-Tab back to the text box.
enum PlanFirstToggle {
    // The key from when Shift-Tab also stepped through Ask mode, so the
    // setting carries over.
    static let enabledKey = "turnModesEnabled"
    static var enabled: Bool { UserDefaults.standard.object(forKey: enabledKey) as? Bool ?? true }
}

extension AppModel {
    /// Shift-Tab: Plan first on or off for that composer (a thread's id, or
    /// "new"). False when Settings has it off, so Shift-Tab does what it did.
    @discardableResult
    func togglePlanFirst(_ key: String) -> Bool {
        guard PlanFirstToggle.enabled else { return false }
        if permission(for: key) == .plan {
            // Back to the access it had before. When this app never saw it
            // go into Plan first (it was put there before a restart, or by
            // the runner), Ask before edits: never more access than it may
            // have had. Falling back to Full access raised it (Wayne, 7 Nov 2026).
            setPermission(for: key, planPutBack[key] ?? .ask)
        } else {
            setPermission(for: key, .plan)
        }
        return true
    }

    private func permission(for key: String) -> PermissionMode {
        key == "new" ? newThreadPermission : threads[key]?.permissionMode ?? .full
    }

    /// A composer's permission mode, from Shift-Tab or the access menu
    /// alike, so going into Plan first either way notes the mode to put back.
    func setPermission(for key: String, _ mode: PermissionMode) {
        let from = permission(for: key)
        if mode == .plan {
            if from != .plan { planPutBack[key] = from }
        } else {
            planPutBack.removeValue(forKey: key)
        }
        if key == "new" { newThreadPermission = mode } else { setPermission(key, mode) }
    }
}
