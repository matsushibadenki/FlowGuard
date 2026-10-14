import XCTest
@testable import FlowGuardPolicy
final class PolicyTests: XCTestCase {
    func rule(_ source: String = "live", _ direction: String = "both", _ action: String = "block", _ updated: Int = 1) throws -> PolicyRule {
        let json = """
        {"id":"\(source)-\(direction)","appKey":"signed:v1:developer:ABCDE12345:com.test.app","identity":{"authority":"developer","teamId":"ABCDE12345","identifier":"com.test.app"},"app":"Mail","destination":"1.2.3.4","port":993,"direction":"\(direction)","action":"\(action)","updatedAt":\(updated),"source":"\(source)"}
        """
        return try JSONDecoder().decode(PolicyRule.self, from: Data(json.utf8))
    }
    let signing = SigningIdentity(authority: "developer", teamId: "ABCDE12345", identifier: "com.test.app")
    let flow = FlowIdentity(executable: "/Applications/Mail.app/Contents/MacOS/Mail", destination: "1.2.3.4", port: 993, direction: .outbound, identity: SigningIdentity(authority: "developer", teamId: "ABCDE12345", identifier: "com.test.app"))
    func testDemoRulesCannotEnforce() throws {
        XCTAssertNil(Policy(rules: [try rule("demo")]).decision(for: flow))
    }
    func testExactIdentityAndPort() throws {
        let policy = Policy(rules: [try rule()])
        XCTAssertEqual(policy.decision(for: flow), .block)
        XCTAssertNil(policy.decision(for: FlowIdentity(executable: "/other/Mail", destination: "1.2.3.4", port: 993, direction: .outbound)))
        XCTAssertNil(policy.decision(for: FlowIdentity(executable: flow.executable, destination: "1.2.3.4", port: 443, direction: .outbound, identity: signing)))
    }
    func testSpecificDirectionHasPriority() throws {
        let policy = Policy(rules: [try rule(), try rule("live", "outbound", "allow", 2)])
        XCTAssertEqual(policy.decision(for: flow), .allow)
        XCTAssertEqual(policy.decision(for: FlowIdentity(executable: flow.executable, destination: flow.destination, port: flow.port, direction: .inbound, identity: signing)), .block)
    }
    func testPublisherIdentitySurvivesMovesButNeverMatchesUnsignedOrAnotherSigner() throws {
        let policy = Policy(rules: [try rule()])
        XCTAssertEqual(policy.decision(for: FlowIdentity(executable: "/Moved/Mail", destination: flow.destination, port: flow.port, direction: .outbound, identity: signing)), .block)
        for identity in [nil, SigningIdentity(authority: "developer", teamId: "ZZZZZ12345", identifier: signing.identifier), SigningIdentity(authority: "developer", teamId: signing.teamId, identifier: "com.impostor")] {
            XCTAssertNil(policy.decision(for: FlowIdentity(executable: flow.executable, destination: flow.destination, port: flow.port, direction: .outbound, identity: identity)))
        }
        XCTAssertNil(SigningIdentity.verified(auditToken: nil))
        XCTAssertNil(SigningIdentity.verified(auditToken: Data(repeating: 0, count: 31)))
        XCTAssertNil(SigningIdentity(authority: "developer", teamId: signing.teamId, identifier: "x\" or true").requirement)
    }
    func testLegacyRuleCannotMatchEvenAValidSignatureAtItsOldPath() throws {
        let data = Data(#"{"id":"old","appKey":"/Applications/Mail.app/Contents/MacOS/Mail","app":"Mail","destination":"1.2.3.4","port":993,"direction":"both","action":"allow","updatedAt":1,"source":"live"}"#.utf8)
        let old = try JSONDecoder().decode(PolicyRule.self, from: data)
        XCTAssertNil(old.identity)
        XCTAssertNil(Policy(rules: [old]).decision(for: flow))
    }

}
