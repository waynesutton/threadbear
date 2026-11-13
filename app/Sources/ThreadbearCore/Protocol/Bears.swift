import Foundation

/// The bear whose folder a thread is in (docs/protocol.md, "Bears"): what
/// the sidebar draws in place of the folder. A runner sends it only while
/// the bears experiment is on; nil for any other thread.
public struct ThreadBear: Sendable, Hashable, Codable {
    public var id: String
    public var name: String
    /// An SF Symbol, from the bear's bear.json `icon`.
    public var icon: String?
    /// "#rrggbb" or "#rgb", from the bear's bear.json `color`.
    public var color: String?
    /// The head bear, which runs the others.
    public var head: Bool?

    public init(id: String, name: String, icon: String? = nil, color: String? = nil, head: Bool? = nil) {
        self.id = id
        self.name = name
        self.icon = icon
        self.color = color
        self.head = head
    }

    /// Red, green and blue from 0 to 1, or nil for a colour that isn't
    /// "#rgb" or "#rrggbb".
    public var rgb: (Double, Double, Double)? {
        guard var hex = color, hex.hasPrefix("#") else { return nil }
        hex.removeFirst()
        if hex.count == 3 { hex = hex.map { "\($0)\($0)" }.joined() }
        guard hex.count == 6, let value = UInt32(hex, radix: 16) else { return nil }
        return (Double((value >> 16) & 0xFF) / 255, Double((value >> 8) & 0xFF) / 255, Double(value & 0xFF) / 255)
    }
}

/// What bears.update answers; the app only needs it to have worked.
private struct BearUpdated: Decodable, Sendable {}

public extension RPCClient {
    /// Sets a bear's colour (bears.update); nil goes back to its default.
    func setBearColor(_ bear: String, color: String?) async throws {
        _ = try await call("bears.update", .object(["bear": .string(bear), "color": color.map { .string($0) } ?? .null]), as: BearUpdated.self)
    }
}

/// A bear as bears.list gives it, for the composer's @ list.
public struct BearEntry: Sendable, Hashable, Decodable {
    public var id: String
    public var name: String
    public var does: String?
    public var icon: String?
    public var color: String?
    public var head: Bool?
    public var archived: Bool?

    public init(id: String, name: String, does: String? = nil, icon: String? = nil, color: String? = nil, head: Bool? = nil) {
        self.id = id
        self.name = name
        self.does = does
        self.icon = icon
        self.color = color
        self.head = head
    }

    /// The same bear as a thread's tile draws it.
    public var tile: ThreadBear { ThreadBear(id: id, name: name, icon: icon, color: color, head: head) }
}

private struct BearsList: Decodable, Sendable {
    var on: Bool
    var bears: [BearEntry]?
}

public extension RPCClient {
    /// The bears (bears.list, no faces): none while the experiment is off.
    func listBears() async throws -> [BearEntry] {
        let r = try await call("bears.list", .object(["faces": .bool(false)]), as: BearsList.self)
        return r.on ? (r.bears ?? []).filter { $0.archived != true } : []
    }
}
