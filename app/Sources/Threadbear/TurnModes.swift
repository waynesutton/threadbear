import AppKit
import ThreadbearCore

/// Shift-Tab in the composer steps off, Plan first, Ask, off (docs/protocol.md,
/// "Ask mode"). Plan first is the thread's own permission mode (Mike's: the
/// agent plans and waits for Go ahead on its plan card, and the plan is also
/// saved as plans/<date>-<project>.md); stepping on from it puts back the mode
/// it had. Ask sends each message read-only, just to be answered, with its
/// chip beside the permission one. Off is as before. Settings > General can
/// turn Shift-Tab's modes off altogether.
enum TurnModes {
    static let enabledKey = "turnModesEnabled"
    static var enabled: Bool { UserDefaults.standard.object(forKey: enabledKey) as? Bool ?? true }
}

extension TurnMode {
    var title: String {
        switch self {
        case .plan: "Plan"
        case .ask: "Ask"
        }
    }

    var symbol: String {
        switch self {
        case .plan: "list.bullet.clipboard"
        case .ask: "questionmark.bubble"
        }
    }

    var help: String {
        switch self {
        case .plan: "Plan mode"
        case .ask: "Ask mode: nothing changes; it just answers. Shift-Tab to turn it off"
        }
    }
}

extension AppModel {
    /// The composer's Ask mode, while the modes are on.
    func turnMode(for key: String) -> TurnMode? {
        TurnModes.enabled && turnModes[key] == .ask ? .ask : nil
    }

    /// Shift-Tab: off, Plan first, Ask, off for that composer (a thread's id,
    /// or "new"). False when the modes are off, so Shift-Tab does what it did.
    @discardableResult
    func cycleTurnMode(_ key: String) -> Bool {
        guard TurnModes.enabled else { return false }
        let mode = permission(for: key)
        if mode == .plan {
            // Plan first to Ask: back to the access it had before. When this
            // app never saw it go into Plan first (it was put there before a
            // restart, or by the runner), Ask before edits: never more
            // access than it may have had. Falling back to Full access
            // raised it (Wayne, 7 Nov 2026).
            setPermission(for: key, planPutBack[key] ?? .ask)
            turnModes[key] = .ask
        } else if turnModes[key] == .ask {
            turnModes[key] = nil
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
