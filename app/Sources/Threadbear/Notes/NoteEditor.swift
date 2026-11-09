import AppKit
import SwiftUI
import ThreadbearCore

/// The notes editor: a code editor's plain text box, as in Sublime Text,
/// with line numbers down the left in a monospaced face, and basic
/// Markdown coloured as you type (NoteHighlight): headings, bold, italics,
/// code, list marks, links and quotes, with the marks themselves dimmed.
/// Headings get bigger by level, `- [ ]` boxes tick with a click (the text
/// stays plain Markdown), and Return carries a list on to its next item.
struct NoteEditor: NSViewRepresentable {
    let noteId: String
    let text: String
    let onChange: (String) -> Void

    func makeCoordinator() -> Coordinator { Coordinator(onChange: onChange) }

    func makeNSView(context: Context) -> NSScrollView {
        let scroll = NSTextView.scrollableTextView()
        // Nothing of the gutter's drawn outside the editor (views don't clip by default).
        scroll.clipsToBounds = true
        scroll.drawsBackground = false
        scroll.borderType = .noBorder
        scroll.hasVerticalScroller = true
        guard let view = scroll.documentView as? NSTextView else { return scroll }
        view.delegate = context.coordinator
        view.isRichText = false
        view.allowsUndo = true
        view.isAutomaticQuoteSubstitutionEnabled = false
        view.isAutomaticDashSubstitutionEnabled = false
        view.isAutomaticTextReplacementEnabled = false
        view.isAutomaticSpellingCorrectionEnabled = false
        view.smartInsertDeleteEnabled = false
        view.font = NoteStyle.font
        view.textColor = .labelColor
        view.drawsBackground = false
        view.insertionPointColor = .controlAccentColor
        view.textContainerInset = NSSize(width: 8, height: 12)
        view.typingAttributes = NoteStyle.base
        // Boxes are links so a click reaches the delegate; NoteStyle draws them, not the link look.
        view.linkTextAttributes = [.cursor: NSCursor.pointingHand]
        view.setAccessibilityLabel("Note")
        let ruler = LineNumberRuler(textView: view)
        scroll.verticalRulerView = ruler
        scroll.hasVerticalRuler = true
        scroll.rulersVisible = true
        context.coordinator.view = view
        context.coordinator.load(text, noteId: noteId)
        DispatchQueue.main.async { view.window?.makeFirstResponder(view) }
        return scroll
    }

    func updateNSView(_ scroll: NSScrollView, context: Context) {
        context.coordinator.onChange = onChange
        // Another note picked: its text, from the top.
        if context.coordinator.noteId != noteId { context.coordinator.load(text, noteId: noteId) }
    }

    @MainActor final class Coordinator: NSObject, NSTextViewDelegate {
        var onChange: (String) -> Void
        weak var view: NSTextView?
        var noteId: String?

        init(onChange: @escaping (String) -> Void) { self.onChange = onChange }

        func load(_ text: String, noteId: String) {
            self.noteId = noteId
            guard let view else { return }
            // Before its text changes, so talking into the last note stops there.
            NoteDictation.shared.attach(view, noteId: noteId)
            view.string = text
            NoteStyle.apply(to: view)
            view.setSelectedRange(NSRange(location: 0, length: 0))
            view.scrollToBeginningOfDocument(nil)
            view.enclosingScrollView?.verticalRulerView?.needsDisplay = true
        }

        func textView(_ textView: NSTextView, clickedOnLink link: Any, at charIndex: Int) -> Bool {
            guard let url = link as? URL, url.scheme == NoteStyle.boxScheme,
                  let location = Int(url.host() ?? url.path()),
                  let edit = NoteEditing.toggledBox(in: textView.string, at: location) else { return false }
            // Through the text view's own edit calls, so ⌘Z unticks it and the note saves.
            if textView.shouldChangeText(in: edit.range, replacementString: edit.replacement) {
                textView.textStorage?.replaceCharacters(in: edit.range, with: edit.replacement)
                textView.didChangeText()
            }
            return true
        }

