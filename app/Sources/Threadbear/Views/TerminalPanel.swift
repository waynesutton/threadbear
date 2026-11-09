import AppKit
import SwiftTerm
import SwiftUI
import ThreadbearCore

/// Real terminals (SwiftTerm) for the thread, each running his login shell
/// in the thread's folder, as Terminal.app would: his PATH, nvm and aliases,
/// colours, and anything full-screen (vim, top). A thread can have several,
/// as tabs along the top with + for another, so a dev server keeps running
/// in one while he works in the next. They're kept for as long as the app
/// runs, so moving to another thread and back finds them as they were. A
/// shell that exits closes its tab; the last one starts again on the next
/// look. ⌘-click opens a web address in the browser (TerminalLinks). Only
/// for threads on this Mac: another Mac's folder isn't here to run a shell in.
struct TerminalPanel: View {
    let thread: ThreadSummary
    @ObservedObject private var sessions = TerminalSessions.shared

    var body: some View {
        let tabs = sessions.tabs(for: thread)
        let current = sessions.current(for: thread)
        VStack(spacing: 0) {
            TerminalTabStrip(thread: thread, tabs: tabs, current: current.id)
            Divider()
            TerminalHost(views: tabs.map(\.view), current: current.view)
        }
        .onAppear { sessions.dropEnded(for: thread) }
    }
}

/// The tabs above the terminal: one per shell, ✕ to close one, + for another.
private struct TerminalTabStrip: View {
    let thread: ThreadSummary
    let tabs: [TerminalTab]
    let current: UUID
    private var sessions: TerminalSessions { .shared }

    var body: some View {
        HStack(spacing: zoomed(2)) {
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: zoomed(2)) {
                    ForEach(tabs) { tab in
                        TerminalTabButton(
                            tab: tab, selected: tab.id == current, closable: tabs.count > 1,
                            select: { sessions.select(tab.id, in: thread) },
                            close: { sessions.close(tab.id, in: thread) }
                        )
                    }
                }
                .padding(.horizontal, zoomed(6))
            }
            Button { sessions.add(for: thread) } label: { Image(systemName: "plus") }
                .buttonStyle(RowIconButtonStyle())
                .tip("Open another terminal in this thread's folder")
                .accessibilityLabel("New terminal")
                .padding(.trailing, zoomed(6))
        }
        .frame(height: zoomed(30))
    }
}

/// One tab: click to pick it, double-click (or Rename… in its right-click
/// menu) to give it a name of your own, ✕ to close it.
private struct TerminalTabButton: View {
    @ObservedObject var tab: TerminalTab
    let selected: Bool
    let closable: Bool
    let select: () -> Void
    let close: () -> Void
    @State private var hovering = false
    @State private var renaming = false
    @State private var draft = ""
    @FocusState private var fieldFocused: Bool

