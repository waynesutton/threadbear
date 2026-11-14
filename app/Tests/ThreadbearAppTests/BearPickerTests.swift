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

    /// Shift-Return's new line and Shift-Tab's mode switch stay the composer's.
    func testKeysWithAModifierAreTheComposers() {
        XCTAssertTrue(BearPicker.takesKey(36, modifiers: []))   // Return picks
        XCTAssertTrue(BearPicker.takesKey(48, modifiers: []))   // Tab picks
        XCTAssertTrue(BearPicker.takesKey(53, modifiers: []))   // Esc closes
        XCTAssertFalse(BearPicker.takesKey(36, modifiers: .shift))
        XCTAssertFalse(BearPicker.takesKey(48, modifiers: .shift))
        XCTAssertFalse(BearPicker.takesKey(36, modifiers: .option))
        XCTAssertFalse(BearPicker.takesKey(125, modifiers: .command))
        XCTAssertFalse(BearPicker.takesKey(0, modifiers: []))   // a letter types on
    }

    /// Another runner's thread hands a mention to its own bears, so this
    /// Mac's list would offer bears that never get the message.
    func testNoListWhenTheMessageGoesToAnotherRunner() {
        let gate = BearPicker.Gate()
        XCTAssertTrue(gate.mayOffer(tokenAt: 4, focused: true, bearsHere: true))
        XCTAssertFalse(gate.mayOffer(tokenAt: 4, focused: true, bearsHere: false))
    }

    /// A refresh landing after Esc, or after the box loses the cursor,
    /// doesn't bring the list back.
    func testEscAndLosingTheCursorKeepItAway() {
        var gate = BearPicker.Gate()
        gate.dismissedAt = 4
        gate.moved(toTokenAt: 4)
        XCTAssertFalse(gate.mayOffer(tokenAt: 4, focused: true, bearsHere: true), "Esc holds on the same @word")
        XCTAssertFalse(BearPicker.Gate().mayOffer(tokenAt: 4, focused: false, bearsHere: true), "not without the cursor")
        gate.moved(toTokenAt: nil)
        XCTAssertTrue(gate.mayOffer(tokenAt: 4, focused: true, bearsHere: true), "a new @ word offers again")
        var other = BearPicker.Gate(dismissedAt: 4)
        other.moved(toTokenAt: 20)
        XCTAssertTrue(other.mayOffer(tokenAt: 20, focused: true, bearsHere: true))
    }
}
