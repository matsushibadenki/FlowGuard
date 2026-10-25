import XCTest
@testable import FlowGuardBridge
import FlowGuardPolicy
final class BridgeTests: XCTestCase {
    func rules(_ action: String = "block", source: String = "live") throws -> [PolicyRule] {
        let data = Data("""
        [{"id":"one","transport":"TCP","appKey":"signed:v1:developer:ABCDE12345:com.test.app","identity":{"authority":"developer","teamId":"ABCDE12345","identifier":"com.test.app"},"app":"Test","destination":"1.2.3.4","port":443,"direction":"both","action":"\(action)","updatedAt":1,"source":"\(source)"}]
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
        let decision = Policy(rules: initial.rules).decision(for: FlowIdentity(transport: .tcp, executable: "/Applications/Test.app/Contents/MacOS/Test", destination: "1.2.3.4", port: 443, direction: .outbound, identity: SigningIdentity(authority: "developer", teamId: "ABCDE12345", identifier: "com.test.app")))
        XCTAssertEqual(decision, .block)
        XCTAssertTrue(try InitialPolicy.decode(Data("{\"rules\":[]}".utf8)).rules.isEmpty)
    }
    func testInvalidInitialPolicyNeverStartsWithAnEmptyFallback() throws {
        for invalid in [try rules(source: "demo"), try rules() + rules()] {
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
        for invalid in [try rules(source: "demo"), try rules() + rules()] {
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
        XCTAssertEqual(BridgeContract.digest([]), "179851694d92493625d6aea51ce56532985145b32fc81f68e48fc9303e50e8fa")
        XCTAssertEqual(BridgeContract.digest(try rules()), "70bf7d9160f506aac6bac2f59c3e5eb4d0945b1958645dfdc6fd271ca299e25e")
        let receiver = PolicyReceiver(); receiver.seed(try rules()); receiver.setReady(true)
        XCTAssertEqual(receiver.status().policyDigest, BridgeContract.digest(try rules()))
        let before = receiver.status().policyDigest
        let accepted = receiver.replace(try payload(receiver, rules: [])) { _ in }
        XCTAssertNotEqual(accepted.policyDigest, before)
        XCTAssertEqual(receiver.status().policyDigest, accepted.policyDigest)
        receiver.setReady(false); XCTAssertNil(receiver.status().policyDigest)
    }

    func testLegacyPolicyAndOldProtocolAreRejected() throws {
        let legacy = Data(#"{"rules":[{"id":"old","transport":"TCP","appKey":"/old/path","app":"Test","destination":"1.2.3.4","port":443,"direction":"both","action":"allow","updatedAt":1,"source":"live"}]}"#.utf8)
        XCTAssertThrowsError(try InitialPolicy.decode(legacy))
        let receiver = PolicyReceiver(); receiver.setReady(true)
        var object = try JSONSerialization.jsonObject(with: payload(receiver, rules: rules())) as! [String: Any]
        for version in [1, 2, 3, 4] {
            object["protocolVersion"] = version
            let reply = receiver.replace(try JSONSerialization.data(withJSONObject: object)) { _ in XCTFail("old protocol must not apply") }
            XCTAssertEqual(reply.error, "invalid_protocol")
        }
    }

    func testSavingV4RemovesAllObsoletePolicySlots() throws {
        let policy = try JSONEncoder().encode(InitialPolicy(rules: rules()))
        let vendor = InitialPolicy.vendorConfiguration(previous: [InitialPolicy.legacyKey: Data("old".utf8), "flowguard.initialPolicy.v2": Data("old-v2".utf8), "flowguard.initialPolicy.v3": Data("old-v3".utf8), "other": "retained"], policy: policy)
        XCTAssertNil(vendor[InitialPolicy.legacyKey])
        XCTAssertNil(vendor["flowguard.initialPolicy.v2"])
        XCTAssertNil(vendor["flowguard.initialPolicy.v3"])
        XCTAssertEqual(vendor[InitialPolicy.key] as? Data, policy)
        XCTAssertEqual(vendor["other"] as? String, "retained")
    }

}
