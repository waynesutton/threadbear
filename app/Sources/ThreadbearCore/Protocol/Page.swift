import Foundation

/// A Page (docs/protocol.md, "Pages"): a named, versioned page an agent
/// made, kept by the runner, in zero or more projects. The pages
/// experiment; with it off the runner sends none.
public struct Page: Sendable, Hashable, Decodable, Identifiable {
    public struct ProjectRef: Sendable, Hashable, Decodable {
        public var id: String
        public var pinned: Bool

        enum CodingKeys: String, CodingKey { case id, pinned }

        public init(from decoder: Decoder) throws {
            let c = try decoder.container(keyedBy: CodingKeys.self)
            id = try c.decode(String.self, forKey: .id)
            pinned = (try? c.decodeIfPresent(Bool.self, forKey: .pinned)) ?? false
        }
    }

    public var id: String
    public var title: String
    public var summary: String
    public var ownerThread: String
    public var projects: [ProjectRef]
    public var version: Int
    public var bytes: Int
    /// Scripts allowed: they run only after Run them.
    public var live: Bool
    public var source: String?
    public var updatedAt: Date

    enum CodingKeys: String, CodingKey { case id, title, summary, ownerThread, projects, version, bytes, live, source, updatedAt }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        id = try c.decode(String.self, forKey: .id)
        title = try c.decode(String.self, forKey: .title)
        summary = (try? c.decodeIfPresent(String.self, forKey: .summary)) ?? ""
        ownerThread = (try? c.decodeIfPresent(String.self, forKey: .ownerThread)) ?? ""
        projects = c.lossyArray(ProjectRef.self, forKey: .projects)
        version = (try? c.decodeIfPresent(Int.self, forKey: .version)) ?? 1
        bytes = (try? c.decodeIfPresent(Int.self, forKey: .bytes)) ?? 0
        live = (try? c.decodeIfPresent(Bool.self, forKey: .live)) ?? false
        source = try? c.decodeIfPresent(String.self, forKey: .source)
        updatedAt = ((try? c.decodeIfPresent(String.self, forKey: .updatedAt)) ?? nil).flatMap(ISO8601.parse) ?? .distantPast
    }

    public func isIn(project id: String) -> Bool { projects.contains { $0.id == id } }

    /// The experiment's id, as in hello's `experiments`.
    public static let experiment = "pages"
}

public struct PagesResult: Sendable, Decodable {
    public var pages: [Page]

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        pages = c.lossyArray(Page.self, forKey: .pages)
    }

    enum CodingKeys: String, CodingKey { case pages }
}

/// `pages.get`: the page, and its newest `index.html` on the runner's Mac.
public struct PageGetResult: Sendable, Decodable {
    public var page: Page
    public var file: String
    public var versions: [Int]
}