        func textView(_ textView: NSTextView, doCommandBy selector: Selector) -> Bool {
            guard selector == #selector(NSResponder.insertNewline(_:)) else { return false }
            let caret = textView.selectedRange()
            guard caret.length == 0 else { return false }
            let text = textView.string as NSString
            var start = 0, end = 0
            text.getLineStart(&start, end: nil, contentsEnd: &end, for: caret)
            let before = text.substring(with: NSRange(location: start, length: caret.location - start))
            let after = text.substring(with: NSRange(location: caret.location, length: end - caret.location))
            switch NoteEditing.newline(before: before, after: after) {
            case .plain: return false
            case .continueList(let prefix):
                textView.insertText("\n" + prefix, replacementRange: caret)
            case .endList:
                textView.insertText("", replacementRange: NSRange(location: start, length: caret.location - start))
            }
            return true
        }

        func textDidChange(_ notification: Notification) {
            guard let view else { return }
            NoteStyle.apply(to: view)
            view.enclosingScrollView?.verticalRulerView?.needsDisplay = true
            onChange(view.string)
        }
    }
}

enum NoteStyle {
    static var font: NSFont { .monospacedSystemFont(ofSize: TypeScale.scaled(13), weight: .regular) }

    static let boxScheme = "threadbear-note-box"

    /// # is the biggest, then ##, ###; #### and smaller are just bold.
    static func headingFont(level: Int) -> NSFont {
        let scale: CGFloat = switch level {
        case 1: 1.6
        case 2: 1.35
        case 3: 1.15
        default: 1
        }
        return .monospacedSystemFont(ofSize: TypeScale.scaled(13) * scale, weight: .bold)
    }

    static var base: [NSAttributedString.Key: Any] {
        let style = NSMutableParagraphStyle()
        style.lineHeightMultiple = 1.25
        return [.font: font, .foregroundColor: NSColor.labelColor, .paragraphStyle: style]
    }

    /// The colours, on the text as it is; the words never change.
    static func apply(to view: NSTextView) {
        guard let storage = view.textStorage else { return }
        let whole = NSRange(location: 0, length: storage.length)
        let selected = view.selectedRanges
        storage.beginEditing()
        storage.setAttributes(base, range: whole)
        let mono = font
        let bold = NSFontManager.shared.convert(mono, toHaveTrait: .boldFontMask)
        let italic = NSFontManager.shared.convert(mono, toHaveTrait: .italicFontMask)
        for span in NoteHighlight.spans(in: storage.string) where NSMaxRange(span.range) <= storage.length {
            switch span.kind {
            case .heading: storage.addAttributes([.font: headingFont(level: span.level), .foregroundColor: NSColor.controlAccentColor], range: span.range)
            case .checkbox:
                let link = URL(string: "\(boxScheme)://\(span.range.location)")!
                storage.addAttributes([.font: bold, .foregroundColor: NSColor.controlAccentColor, .link: link, .cursor: NSCursor.pointingHand], range: span.range)
            case .done: storage.addAttributes([.foregroundColor: NSColor.secondaryLabelColor, .strikethroughStyle: NSUnderlineStyle.single.rawValue], range: span.range)
            case .bold: storage.addAttribute(.font, value: bold, range: span.range)
            case .italic: storage.addAttribute(.font, value: italic, range: span.range)
            case .code: storage.addAttributes([.foregroundColor: NSColor.systemPink, .backgroundColor: NSColor.quaternaryLabelColor.withAlphaComponent(0.15)], range: span.range)
            case .codeBlock: storage.addAttributes([.foregroundColor: NSColor.systemPink, .backgroundColor: NSColor.quaternaryLabelColor.withAlphaComponent(0.12)], range: span.range)
            case .listMarker: storage.addAttributes([.foregroundColor: NSColor.controlAccentColor, .font: bold], range: span.range)
            case .link: storage.addAttributes([.foregroundColor: NSColor.linkColor, .underlineStyle: NSUnderlineStyle.single.rawValue], range: span.range)
            case .quote: storage.addAttribute(.foregroundColor, value: NSColor.secondaryLabelColor, range: span.range)
            case .mark: storage.addAttribute(.foregroundColor, value: NSColor.tertiaryLabelColor, range: span.range)
            }
        }
        storage.endEditing()
        view.selectedRanges = selected
        view.typingAttributes = base
    }
}
