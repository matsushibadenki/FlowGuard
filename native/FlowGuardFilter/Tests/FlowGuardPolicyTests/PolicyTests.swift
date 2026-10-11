import XCTest
@testable import FlowGuardPolicy
final class PolicyTests: XCTestCase {
    func rule(_ source: String = "live", _ direction: String = "both", _ action: String = "block", _ updated: Int = 1) throws -> PolicyRule {
        let json = """
        {"id":"\(source)-\(direction)","appKey":"/Applications/Mail.app/Contents/MacOS/Mail","app":"Mail","destination":"1.2.3.4","port":993,"direction":"\(direction)","action":"\(action)","updatedAt":\(updated),"source":"\(source)"}
        """
        return try JSONDecoder().decode(PolicyRule.self, from: Data(json.utf8))
    }
    let flow = FlowIdentity(executable: "/Applications/Mail.app/Contents/MacOS/Mail", destination: "1.2.3.4", port: 993, direction: .outbound)
    func testDemoRulesCannotEnforce() throws {
        XCTAssertNil(Policy(rules: [try rule("demo")]).decision(for: flow))
    }
    func testExactIdentityAndPort() throws {
        let policy = Policy(rules: [try rule()])
        XCTAssertEqual(policy.decision(for: flow), .block)
        XCTAssertNil(policy.decision(for: FlowIdentity(executable: "/other/Mail", destination: "1.2.3.4", port: 993, direction: .outbound)))
        XCTAssertNil(policy.decision(for: FlowIdentity(executable: flow.executable, destination: "1.2.3.4", port: 443, direction: .outbound)))
    }
    func testSpecificDirectionHasPriority() throws {
        let policy = Policy(rules: [try rule(), try rule("live", "outbound", "allow", 2)])
        XCTAssertEqual(policy.decision(for: flow), .allow)
        XCTAssertEqual(policy.decision(for: FlowIdentity(executable: flow.executable, destination: flow.destination, port: flow.port, direction: .inbound)), .block)
    }
}
