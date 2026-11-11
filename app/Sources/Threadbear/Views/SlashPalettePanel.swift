import AppKit
import SwiftUI
import ThreadbearCore

/// The slash command palette's own window: a borderless child panel over
/// the message box. Drawn inside the transcript's pane it was cut off at
/// the pane's edges, so it has a window of its own, which can be as wide
/// as the whole window. Its edges are PaletteFrame's wide bands rather
/// than the system's few points (Mike, 7 Nov: "the hit area for the drag
/// handles is small"). It only becomes key while its search field is in
/// use (PaletteSearchField); otherwise the message box keeps the
/// cursor and the palette's keys.
final class SlashPalettePanel: NSPanel {
    init() {
        super.init(contentRect: .zero, styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: true)
        isOpaque = false
        backgroundColor = .clear
        hasShadow = true
        hidesOnDeactivate = false
        isMovable = false
        isReleasedWhenClosed = false
        becomesKeyOnlyIfNeeded = true
        animationBehavior = .none
        minSize = NSSize(width: 480, height: 220)
    }

    /// Set while the palette's search field is in use (PaletteSearchField):
    /// the only time the panel takes the keyboard. Cleared when it loses it.
    var takesKeys = false

    override var canBecomeKey: Bool { takesKeys }
    override var canBecomeMain: Bool { false }
}

/// Puts the palette in its panel, just above the view this sits on (the
/// message box's top edge, as wide as the box), and takes it away when
/// `content` is nil. The size Mike drags it to is kept in `layout`.
struct SlashPaletteHost<Content: View>: NSViewRepresentable {
    var content: Content?
    /// The height worked out for the window, when he hasn't chosen one.
    var autoHeight: CGFloat
    var layout: Binding<SlashPalette.Layout>

    func makeCoordinator() -> SlashPaletteCoordinator { SlashPaletteCoordinator() }

    func makeNSView(context: Context) -> NSView {
        let view = AnchorView()
        view.onMove = { [weak coordinator = context.coordinator] in coordinator?.place() }
        return view
    }

    func updateNSView(_ view: NSView, context: Context) {
        let c = context.coordinator
        c.anchor = view
        c.autoHeight = autoHeight
        c.layout = layout
        if let content { c.show(AnyView(content)) } else { c.hide() }
    }

    static func dismantleNSView(_ view: NSView, coordinator: SlashPaletteCoordinator) {
        coordinator.hide()
    }

    /// Tells the panel to follow when the box moves (the window resized, the
    /// side pane opened).
    final class AnchorView: NSView {
        var onMove: (() -> Void)?
        override func setFrameSize(_ newSize: NSSize) { super.setFrameSize(newSize); onMove?() }
        override func setFrameOrigin(_ newOrigin: NSPoint) { super.setFrameOrigin(newOrigin); onMove?() }
        override func hitTest(_ point: NSPoint) -> NSView? { nil }
    }

}

/// The panel never becomes key, and AppKit hands a click in a window that
/// isn't key only to a view that takes the first click: without this, the
/// palette's rows, buttons and edges got nothing from a real mouse.
final class FirstClickHostingView: NSHostingView<AnyView> {
    override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }
}

/// SlashPaletteHost's: owns the panel, places it, and resizes it.
@MainActor
final class SlashPaletteCoordinator: NSObject, NSWindowDelegate {
    weak var anchor: NSView?
    var autoHeight: CGFloat = 300
    var layout: Binding<SlashPalette.Layout>?
    private var panel: SlashPalettePanel?
    private var hosting: FirstClickHostingView?

    func show(_ content: AnyView) {
        guard let parent = anchor?.window else { return }
        let panel = self.panel ?? makePanel()
        // A window of its own doesn't inherit the main window's openURL, so
        // anything clickable in it goes through OpenGate here too, as a
        // link from no thread (Daybreak, 8 Nov). Descriptions have no links
        // now (SlashPalette.description); this is for whatever comes next.
        hosting?.rootView = AnyView(PaletteFrame(content: content, resize: self)
            .environment(\.openURL, OpenGate.urlAction(files: .unknown)))
        if panel.parent !== parent {
            panel.parent?.removeChildWindow(panel)
            parent.addChildWindow(panel, ordered: .above)
        }
        if resizing == nil { place() }
        if !panel.isVisible { panel.orderFront(nil) }
    }

    func windowDidResignKey(_ notification: Notification) {
        panel?.takesKeys = false
    }

    func hide() {
        guard let panel, panel.isVisible || panel.parent != nil else { return }
        if panel.isKeyWindow { panel.parent?.makeKey() }
        panel.takesKeys = false
        panel.parent?.removeChildWindow(panel)
        panel.orderOut(nil)
    }

    private func makePanel() -> SlashPalettePanel {
        let panel = SlashPalettePanel()
        let hosting = FirstClickHostingView(rootView: AnyView(EmptyView()))
        // The panel sets the size, never the SwiftUI inside it.
        hosting.sizingOptions = []
        panel.contentView = hosting
        panel.delegate = self
        self.panel = panel
        self.hosting = hosting
        return panel
    }

