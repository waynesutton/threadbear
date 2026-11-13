import AppKit
import SwiftUI
import ThreadbearCore

/// The composer's @ list (ComposerTextView): Wayne's bears matching what's
/// typed after the @, in a small window just over the cursor, like the /
/// list (SkillPicker). Picking one writes "@Name ", which the runner reads
/// when the message is sent and hands to that bear (docs/protocol.md,
/// "Bears"). It never takes the keyboard: the text view passes it ↑↓,
/// Return, Tab and Esc.
@MainActor
final class BearPicker {
    private let pick: (BearEntry) -> Void
    private var window: NSPanel?
    private let model = Model()

    @Observable
    final class Model {
        var matches: [BearEntry] = []
        var selected = 0
    }

    init(pick: @escaping (BearEntry) -> Void) { self.pick = pick }

    var isShown: Bool { window?.isVisible == true }
    var selected: BearEntry? { model.matches.indices.contains(model.selected) ? model.matches[model.selected] : nil }

    /// Names starting with what's typed first (the head bear first among
    /// them), then ones with a word starting with it.
    static func matches(_ query: String, in bears: [BearEntry]) -> [BearEntry] {
        let q = query.lowercased()
        let sorted = bears.sorted { ($0.head == true ? 0 : 1, $0.name) < ($1.head == true ? 0 : 1, $1.name) }
        let starts = sorted.filter { $0.name.lowercased().hasPrefix(q) }
        let words = sorted.filter { bear in
            !bear.name.lowercased().hasPrefix(q) && bear.name.lowercased().split(separator: " ").contains { $0.hasPrefix(q) }
        }
        return Array((starts + words).prefix(8))
    }

    func show(_ matches: [BearEntry], above caret: NSRect, parent: NSWindow?) {
        let kept = selected?.id
        model.matches = matches
        model.selected = kept.flatMap { id in matches.firstIndex { $0.id == id } } ?? 0
        let panel = window ?? makeWindow()
        let height = CGFloat(min(matches.count, 8)) * 44 + 12
        panel.setFrame(NSRect(x: caret.minX - 8, y: caret.maxY + 6, width: 320, height: height), display: true)
        if let parent, panel.parent !== parent { parent.addChildWindow(panel, ordered: .above) }
        panel.orderFront(nil)
    }

    func move(_ by: Int) {
        guard !model.matches.isEmpty else { return }
        model.selected = (model.selected + by + model.matches.count) % model.matches.count
    }

    func hide() {
        guard let window, window.isVisible else { return }
        window.parent?.removeChildWindow(window)
        window.orderOut(nil)
    }

    private func makeWindow() -> NSPanel {
        let panel = NSPanel(contentRect: .zero, styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: true)
        panel.isFloatingPanel = true
        panel.hasShadow = true
        panel.backgroundColor = .clear
        panel.isOpaque = false
        panel.becomesKeyOnlyIfNeeded = true
        panel.contentView = NSHostingView(rootView: BearPickerList(model: model) { [weak self] bear in self?.pick(bear) })
        window = panel
        return panel
    }
}

private struct BearPickerList: View {
    let model: BearPicker.Model
    let pick: (BearEntry) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            ForEach(Array(model.matches.enumerated()), id: \.element.id) { index, bear in
                HStack(spacing: 8) {
                    BearTile(bear: bear.tile, size: 26)
                    VStack(alignment: .leading, spacing: 1) {
                        Text("@" + bear.name)
                            .font(.body.weight(.semibold))
                        if let does = bear.does {
                            Text(does)
                                .font(.caption)
                                .lineLimit(1)
                                .opacity(0.8)
                        }
                    }
                    Spacer(minLength: 0)
                }
                .foregroundStyle(index == model.selected ? Color.white : Color.primary)
                .padding(.horizontal, 8)
                .frame(height: 44)
                .background(RoundedRectangle(cornerRadius: 6).fill(index == model.selected ? Color.accentColor : .clear))
                .contentShape(Rectangle())
                .onTapGesture { pick(bear) }
                .tip("Ask \(bear.name): it gets this message when you send it")
            }
        }
        .padding(6)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .background(RoundedRectangle(cornerRadius: 10).fill(.regularMaterial))
        .overlay(RoundedRectangle(cornerRadius: 10).strokeBorder(Color.primary.opacity(0.1)))
    }
}
