import XCTest
import ThreadbearCore
@testable import Threadbear

/// The composer's @ list (BearPicker): which bears it offers for what's typed.
@MainActor
final class BearPickerTests: XCTestCase {
    private let bears = [
        BearEntry(id: "writer-1", name: "Writer"),
        BearEntry(id: "head-bear-1", name: "Head Bear", head: true),
        BearEntry(id: "researcher-1", name: "Researcher"),
    ]

    func testAnEmptyQueryListsEveryBearWithTheHeadBearFirst() {
        XCTAssertEqual(BearPicker.matches("", in: bears).map(\.name), ["Head Bear", "Researcher", "Writer"])
    }

    func testNamesStartingWithItComeBeforeAWordStartingWithIt() {
        XCTAssertEqual(BearPicker.matches("b", in: bears).map(\.name), ["Head Bear"])
        XCTAssertEqual(BearPicker.matches("re", in: bears).map(\.name), ["Researcher"])
        XCTAssertEqual(BearPicker.matches("WR", in: bears).map(\.name), ["Writer"])
        XCTAssertEqual(BearPicker.matches("head b", in: bears).map(\.name), ["Head Bear"])
        XCTAssertEqual(BearPicker.matches("zzz", in: bears), [])
    }
}