    /// Where it goes: on the box's top edge with a gap, centred on the
    /// box, as wide as the box unless Mike chose a width, and never past
    /// the window's sides or the screen's edges. With the box near the
    /// top of the screen it ran off it and the first rows were cut off
    /// (Wayne, 10 Nov), so it shrinks to the room above, and goes under
    /// the box when there's more room there. The screen is the box's, not
    /// the window's: with the window across two displays, macOS draws it
    /// on one only, and the picker went off on the other (Wayne, 10 Nov).
    func place() {
        guard let panel, let anchor, let parent = anchor.window else { return }
        let box = parent.convertToScreen(anchor.convert(anchor.bounds, to: nil))
        let window = parent.frame
        let boxScreen = NSScreen.screens.first { $0.frame.contains(NSPoint(x: box.midX, y: box.midY)) }
        let screen = (boxScreen ?? parent.screen ?? NSScreen.main)?.visibleFrame ?? window
        let margin: CGFloat = 8, gap = zoomed(8)
        // Sideways it stays inside both the window and the screen.
        let left = max(window.minX, screen.minX) + margin
        let right = min(window.maxX, screen.maxX) - margin
        // A drag going on wins over what's kept.
        let chosen = live ?? layout?.wrappedValue
        var width = CGFloat(chosen?.width.map { SlashPalette.Layout.width($0, window: window.width + 64) } ?? box.width)
        width = max(min(width, right - left), 0)
        let wanted = CGFloat(chosen?.height.map { SlashPalette.Layout.height($0, window: window.height) } ?? autoHeight)
        let above = screen.maxY - margin - (box.maxY + gap)
        let below = (box.minY - gap) - (screen.minY + margin)
        let goBelow = wanted > above && below > above
        let height = max(min(wanted, goBelow ? below : above), 0)
        var x = box.midX - width / 2
        x = min(max(x, left), right - width)
        let y = goBelow ? box.minY - gap - height : box.maxY + gap
        let frame = NSRect(x: x, y: y, width: width, height: height)
        if panel.frame != frame { panel.setFrame(frame, display: true) }
        panel.maxSize = NSSize(width: right - left, height: min(window.height * 0.85, max(above, below)))
    }

    // A drag of one of PaletteFrame's edges. Worked from where the
    // pointer is on the screen, not how far it's moved in the view, as
    // the panel moves and grows under it while it goes. It stays on
    // the box and centred over it: a side grows both sides at once.
    private(set) var resizing: (edge: PaletteFrame.Edge, frame: NSRect, mouse: NSPoint)?
    private var live: SlashPalette.Layout?

    func beginResize(_ edge: PaletteFrame.Edge) {
        guard let panel else { return }
        resizing = (edge, panel.frame, NSEvent.mouseLocation)
    }

    func resizeToPointer() {
        guard let r = resizing else { return }
        let mouse = NSEvent.mouseLocation
        let dx = mouse.x - r.mouse.x, dy = mouse.y - r.mouse.y
        var width = r.frame.width, height = r.frame.height
        switch r.edge {
        case .top: height += dy
        case .leading: width -= 2 * dx
        case .trailing: width += 2 * dx
        case .topLeading: height += dy; width -= 2 * dx
        case .topTrailing: height += dy; width += 2 * dx
        }
        live = SlashPalette.Layout(height: Double(height), width: Double(width))
        place()
        panel?.invalidateShadow()
    }

    /// Kept for next time: the width after a side, the height after
    /// the top, both after a corner.
    func endResize() {
        if let r = resizing, let panel {
            if r.edge != .top { layout?.wrappedValue.width = Double(panel.frame.width) }
            if r.edge != .leading, r.edge != .trailing { layout?.wrappedValue.height = Double(panel.frame.height) }
        }
        live = nil
        resizing = nil
        place()
    }
}

/// The palette with grab bands on its top and sides: wide enough to find
/// (10 points, the system's own edges are a few), the resize pointer over
/// them, and the border lit in the accent colour while one is hovered or
/// dragged, so it's plain the edge will move.
struct PaletteFrame: View {
    enum Edge { case top, leading, trailing, topLeading, topTrailing }
    let content: AnyView
    let resize: SlashPaletteCoordinator
    @State private var lit: Edge?
    @State private var dragging = false

    static var band: CGFloat { zoomed(10) }
    static var radius: CGFloat { zoomed(14) }

    var body: some View {
        content
            .overlay {
                RoundedRectangle(cornerRadius: Self.radius, style: .continuous)
                    .strokeBorder(Color(nsColor: .controlAccentColor), lineWidth: 2)
                    .opacity(lit != nil || dragging ? 1 : 0)
                    .allowsHitTesting(false)
            }
            .overlay(alignment: .top) { band(.top).frame(height: Self.band).padding(.horizontal, Self.band) }
            .overlay(alignment: .leading) { band(.leading).frame(width: Self.band).padding(.top, Self.band) }
            .overlay(alignment: .trailing) { band(.trailing).frame(width: Self.band).padding(.top, Self.band) }
            .overlay(alignment: .topLeading) { band(.topLeading).frame(width: Self.band, height: Self.band) }
            .overlay(alignment: .topTrailing) { band(.topTrailing).frame(width: Self.band, height: Self.band) }
    }

    private func band(_ edge: Edge) -> some View {
        Color.clear
            .contentShape(Rectangle())
            .pointerStyle(.frameResize(position: position(edge)))
            .onHover { inside in
                if inside { lit = edge } else if lit == edge { lit = nil }
            }
            .gesture(
                DragGesture(minimumDistance: 1, coordinateSpace: .global)
                    .onChanged { _ in
                        if !dragging {
                            dragging = true
                            resize.beginResize(edge)
                        }
                        resize.resizeToPointer()
                    }
                    .onEnded { _ in
                        dragging = false
                        resize.endResize()
                    }
            )
            .tip("Drag to resize the palette; right-click for its default size")
    }

    private func position(_ edge: Edge) -> FrameResizePosition {
        switch edge {
        case .top: .top
        case .leading: .leading
        case .trailing: .trailing
        case .topLeading: .topLeading
        case .topTrailing: .topTrailing
        }
    }
}
