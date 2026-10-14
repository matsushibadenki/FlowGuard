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
    func testConfigurationOwnershipProtectsOtherProducts() {
        XCTAssertTrue(FilterConfigurationOwnership.mayModify(hasConfiguration: true, providerIdentifier: "com.flowguard.filter", enabled: true))
        XCTAssertTrue(FilterConfigurationOwnership.mayModify(hasConfiguration: false, providerIdentifier: nil, enabled: false))
        XCTAssertFalse(FilterConfigurationOwnership.mayModify(hasConfiguration: false, providerIdentifier: nil, enabled: true))
        for identifier in [nil, "com.other.filter"] {
            XCTAssertFalse(FilterConfigurationOwnership.mayModify(hasConfiguration: true, providerIdentifier: identifier, enabled: false))
        }
    }
    func testInitialPolicyValidatesBeforeProviderStartup() throws {
        let valid = try rules()
        let data = try JSONEncoder().encode(InitialPolicy(rules: valid))
        let initial = try InitialPolicy.decode(data)
        let decision = Policy(rules: initial.rules).decision(for: FlowIdentity(executable: "/Applications/Test.app/Contents/MacOS/Test", destination: "1.2.3.4", port: 443, direction: .outbound))
        XCTAssertEqual(decision, .block)
        XCTAssertTrue(try InitialPolicy.decode(Data("{\"rules\":[]}".utf8)).rules.isEmpty)
    }
    func testInvalidInitialPolicyNeverStartsWithAnEmptyFallback() throws {
        for invalid in [try rules("ask"), try rules(source: "demo"), try rules() + rules()] {
            XCTAssertThrowsError(try InitialPolicy.decode(JSONEncoder().encode(InitialPolicy(rules: invalid))))
        }
        XCTAssertThrowsError(try InitialPolicy.decode(Data("{}".utf8)))
        XCTAssertThrowsError(try InitialPolicy.decode(Data(repeating: 0, count: 4 * 1024 * 1024 + 1)))
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
    func testPersistenceVerificationRejectsStaleDisabledAndForeignConfiguration() throws {
        let current = try JSONEncoder().encode(InitialPolicy(rules: rules()))
        let old = Data("{\"rules\":[]}".utf8)
        XCTAssertTrue(InitialPolicy.persisted(enabled: true, provider: "com.flowguard.filter", stored: current, requested: current))
        XCTAssertFalse(InitialPolicy.persisted(enabled: true, provider: "com.flowguard.filter", stored: old, requested: current))
        XCTAssertFalse(InitialPolicy.persisted(enabled: false, provider: "com.flowguard.filter", stored: current, requested: current))
        XCTAssertFalse(InitialPolicy.persisted(enabled: true, provider: "other.product", stored: current, requested: current))
        XCTAssertFalse(InitialPolicy.persisted(enabled: true, provider: "com.flowguard.filter", stored: nil, requested: current))
        XCTAssertFalse(InitialPolicy.persisted(enabled: true, provider: "com.flowguard.filter", stored: Data(), requested: Data()))
    }

    func testDigestTracksExactPolicyAndLostAcknowledgement() throws {
        XCTAssertEqual(BridgeContract.digest([]), "6bd8902048330975e577e9d4605d264456a0bcacc8eda9edc591be2383500783")
        XCTAssertEqual(BridgeContract.digest(try rules()), "89a3fffc4b5a9dd520965a06291d106ee1f6d878a7bc3a778d992d79a9306afb")
        let receiver = PolicyReceiver(); receiver.seed(try rules()); receiver.setReady(true)
        XCTAssertEqual(receiver.status().policyDigest, BridgeContract.digest(try rules()))
        let before = receiver.status().policyDigest
        let accepted = receiver.replace(try payload(receiver, rules: [])) { _ in }
        XCTAssertNotEqual(accepted.policyDigest, before)
        XCTAssertEqual(receiver.status().policyDigest, accepted.policyDigest)
        receiver.setReady(false); XCTAssertNil(receiver.status().policyDigest)
    }

}