    var body: some View {
        HStack(spacing: zoomed(4)) {
            if renaming {
                TextField("Name", text: $draft)
                    .textFieldStyle(.plain)
                    .font(.system(size: zoomed(11.5), weight: .medium))
                    .frame(width: zoomed(120))
                    .focused($fieldFocused)
                    .onSubmit(commit)
                    .onExitCommand { renaming = false }
                    .onChange(of: fieldFocused) { _, focused in if !focused && renaming { commit() } }
                    .tip("Press Return to keep the name, Escape to leave it. Empty goes back to the shell's own")
            } else {
                Text(tab.title)
                    .font(.system(size: zoomed(11.5), weight: selected ? .medium : .regular))
                    .foregroundStyle(selected ? .primary : .secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .frame(maxWidth: zoomed(140), alignment: .leading)
            }
            if closable {
                Button(action: close) {
                    Image(systemName: "xmark")
                        .font(.system(size: zoomed(8.5), weight: .semibold))
                        .frame(width: zoomed(14), height: zoomed(14))
                        .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .foregroundStyle(.secondary)
                .opacity(hovering || selected ? 1 : 0)
                .tip("Close this terminal and end its shell")
                .accessibilityLabel("Close \(tab.title)")
            }
        }
        .padding(.leading, zoomed(8))
        .padding(.trailing, zoomed(closable ? 4 : 8))
        .frame(height: zoomed(22))
        .background(
            Color.primary.opacity(selected ? 0.1 : hovering ? 0.05 : 0),
            in: RoundedRectangle(cornerRadius: zoomed(5))
        )
        .contentShape(Rectangle())
        // Both at once, so a single click picks the tab without waiting to
        // see whether a second click is coming.
        .onTapGesture(perform: select)
        .simultaneousGesture(TapGesture(count: 2).onEnded { startRenaming() })
        .onHover { hovering = $0 }
        .contextMenu {
            Button("Rename…", action: startRenaming)
            if tab.name != nil { Button("Use the Shell's Name") { tab.name = nil } }
            if closable {
                Divider()
                Button("Close Terminal", action: close)
            }
        }
        .tip(tab.ended ? "This shell has ended" : "Switch to \(tab.title). Double-click to rename it")
        .accessibilityElement(children: .combine)
        .accessibilityAddTraits(selected ? [.isButton, .isSelected] : .isButton)
        .accessibilityAction(named: "Rename") { startRenaming() }
        .accessibilityAction(named: "Close") { if closable { close() } }
    }

    private func startRenaming() {
        select()
        draft = tab.title
        renaming = true
        DispatchQueue.main.async { fieldFocused = true }
    }

    private func commit() {
        guard renaming else { return }
        renaming = false
        let name = draft.trimmingCharacters(in: .whitespacesAndNewlines)
        tab.name = name.isEmpty ? nil : String(name.prefix(60))
    }
}

/// Holds all of a thread's terminals, the picked one showing. The others
/// stay in place, hidden, at the same size: a terminal taken out and put
/// back can be squeezed to nothing in between, which loses its screen.
private struct TerminalHost: NSViewRepresentable {
    let views: [ThreadTerminalView]
    let current: ThreadTerminalView

    func makeNSView(context: Context) -> NSView {
        let box = NSView()
        box.wantsLayer = true
        attach(to: box)
        return box
    }

    func updateNSView(_ box: NSView, context: Context) {
        attach(to: box)
    }

    private func attach(to box: NSView) {
        for old in box.subviews where !views.contains(where: { $0 === old }) { old.removeFromSuperview() }
        // The terminal takes the keys when it's put here or picked, not on
        // every update (that would pull the cursor out of the message box).
        var focus = current.superview !== box
        for view in views where view.superview !== box {
            view.removeFromSuperview()
            if !box.bounds.isEmpty { view.frame = box.bounds }
            view.translatesAutoresizingMaskIntoConstraints = false
            box.addSubview(view)
            NSLayoutConstraint.activate([
                view.leadingAnchor.constraint(equalTo: box.leadingAnchor),
                view.trailingAnchor.constraint(equalTo: box.trailingAnchor),
                view.topAnchor.constraint(equalTo: box.topAnchor),
                view.bottomAnchor.constraint(equalTo: box.bottomAnchor),
            ])
        }
        for view in views {
            let hide = view !== current
            if view.isHidden != hide { view.isHidden = hide; focus = focus || !hide }
        }
        if focus {
            DispatchQueue.main.async { [current] in
                if !current.isHidden { current.window?.makeFirstResponder(current) }
            }
        }
    }
}

/// A terminal that knows what's printed where: a web address under the
/// pointer is underlined, with a pointing hand and a tip, and ⌘-click
/// opens it (TerminalSessions.watchForLinkClicks).
final class ThreadTerminalView: LocalProcessTerminalView {
    private lazy var hover = LinkHover(terminal: self)
    private let underline = LinkUnderlineView()

    /// Starts following the pointer for links. Its own tracking area, as
    /// SwiftTerm's mouseMoved can't be overridden from here.
    func watchLinks() {
        underline.isHidden = true
        addSubview(underline)
        addTrackingArea(NSTrackingArea(rect: .zero, options: [.mouseMoved, .mouseEnteredAndExited, .activeInActiveApp, .inVisibleRect], owner: hover))
    }

    private var cell: CGSize {
        let font = self.font as CTFont
        let height = ceil(CTFontGetAscent(font) + CTFontGetDescent(font) + CTFontGetLeading(font))
        let width = self.font.advancement(forGlyph: self.font.glyph(withName: "W")).width
        return CGSize(width: width, height: height)
    }

    /// The web address under a point in the view, from a link the program
    /// marked (OSC 8) or from the text there, or nil. Only http and https.
    func webLink(at point: NSPoint) -> URL? { linkSpan(at: point)?.url }

    /// The link under a point: its row on screen, the cells it covers and
    /// where it goes.
    func linkSpan(at point: NSPoint) -> (row: Int, columns: Range<Int>, url: URL)? {
        let cell = self.cell
        guard cell.height > 0, cell.width > 0, bounds.contains(point) else { return nil }
        let term = getTerminal()
        let row = Int((bounds.height - point.y) / cell.height)
        let col = Int(point.x / cell.width)
        guard row >= 0, row < term.rows, col >= 0, col < term.cols, let line = term.getLine(row: row) else { return nil }
        let count = min(term.cols, line.count)
        guard col < count else { return nil }
        if let payload = line[col].getPayload() as? String {
            guard let marked = payload.split(separator: ";", maxSplits: 1, omittingEmptySubsequences: false).last,
                  let url = LinkPolicy.web(String(marked)) else { return nil }
            var start = col, end = col + 1
            while start > 0, line[start - 1].getPayload() as? String == payload { start -= 1 }
            while end < count, line[end].getPayload() as? String == payload { end += 1 }
            return (row, start..<end, url)
        }
        let text = String((0..<count).map { i -> Character in
            let c = line[i].getCharacter()
            return c == "\0" ? " " : c
        })
        return TerminalLinks.span(in: text, at: col).map { (row, $0.columns, $0.url) }
    }

    /// Underlines the link under the point, or takes the underline away.
    func showHover(at point: NSPoint?) {
        guard let point, let span = linkSpan(at: point) else {
            if !underline.isHidden { underline.isHidden = true; window?.invalidateCursorRects(for: underline) }
            return
        }
        let cell = self.cell
        let frame = NSRect(x: CGFloat(span.columns.lowerBound) * cell.width,
                           y: bounds.height - CGFloat(span.row + 1) * cell.height,
                           width: CGFloat(span.columns.count) * cell.width, height: cell.height)
        if underline.isHidden || underline.frame != frame || underline.url != span.url {
            underline.frame = frame
            underline.url = span.url
            underline.isHidden = false
            underline.needsDisplay = true
            window?.invalidateCursorRects(for: underline)
        }
        NSCursor.pointingHand.set()
    }

    /// The underlined link right now, for the automation's `terminal` command.
    var hoveredLink: URL? { underline.isHidden ? nil : underline.url }

    /// Every link ⌘-click would find, trying the middle of each cell on
    /// screen (for the automation's `terminal` command).
    func webLinksOnScreen() -> [URL] {
        let term = getTerminal()
        let cell = self.cell
        var found: [URL] = []
        for row in 0..<term.rows {
            for col in 0..<term.cols {
                let point = NSPoint(x: (CGFloat(col) + 0.5) * cell.width, y: bounds.height - (CGFloat(row) + 0.5) * cell.height)
                if let url = webLink(at: point), found.last != url { found.append(url) }
            }
        }
        return found
    }

    /// The middle of the first cell of the first link on screen, for the
    /// automation to hover over.
    func firstLinkPoint() -> NSPoint? {
        let term = getTerminal()
        let cell = self.cell
        for row in 0..<term.rows {
            for col in 0..<term.cols {
                let point = NSPoint(x: (CGFloat(col) + 0.5) * cell.width, y: bounds.height - (CGFloat(row) + 0.5) * cell.height)
                if webLink(at: point) != nil { return point }
            }
        }
        return nil
    }
}

/// Gets the pointer's moves over a terminal and passes them back to it.
private final class LinkHover: NSResponder {
    weak var terminal: ThreadTerminalView?

    init(terminal: ThreadTerminalView) {
        self.terminal = terminal
        super.init()
    }

    required init?(coder: NSCoder) { nil }

    override func mouseMoved(with event: NSEvent) { follow(event) }
    override func mouseEntered(with event: NSEvent) { follow(event) }
    override func mouseExited(with event: NSEvent) { terminal?.showHover(at: nil) }

    private func follow(_ event: NSEvent) {
        guard let terminal, event.window === terminal.window else { return }
        terminal.showHover(at: terminal.convert(event.locationInWindow, from: nil))
    }
}

/// The line under a link the pointer is over. Clicks go through it to the
/// terminal; it gives the pointing hand and the tip.
private final class LinkUnderlineView: NSView {
    var url: URL? {
        didSet { if url != oldValue { setTip(url.map { "⌘-click to open \(LinkPolicy.shown($0))" } ?? "") } }
    }

    override func hitTest(_ point: NSPoint) -> NSView? { nil }

    override func resetCursorRects() {
        addCursorRect(bounds, cursor: .pointingHand)
    }

    override func draw(_ dirtyRect: NSRect) {
        NSColor.linkColor.setFill()
        NSRect(x: 0, y: 1, width: bounds.width, height: 1).fill()
    }
}

/// One shell in a thread's Terminal panel.
@MainActor
final class TerminalTab: ObservableObject, Identifiable {
    let id = UUID()
    let view: ThreadTerminalView
    let number: Int
    @Published var shellTitle = ""
    /// A name he gave it (double-click or Rename…), over the shell's own.
    @Published var name: String?
    @Published var ended = false

    init(view: ThreadTerminalView, number: Int) {
        self.view = view
        self.number = number
    }

    var title: String {
        if let name { return name }
        let own = shellTitle.trimmingCharacters(in: .whitespacesAndNewlines)
        return own.isEmpty ? "Terminal \(number)" : own
    }
}

@MainActor
final class TerminalSessions: NSObject, ObservableObject, LocalProcessTerminalViewDelegate {
    static let shared = TerminalSessions()
    private var byThread: [String: [TerminalTab]] = [:]
    private var selected: [String: UUID] = [:]
    private var linkMonitor: Any?

    override init() {
        super.init()
        watchForLinkClicks()
    }

    /// The thread's terminals, starting the first if it has none.
    func tabs(for thread: ThreadSummary) -> [TerminalTab] {
        if byThread[thread.id]?.isEmpty ?? true { byThread[thread.id] = [make(for: thread)] }
        return byThread[thread.id] ?? []
    }

    func current(for thread: ThreadSummary) -> TerminalTab {
        let tabs = tabs(for: thread)
        return tabs.first { $0.id == selected[thread.id] } ?? tabs[tabs.count - 1]
    }

    func add(for thread: ThreadSummary) {
        objectWillChange.send()
        let tab = make(for: thread)
        byThread[thread.id, default: []].append(tab)
        selected[thread.id] = tab.id
    }

    func select(_ id: UUID, in thread: ThreadSummary) {
        objectWillChange.send()
        selected[thread.id] = id
    }

    func close(_ id: UUID, in thread: ThreadSummary) {
        guard let tab = byThread[thread.id]?.first(where: { $0.id == id }) else { return }
        objectWillChange.send()
        remove(tab, from: thread.id)
        if !tab.ended { tab.view.process.terminate() }
    }

    /// Opening the panel again: shells that ended go, and the panel starts
    /// a fresh one if that was all there was.
    func dropEnded(for thread: ThreadSummary) {
        guard let tabs = byThread[thread.id], tabs.contains(where: \.ended) else { return }
        objectWillChange.send()
        byThread[thread.id] = tabs.filter { !$0.ended }
    }

    private func remove(_ tab: TerminalTab, from threadId: String) {
        guard var tabs = byThread[threadId], let index = tabs.firstIndex(where: { $0.id == tab.id }) else { return }
        tabs.remove(at: index)
        byThread[threadId] = tabs
        if selected[threadId] == tab.id, !tabs.isEmpty {
            selected[threadId] = tabs[min(index, tabs.count - 1)].id
        }
    }

    private func make(for thread: ThreadSummary) -> TerminalTab {
        // The lowest number no open tab has, as Terminal.app's windows do.
        let taken = Set((byThread[thread.id] ?? []).map(\.number))
        let number = (1...).first { !taken.contains($0) } ?? 1
        let view = ThreadTerminalView(frame: NSRect(x: 0, y: 0, width: 400, height: 300))
        view.processDelegate = self
        view.font = .monospacedSystemFont(ofSize: TypeScale.scaled(12), weight: .regular)
        view.nativeBackgroundColor = .textBackgroundColor
        view.nativeForegroundColor = .textColor
        view.caretColor = .controlAccentColor
        view.setAccessibilityLabel("Terminal \(number) for \(thread.title)")
        view.watchLinks()
        let shell = Self.shell
        var env = ProcessInfo.processInfo.environment
        env["TERM"] = "xterm-256color"
        env["COLORTERM"] = "truecolor"
        env["TERM_PROGRAM"] = "Threadbear"
        // So the thread's own variables (THREADBEAR_THREAD_ID and the like) don't leak in.
        for key in env.keys where key.hasPrefix("THREADBEAR_") || key.hasPrefix("CLAUDE") { env[key] = nil }
        let cwd = (thread.cwd as NSString).expandingTildeInPath
        view.startProcess(executable: shell, args: ["-l"], environment: env.map { "\($0.key)=\($0.value)" },
                          execName: "-" + (shell as NSString).lastPathComponent,
                          currentDirectory: FileManager.default.fileExists(atPath: cwd) ? cwd : NSHomeDirectory())
        return TerminalTab(view: view, number: number)
    }

    private func tab(for view: TerminalView) -> (tab: TerminalTab, threadId: String)? {
        for (threadId, tabs) in byThread {
            if let tab = tabs.first(where: { $0.view === view }) { return (tab, threadId) }
        }
        return nil
    }

    /// ⌘-click on a web address in any terminal opens it in the browser,
    /// through OpenGate. Taken before SwiftTerm sees it, as SwiftTerm's own
    /// ⌘-click would hand a marked link of any kind (`file:`, an app's
    /// scheme) straight to macOS. A ⌘-click that isn't on a web address
    /// goes on to the terminal as usual.
    private func watchForLinkClicks() {
        linkMonitor = NSEvent.addLocalMonitorForEvents(matching: .leftMouseUp) { event in
            guard event.modifierFlags.contains(.command), let window = event.window else { return event }
            let hit = window.contentView?.hitTest(window.contentView?.superview?.convert(event.locationInWindow, from: nil) ?? event.locationInWindow)
            var view = hit
            while let current = view, !(current is ThreadTerminalView) { view = current.superview }
            guard let term = view as? ThreadTerminalView else { return event }
            let point = term.convert(event.locationInWindow, from: nil)
            guard let url = MainActor.assumeIsolated({ term.webLink(at: point) }) else { return event }
            MainActor.assumeIsolated { _ = OpenGate.openWeb(url) }
            return nil
        }
    }

    /// His login shell, as Terminal.app uses.
    static var shell: String {
        if let own = ProcessInfo.processInfo.environment["SHELL"], FileManager.default.isExecutableFile(atPath: own) { return own }
        return "/bin/zsh"
    }

    /// Every shell, when the app quits.
    func endAll() {
        for tabs in byThread.values { for tab in tabs where !tab.ended { tab.view.process.terminate() } }
        byThread = [:]
    }

    nonisolated func sizeChanged(source: LocalProcessTerminalView, newCols: Int, newRows: Int) {}
    nonisolated func setTerminalTitle(source: LocalProcessTerminalView, title: String) {
        MainActor.assumeIsolated { tab(for: source)?.tab.shellTitle = title }
    }
    nonisolated func hostCurrentDirectoryUpdate(source: TerminalView, directory: String?) {}
    nonisolated func processTerminated(source: TerminalView, exitCode: Int32?) {
        MainActor.assumeIsolated {
            guard let (tab, threadId) = tab(for: source) else { return }
            tab.ended = true
            if (byThread[threadId]?.count ?? 0) > 1 {
                // `exit` in one of several: its tab goes, like Terminal.app's.
                objectWillChange.send()
                remove(tab, from: threadId)
            } else {
                // The last one: a fresh shell the next time the panel's opened.
                source.feed(text: "\r\n[The shell ended. Open Terminal again for a new one.]\r\n")
            }
        }
    }
}
