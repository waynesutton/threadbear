import AppKit
import SwiftUI

/// A reply's diagram or page in a window of its own, most of the screen,
/// like a design review (Mike, 8 Nov): beside the thread in the pane it was
/// still too small to read a big diagram.
@MainActor
final class RichBlockWindowController: NSWindowController, NSWindowDelegate {
    /// One window per block: opening it again brings that one forward.
    private static var open: [String: RichBlockWindowController] = [:]

    /// The blocks open full size, for the scripted app's dump.
    static var openIds: [String] { open.keys.sorted() }

    static func show(_ page: URL, id: String, title: String, subtitle: String?) {
        if let existing = open[id] {
            existing.refresh.count += 1
            existing.present()
            return
        }
        let controller = RichBlockWindowController(page: page, title: title, subtitle: subtitle)
        controller.onClose = { open.removeValue(forKey: id) }
        open[id] = controller
        controller.present()
    }

    /// Bumped to fetch the page's view from the pool again: the pool keeps
    /// only a few, and one dropped while this window was behind would leave
    /// it blank.
    final class Refresh: ObservableObject { @Published var count = 0 }
    private let refresh = Refresh()
    private let shown: Shown
    private var onClose: (() -> Void)?

    private init(page: URL, title: String, subtitle: String?) {
        shown = Shown(page)
        let screen = NSApp.keyWindow?.screen ?? NSApp.mainWindow?.screen ?? NSScreen.main
        let frame = ReviewWindowController.frame(on: screen?.visibleFrame ?? NSRect(x: 0, y: 0, width: 1240 / 0.9, height: 860 / 0.9))
        let window = NSWindow(contentRect: frame, styleMask: [.titled, .closable, .miniaturizable, .resizable], backing: .buffered, defer: false)
        window.title = title
        if let subtitle { window.subtitle = subtitle }
        window.minSize = NSSize(width: 480, height: 360)
        window.isReleasedWhenClosed = false
        window.collectionBehavior = [.fullScreenPrimary]
        super.init(window: window)
        window.delegate = self
        window.contentView = NSHostingView(rootView: Body(shown: shown, refresh: refresh))
        window.setFrame(frame, display: false)
    }

    required init?(coder: NSCoder) { fatalError() }

    private func present() {
        #if DEBUG
        if Automation.instanceId != nil {
            window?.tuckBehind()
            return
        }
        #endif
        showWindow(nil)
        window?.makeKeyAndOrderFront(nil)
    }

    func windowDidBecomeKey(_ notification: Notification) { refresh.count += 1 }

    func windowWillClose(_ notification: Notification) { onClose?() }

    private struct Body: View {
        @ObservedObject var shown: Shown
        @ObservedObject var refresh: Refresh

        var body: some View { LocalPagePreview(url: shown.page, opened: refresh.count) }
    }

    /// The page on show, which a todo list or board moves to its new version
    /// after a click (follow).
    final class Shown: ObservableObject {
        @Published var page: URL
        init(_ page: URL) { self.page = page }
    }

    /// A Print's window showing version `from` moves to `to`, the version a
    /// click in it just saved, so the next click isn't from an old one.
    static func follow(pageId: String, from: Int, to page: URL, version: Int) {
        let old = "page:\(pageId):\(from)"
        guard let controller = open.removeValue(forKey: old) else { return }
        let new = "page:\(pageId):\(version)"
        open[new] = controller
        controller.onClose = { open.removeValue(forKey: new) }
        controller.shown.page = page
        controller.window?.subtitle = "Version \(version)"
    }
}
