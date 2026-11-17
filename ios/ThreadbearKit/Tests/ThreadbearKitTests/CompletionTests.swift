import Foundation
import Testing
@testable import ThreadbearKit

struct CompletionTests {
    @Test func partialReportsAndFutureStatusesNeverClaimAllCompleted() throws {
        let json = #"{"tasks":[{"id":"a","title":"Build","status":"completed"},{"id":"b","title":"Try","status":"blocked"},{"id":"c","title":"Later","status":"future"}]}"#
        let ledger = try JSONDecoder().decode(PhoneCompletion.self, from: Data(json.utf8))
        #expect(ledger.heading == "1 of 3 tasks completed")
        #expect(ledger.tasks[0].isCompleted)
        #expect(!ledger.tasks[2].isCompleted)
    }

    @Test func phoneRowsDecodeWithAndWithoutTheOptionalLedger() throws {
        let json = #"{"threadId":"t","runnerId":"mac","title":"Thread","cwd":"/","accountId":"a","provider":"claude","status":"idle","needsYou":false,"unread":false,"preview":"","lastActivityAt":"2026-11-16T00:00:00Z","lastActivityMs":1,"createdAt":"2026-11-16T00:00:00Z","view":"inbox"}"#
        #expect(try JSONDecoder().decode(PhoneThread.self, from: Data(json.utf8)).completion == nil)
        let withLedger = String(json.dropLast()) + #","completion":{"tasks":[{"id":"a","title":"Build","status":"completed"}]}}"#
        #expect(try JSONDecoder().decode(PhoneThread.self, from: Data(withLedger.utf8)).completion?.heading == "1 task completed")
    }

    @Test func completeAndEmptyLedgersHaveDistinctHeadings() throws {
        let complete = #"{"tasks":[{"id":"a","title":"Build","status":"completed"}]}"#
        #expect(try JSONDecoder().decode(PhoneCompletion.self, from: Data(complete.utf8)).heading == "1 task completed")
        #expect(try JSONDecoder().decode(PhoneCompletion.self, from: Data(#"{"tasks":[]}"#.utf8)).heading == nil)
    }
}
