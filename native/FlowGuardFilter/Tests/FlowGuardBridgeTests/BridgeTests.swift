import XCTest
@testable import FlowGuardBridge
import FlowGuardPolicy
final class BridgeTests: XCTestCase {
    func rules(_ action: String = "block", source: String = "live") throws -> [PolicyRule] {
        let data = Data("""
        [{"id":"one","appKey":"/Applications/Test.app/Contents/MacOS/Test","app":"Test","destination":"1.2.3.4","port":443,"direction":"both","action":"\(action)","updatedAt":1,"source":"\(source)"}]
        """.utf8)
        return try JSONDecoder().decode([PolicyRule].self, from: data)
    }
    func payload(_ receiver: PolicyReceiver, rules: [PolicyRule], revision: UInt64? = nil, instance: String? = nil) throws -> Data {
        let status = receiver.status()
        return try JSONEncoder().encode(PolicyUpdate(requestID: UUID().uuidString, expectedInstanceID: instance ?? status.instanceID, expectedRevision: revision ?? status.revision, rules: rules))
    }
    func testReadyRequiredAndAcceptedRevision() throws {
        let receiver = PolicyReceiver(); var applied = 0
        XCTAssertEqual(receiver.replace(try payload(receiver, rules: rules())) { _ in applied += 1 }.error, "provider_unavailable")
        receiver.setReady(true)
        let reply = receiver.replace(try payload(receiver, rules: rules())) { _ in applied += 1 }
        XCTAssertEqual(reply.status, "accepted"); XCTAssertEqual(reply.revision, 1); XCTAssertEqual(applied, 1)
    }
    func testStaleRevisionAndRestartCannotOverwritePolicy() throws {
        let receiver = PolicyReceiver(); receiver.setReady(true)
        let old = try payload(receiver, rules: rules())
        XCTAssertNil(receiver.replace(old) { _ in }.error)
        XCTAssertEqual(receiver.replace(old) { _ in XCTFail() }.error, "revision_conflict")
        let restarted = PolicyReceiver(); restarted.setReady(true)
        XCTAssertEqual(restarted.replace(old) { _ in XCTFail() }.error, "revision_conflict")
    }
    func testFailureRollsBackAndInvalidRulesNeverReachProvider() throws {
        let receiver = PolicyReceiver(); receiver.setReady(true)
        XCTAssertEqual(receiver.replace(try payload(receiver, rules: rules())) { _ in throw NSError(domain: "test", code: 1) }.error, "apply_failed")
        XCTAssertEqual(receiver.status().revision, 0); XCTAssertEqual(receiver.status().ruleCount, 0)
        for invalid in [try rules("ask"), try rules(source: "demo"), try rules() + rules()] {
            XCTAssertEqual(receiver.replace(try payload(receiver, rules: invalid)) { _ in XCTFail() }.error, "unsupported_or_invalid_rules")
        }
        XCTAssertEqual(receiver.replace(Data(repeating: 0, count: 4 * 1024 * 1024 + 1)) { _ in XCTFail() }.error, "invalid_payload")
    }
    func testAcknowledgementMustMatchRequestInstanceAndRevision() throws {
        let receiver = PolicyReceiver(); receiver.setReady(true)
        let data = try payload(receiver, rules: rules())
        let update = try JSONDecoder().decode(PolicyUpdate.self, from: data)
        let reply = receiver.replace(data) { _ in }
        XCTAssertTrue(BridgeContract.validAcknowledgement(reply, for: update))
        let other = PolicyUpdate(requestID: UUID().uuidString, expectedInstanceID: update.expectedInstanceID, expectedRevision: 0, rules: update.rules)
        XCTAssertFalse(BridgeContract.validAcknowledgement(reply, for: other))
        let newInstance = PolicyUpdate(requestID: update.requestID, expectedInstanceID: UUID().uuidString, expectedRevision: 0, rules: update.rules)
        XCTAssertFalse(BridgeContract.validAcknowledgement(reply, for: newInstance))
    }
    func testStoppedProviderRejectsUpdates() throws {
        let receiver = PolicyReceiver(); receiver.setReady(true); receiver.setReady(false)
        XCTAssertEqual(receiver.replace(try payload(receiver, rules: rules())) { _ in XCTFail() }.error, "provider_unavailable")
    }
}
