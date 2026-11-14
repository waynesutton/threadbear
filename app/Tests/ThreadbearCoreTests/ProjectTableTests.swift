import XCTest
@testable import ThreadbearCore

/// A table of bears on a project (protocol "Tables"): read when it's there,
/// nil when it isn't, and never a failed decode.
final class ProjectTableTests: XCTestCase {
    private func project(_ extra: String) throws -> Project {
        try JSONDecoder().decode(Project.self, from: Data(#"{"id":"p1","title":"Head table","folders":[],"threadCount":0\#(extra)}"#.utf8))
    }

    func testATablesBearsAreRead() throws {
        let p = try project(#","table":{"bears":["head-bear-1","writer-2"],"maxTurns":4}"#)
        XCTAssertEqual(p.table?.bears, ["head-bear-1", "writer-2"])
        XCTAssertEqual(p.table?.maxTurns, 4)
    }

    func testAnOrdinaryProjectHasNoTable() throws {
        XCTAssertNil(try project("").table)
    }

    func testABadTableIsLeftOutNotFatal() throws {
        let p = try project(#","table":"nonsense""#)
        XCTAssertNil(p.table)
        XCTAssertEqual(p.title, "Head table")
    }
}
