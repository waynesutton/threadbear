import XCTest
@testable import ThreadbearCore

final class SkillMentionsTests: XCTestCase {
    let names: Set<String> = ["pele", "anatomy"]

    private func found(_ text: String) -> [String] {
        SkillMentions.ranges(in: text, names: names).map { (text as NSString).substring(with: $0) }
    }

    func testFindsInstalledSkillsAsWords() {
        XCTAssertEqual(found("/pele draw the checkout flow"), ["/pele"])
        XCTAssertEqual(found("use /anatomy, then (/pele)"), ["/anatomy", "/pele"])
        XCTAssertEqual(found("/PELE please"), ["/PELE"])
    }

    func testLeavesPathsAndOtherWordsAlone() {
        XCTAssertEqual(found("open src/pele/render.mjs"), [])
        XCTAssertEqual(found("~/pele and ./pele and https://x.com/pele"), [])
        XCTAssertEqual(found("/peler /unknown /pele-x"), [])
        XCTAssertEqual(found("no slashes"), [])
    }

    /// Claude's name for a store skill is threadbear:name: that's the
    /// skill, and the colon never ends a shorter match (Wayne, 12 Nov).
    func testPluginNames() {
        let names: Set<String> = ["pele", "threadbear", "convex:add"]
        let found = { (text: String) in SkillMentions.ranges(in: text, names: names).map { (text as NSString).substring(with: $0) } }
        XCTAssertEqual(found("use /threadbear:pele now"), ["/threadbear:pele"])
        XCTAssertEqual(found("/threadbear:nope and /convex:add"), ["/convex:add"])
        XCTAssertEqual(SkillMentions.skill(named: "threadbear:pele", in: names), "pele")
        XCTAssertNil(SkillMentions.skill(named: "threadbear:nope", in: names))
    }

    func testLinks() {
        let url = SkillMentions.url(for: "pele")!
        XCTAssertEqual(SkillMentions.name(of: url), "pele")
        XCTAssertNil(SkillMentions.name(of: URL(string: "https://example.com")!))
    }
}
