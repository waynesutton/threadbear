import Foundation
import ThreadbearCore

/// Bears (docs/protocol.md, "Bears"): the sidebar's Bear Colour menu.
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
}
