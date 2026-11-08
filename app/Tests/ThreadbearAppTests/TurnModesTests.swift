import AppKit
import XCTest
@testable import Threadbear
@testable import ThreadbearCore

/// Shift-Tab's steps (TurnModes.swift): out of Plan first, the composer
/// goes back to the access it had, and never to more than it may have had.
@MainActor
final class TurnModesTests: XCTestCase {
    private var saved: Any?

    override func setUp() {
        _ = NSApplication.shared
        saved = UserDefaults.standard.object(forKey: TurnModes.enabledKey)
        UserDefaults.standard.set(true, forKey: TurnModes.enabledKey)
    }

    override func tearDown() {
        if let saved { UserDefaults.standard.set(saved, forKey: TurnModes.enabledKey) }
        else { UserDefaults.standard.removeObject(forKey: TurnModes.enabledKey) }
    }

    func testShiftTabPutsBackTheModeItSteppedFrom() {
        let app = AppModel()
        app.newThreadPermission = .ask
        app.cycleTurnMode("new")
        XCTAssertEqual(app.newThreadPermission, .plan)
        app.cycleTurnMode("new")
        XCTAssertEqual(app.newThreadPermission, .ask)
        XCTAssertEqual(app.turnMode(for: "new"), .ask)
    }

    func testPlanFirstFromTheMenuPutsBackTheModeItHad() {
        let app = AppModel()
        app.newThreadPermission = .auto
        app.setPermission(for: "new", .plan)
        app.cycleTurnMode("new")
        XCTAssertEqual(app.newThreadPermission, .auto)
    }

    func testPlanFirstWithNothingToPutBackGoesToAskBeforeEditsNotFull() {
        // As after a restart: Plan first, and no note of what came before.
        let app = AppModel()
        app.newThreadPermission = .plan
        app.cycleTurnMode("new")
        XCTAssertEqual(app.newThreadPermission, .ask)
    }
}
