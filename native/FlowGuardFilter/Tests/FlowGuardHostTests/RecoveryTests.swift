import XCTest
import NetworkExtension
import SystemExtensions
@testable import FlowGuardHost
import FlowGuardBridge
final class RecoveryTests: XCTestCase {
    final class Preferences: FilterPreferences {
        var providerConfiguration: NEFilterProviderConfiguration? = {
            let config = NEFilterProviderConfiguration(); config.filterDataProviderBundleIdentifier = "com.flowguard.filter"; return config
        }()
        var localizedDescription: String?
        var isEnabled = true
        var pending: ((Error?) -> Void)?
        var writes: [Bool] = []
        func loadFromPreferences(completionHandler: @escaping (Error?) -> Void) { XCTAssertNil(pending); pending = completionHandler }
        func saveToPreferences(completionHandler: @escaping (Error?) -> Void) { XCTAssertNil(pending); writes.append(isEnabled); pending = completionHandler }
        func removeFromPreferences(completionHandler: @escaping (Error?) -> Void) { XCTAssertNil(pending); pending = completionHandler }
        func complete() {
            let callback = pending; pending = nil; XCTAssertNotNil(callback); callback?(nil)
            RunLoop.main.run(until: Date().addingTimeInterval(0.005))
        }
    }
    func policy() throws -> Data { Data(#"{"rules":[]}"#.utf8) }
    func testRecoveryWaitsForSubmittedEnableSaveAndFinishesWithDisable() throws {
        let prefs = Preferences(); var request: OSSystemExtensionRequest?
        let controller = ExtensionController(authorize: {true}, preferences: {prefs}, submit: {request = $0})
        var old: [String] = []; var recovery: [String] = []
        controller.activate(policy: try policy()) { status, _ in old.append(status) }
        let activation = try XCTUnwrap(request)
        controller.request(activation, didFinishWithResult: .completed)
        prefs.complete() // activation load -> submitted enable save
        XCTAssertEqual(prefs.writes, [true])
        controller.disable(remove: false) { status, _ in recovery.append(status) }
        XCTAssertTrue(recovery.isEmpty); XCTAssertTrue(old.isEmpty)
        prefs.complete() // old save returns -> cancels old operation, starts disable load
        XCTAssertEqual(old, ["cancelled"])
        prefs.complete(); XCTAssertEqual(prefs.writes, [true, false])
        prefs.complete(); prefs.complete()
        XCTAssertEqual(recovery, ["disabled"]); XCTAssertNil(prefs.pending)
        controller.request(activation, didFinishWithResult: .completed) // late duplicate delegate
        XCTAssertNil(prefs.pending); XCTAssertEqual(prefs.writes, [true, false])
        controller.persist(policy: try policy()) { status, _ in XCTAssertEqual(status,"cancelled") }
        XCTAssertNil(prefs.pending)
    }
    func testApprovalAndPersistLoadYieldWithoutStartingAnEnableWrite() throws {
        let prefs = Preferences(); var request: OSSystemExtensionRequest?
        let controller = ExtensionController(authorize: {true}, preferences: {prefs}, submit: {request = $0})
        controller.activate(policy: try policy()) { _, _ in }
        let activation = try XCTUnwrap(request)
        controller.requestNeedsUserApproval(activation)
        var recovery: String?
        controller.disable(remove: false) { status, _ in recovery = status }
        controller.request(activation, didFinishWithResult: .completed)
        prefs.complete(); prefs.complete(); prefs.complete()
        XCTAssertEqual(recovery,"disabled"); XCTAssertEqual(prefs.writes,[false])
        // Only an explicit later activation may clear the recovery latch.
        controller.activate(policy: try policy()) { _, _ in }
        controller.request(try XCTUnwrap(request), didFinishWithResult: .completed)
        prefs.complete(); prefs.complete()
        var cancelled: String?
        controller.persist(policy: try policy()) { status, _ in cancelled = status }
        controller.disable(remove: false) { _, _ in }
        prefs.complete() // old persist load returns; no persist save is issued
        XCTAssertEqual(cancelled,"cancelled")
        prefs.complete(); prefs.complete(); prefs.complete()
        XCTAssertEqual(prefs.writes,[false,true,false])
    }
}
