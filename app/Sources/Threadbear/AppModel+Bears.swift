import Foundation
import ThreadbearCore

/// Bears (docs/protocol.md, "Bears"): the sidebar's Bear Colour menu and
/// the composer's @ list.
extension AppModel {
    /// Colours a bear's tile in the sidebar. Its threads' rows change when
    /// the runner sends them again, so the banner says it's on its way.
    func setBearColor(threadId: String, bear: ThreadBear, color: String?) {
        guard let client = client(for: threadId) else { return }
        whileBusy("bearColor/\(bear.id)", note: "Colouring \(bear.name)…") { [self] in
            do {
                try await client.setBearColor(bear.id, color: color)
                flashDone("\(bear.name)'s colour changed")
            } catch {
                show(error)
            }
        }
    }

    /// Fetches the bears for the @ list, at most every 20 seconds. A read
    /// that fails leaves the list as it was: the list is only a shortcut
    /// for typing a name, which works without it.
    func refreshBears() {
        guard let client, !BearList.fetching, Date().timeIntervalSince(BearList.fetchedAt) > 20 else { return }
        BearList.fetching = true
        Task { [weak self] in
            let bears = try? await client.listBears()
            BearList.fetching = false
            BearList.fetchedAt = Date()
            if let bears { BearList.bears = bears }
            _ = self
            NotificationCenter.default.post(name: BearList.changed, object: nil)
        }
    }
}

/// The bears the composer's @ list offers, as last fetched.
@MainActor
enum BearList {
    static var bears: [BearEntry] = []
    static var fetchedAt = Date.distantPast
    static var fetching = false
    /// Asks the app for the list again; set by the AppModel at start.
    static var refresh: (() -> Void)?
    static let changed = Notification.Name("BearList.changed")
}
