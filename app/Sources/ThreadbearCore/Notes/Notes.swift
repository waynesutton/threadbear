import CryptoKit
import Foundation

/// One of Mike's notes (the bar's Notes page). Kept on this Mac only,
/// each sealed with AES-GCM under a key in the Keychain (NotesStore), so a
/// note never goes to the runner, an agent or the backend, and what's on
/// disk can't be read without the Mac's login.
public struct Note: Sendable, Hashable, Codable, Identifiable {
    public var id: String
    public var title: String
    public var body: String
    public var createdAt: Date
    public var updatedAt: Date

    public init(id: String = UUID().uuidString, title: String, body: String = "", createdAt: Date = Date(), updatedAt: Date = Date()) {
        self.id = id
        self.title = title
        self.body = body
        self.createdAt = createdAt
        self.updatedAt = updatedAt
    }
}

public enum NoteCrypto {
    public enum Failure: Error { case badData }

    /// The note as stored: a version byte, then AES-GCM's nonce, text and tag.
    public static func seal(_ note: Note, key: SymmetricKey) throws -> Data {
        // Dates as exact seconds (ISO 8601 would drop the fractions).
        let box = try AES.GCM.seal(try JSONEncoder().encode(note), using: key)
        guard let combined = box.combined else { throw Failure.badData }
        return Data([1]) + combined
    }

    public static func open(_ data: Data, key: SymmetricKey) throws -> Note {
        guard data.first == 1 else { throw Failure.badData }
        let box = try AES.GCM.SealedBox(combined: data.dropFirst())
        return try JSONDecoder().decode(Note.self, from: try AES.GCM.open(box, using: key))
    }
}

public enum NoteNames {
    public static let base = "Untitled note"

    /// "Untitled note", or the first "Untitled note N" not taken.
    public static func next(after titles: [String]) -> String {
        let taken = Set(titles.map { $0.trimmingCharacters(in: .whitespaces).lowercased() })
        if !taken.contains(base.lowercased()) { return base }
        var n = 2
        while taken.contains("\(base) \(n)".lowercased()) { n += 1 }
        return "\(base) \(n)"
    }

    /// A title as kept: trimmed, one line, never empty.
    public static func clean(_ title: String, fallback: String) -> String {
        let one = title.replacingOccurrences(of: "\n", with: " ").trimmingCharacters(in: .whitespaces)
        return one.isEmpty ? fallback : String(one.prefix(200))
    }
}

/// What the notes editor colours: basic Markdown, line by line, as an
/// editor shows it (the marks stay, dimmed; the words get the style).
/// Headings carry their level (1 for #), so the editor can size them.
public enum NoteHighlight {
    public enum Kind: Sendable, Equatable {
        case heading, bold, italic, code, codeBlock, listMarker, link, quote, mark
        /// A task's box, `[ ]` or `[x]`, that a click ticks.
        case checkbox
        /// The words of a ticked task.
        case done
    }

    public struct Span: Sendable, Equatable {
        public var range: NSRange
        public var kind: Kind
        public var level: Int
        public init(_ range: NSRange, _ kind: Kind, level: Int = 0) {
            self.range = range
            self.kind = kind
            self.level = level
        }
    }

