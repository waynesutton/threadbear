import XCTest
@testable import ThreadbearCore

final class PageTests: XCTestCase {
    func testAPageDecodesAsTheRunnerSendsIt() throws {
        let json = """
        {"pages":[{"id":"pg_bundle_size","title":"Bundle size","summary":"20.22 MB total","ownerThread":"th1",
        "projects":[{"id":"p_tb","by":"agent","pinned":true},{"id":"p_x","by":"mike"}],"version":3,"bytes":90123,"live":false,
        "source":"bundle-analyzer summary","createdAt":"2026-11-10T09:00:00.000Z","updatedAt":"2026-11-10T09:30:00.000Z"},
        {"id":"broken"}]}
        """
        let pages = try JSONDecoder().decode(PagesResult.self, from: Data(json.utf8)).pages
        XCTAssertEqual(pages.count, 1, "one without a title is dropped, not the whole list")
        let page = try XCTUnwrap(pages.first)
        XCTAssertEqual(page.version, 3)
        XCTAssertEqual(page.source, "bundle-analyzer summary")
        XCTAssertTrue(page.isIn(project: "p_tb"))
        XCTAssertTrue(page.isIn(project: "p_x"))
        XCTAssertFalse(page.isIn(project: "p_other"))
        XCTAssertEqual(page.projects.map(\.pinned), [true, false])
        XCTAssertEqual(page.updatedAt, ISO8601.parse("2026-11-10T09:30:00.000Z"))
    }

    func testAPageInNoProjectIsInNone() throws {
        let json = #"{"id":"pg_a","title":"A","summary":"","ownerThread":"t","projects":[],"version":1,"bytes":1,"live":true,"updatedAt":"2026-11-10T09:30:00Z"}"#
        let page = try JSONDecoder().decode(Page.self, from: Data(json.utf8))
        XCTAssertTrue(page.projects.isEmpty)
        XCTAssertTrue(page.live)
    }

    func testKindsRecipeEditsAndDeletionDecode() throws {
        let json = #"{"id":"pg_d","title":"Plan","summary":"","ownerThread":"t","projects":[],"version":12,"bytes":1,"live":false,"kind":"doc","recipe":"write it","editedBy":"user","deletedAt":"2026-11-10T09:30:00Z","updatedAt":"2026-11-10T09:30:00Z"}"#
        let page = try JSONDecoder().decode(Page.self, from: Data(json.utf8))
        XCTAssertTrue(page.isDoc)
        XCTAssertEqual(page.symbol, "doc.plaintext")
        XCTAssertEqual(page.recipe, "write it")
        XCTAssertTrue(page.editedByUser)
        XCTAssertNotNil(page.deletedAt)
        XCTAssertEqual(page.keptVersions, [12, 11, 10, 9, 8, 7, 6, 5, 4, 3], "the newest ten, newest first")
        let old = try JSONDecoder().decode(Page.self, from: Data(#"{"id":"pg_o","title":"Old","version":2}"#.utf8))
        XCTAssertEqual(old.kind, "page", "a page from before kinds")
        XCTAssertEqual(old.keptVersions, [2, 1])
        XCTAssertFalse(old.editedByUser)
    }
}
