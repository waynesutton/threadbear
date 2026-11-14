import AppKit
import XCTest
@testable import Threadbear
@testable import ThreadbearCore

/// Shift-Tab's Plan first (PlanFirstToggle.swift): turned off, the composer
/// goes back to the access it had, and never to more than it may have had.
@MainActor
final class PlanFirstToggleTests: XCTestCase {
    private var saved: Any?

    override func setUp() {
        _ = NSApplication.shared
        saved = UserDefaults.standard.object(forKey: PlanFirstToggle.enabledKey)
        UserDefaults.standard.set(true, forKey: PlanFirstToggle.enabledKey)
    }

    override func tearDown() {
        if let saved { UserDefaults.standard.set(saved, forKey: PlanFirstToggle.enabledKey) }
        else { UserDefaults.standard.removeObject(forKey: PlanFirstToggle.enabledKey) }
    }

    func testShiftTabPutsBackTheModeItSteppedFrom() {
        let app = AppModel()
        app.newThreadPermission = .ask
        app.togglePlanFirst("new")
        XCTAssertEqual(app.newThreadPermission, .plan)
        app.togglePlanFirst("new")
        XCTAssertEqual(app.newThreadPermission, .ask)
    }

    func testPlanFirstFromTheMenuPutsBackTheModeItHad() {
        let app = AppModel()
        app.newThreadPermission = .auto
        app.setPermission(for: "new", .plan)
        app.togglePlanFirst("new")
        XCTAssertEqual(app.newThreadPermission, .auto)
    }

    func testPlanFirstWithNothingToPutBackGoesToAskBeforeEditsNotFull() {
        // As after a restart: Plan first, and no note of what came before.
        let app = AppModel()
        app.newThreadPermission = .plan
        app.togglePlanFirst("new")
        XCTAssertEqual(app.newThreadPermission, .ask)
    }
}