    private static let rules: [(NSRegularExpression, Kind, Int)] = [
        (try! NSRegularExpression(pattern: #"\*\*[^*\n]+\*\*|__[^_\n]+__"#), .bold, 0),
        (try! NSRegularExpression(pattern: #"(?<![*\w])\*[^*\s][^*\n]*\*(?!\*)|(?<![_\w])_[^_\s][^_\n]*_(?!\w)"#), .italic, 0),
        (try! NSRegularExpression(pattern: #"`[^`\n]+`"#), .code, 0),
        (try! NSRegularExpression(pattern: #"\[[^\]\n]+\]\([^)\s]+\)|https?://[^\s)]+"#), .link, 0),
    ]
    private static let headingRule = try! NSRegularExpression(pattern: #"^(#{1,6})\s"#)
    private static let quoteRule = try! NSRegularExpression(pattern: #"^\s*>\s?"#)
    /// A list line: its bullet or number, then maybe a task box.
    static let listRule = try! NSRegularExpression(pattern: #"^\s*([-*+]|\d{1,9}[.)])\s(\[([ xX])\](?:\s|$))?"#)

    public static func spans(in text: String) -> [Span] {
        var out: [Span] = []
        var inFence = false
        var location = 0
        for line in text.components(separatedBy: "\n") {
            let length = (line as NSString).length
            let range = NSRange(location: location, length: length)
            let whole = NSRange(location: 0, length: length)
            let trimmed = line.trimmingCharacters(in: .whitespaces)
            if trimmed.hasPrefix("```") || trimmed.hasPrefix("~~~") {
                out.append(Span(range, .codeBlock))
                inFence.toggle()
            } else if inFence {
                out.append(Span(range, .codeBlock))
            } else if let m = headingRule.firstMatch(in: line, range: whole) {
                out.append(Span(range, .heading, level: m.range(at: 1).length))
                out.append(Span(NSRange(location: location, length: m.range(at: 1).length), .mark))
            } else if let m = quoteRule.firstMatch(in: line, range: whole) {
                out.append(Span(range, .quote))
                out.append(Span(NSRange(location: location + m.range.location, length: m.range.length), .mark))
            } else {
                if let m = listRule.firstMatch(in: line, range: whole) {
                    let bullet = m.range(at: 1)
                    out.append(Span(NSRange(location: location + bullet.location, length: bullet.length), .listMarker))
                    let box = m.range(at: 2)
                    if box.location != NSNotFound {
                        out.append(Span(NSRange(location: location + box.location, length: 3), .checkbox))
                        let ticked = (line as NSString).substring(with: m.range(at: 3)) != " "
                        let rest = NSMaxRange(m.range)
                        if ticked, rest < length {
                            out.append(Span(NSRange(location: location + rest, length: length - rest), .done))
                        }
                    }
                }
                for (regex, kind, _) in rules {
                    for m in regex.matches(in: line, range: whole) {
                        out.append(Span(NSRange(location: location + m.range.location, length: m.range.length), kind))
                    }
                }
            }
            location += length + 1
        }
        return out
    }
}

/// The notes editor's typing help, kept apart from AppKit so it's tested.
public enum NoteEditing {
    /// Ticks or unticks the box whose `[` is at `location`: the one
    /// character to put inside it, or nil if there's no box there.
    public static func toggledBox(in text: String, at location: Int) -> (range: NSRange, replacement: String)? {
        let ns = text as NSString
        guard location >= 0, location + 3 <= ns.length,
              ns.substring(with: NSRange(location: location, length: 1)) == "[",
              ns.substring(with: NSRange(location: location + 2, length: 1)) == "]" else { return nil }
        let inside = NSRange(location: location + 1, length: 1)
        switch ns.substring(with: inside) {
        case " ": return (inside, "x")
        case "x", "X": return (inside, " ")
        default: return nil
        }
    }

    public enum Newline: Equatable {
        /// An ordinary new line.
        case plain
        /// A new line that starts with this, the next item of the list.
        case continueList(String)
        /// Return on an empty item: the list ends, and its marker goes.
        case endList
    }

    /// What Return does with the caret at the end of `before` (the line's
    /// text up to the caret), and `after` (the rest of that line).
    public static func newline(before: String, after: String) -> Newline {
        let ns = before as NSString
        let regex = try! NSRegularExpression(pattern: #"^(\s*)([-*+]|(\d{1,9})([.)]))\s(\[[ xX]\]\s?)?(.*)$"#)
        guard let m = regex.firstMatch(in: before, range: NSRange(location: 0, length: ns.length)) else { return .plain }
        let content = ns.substring(with: m.range(at: 6))
        if content.trimmingCharacters(in: .whitespaces).isEmpty {
            return after.trimmingCharacters(in: .whitespaces).isEmpty ? .endList : .plain
        }
        let indent = ns.substring(with: m.range(at: 1))
        var bullet = ns.substring(with: m.range(at: 2))
        if m.range(at: 3).location != NSNotFound, let n = Int(ns.substring(with: m.range(at: 3))) {
            bullet = "\(n + 1)" + ns.substring(with: m.range(at: 4))
        }
        let box = m.range(at: 5).location != NSNotFound ? "[ ] " : ""
        return .continueList(indent + bullet + " " + box)
    }
}
