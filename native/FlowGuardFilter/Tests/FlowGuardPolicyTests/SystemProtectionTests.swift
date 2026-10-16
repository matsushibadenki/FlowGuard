import XCTest
@testable import FlowGuardPolicy
final class SystemProtectionTests: XCTestCase {
    func rule(action: String = "block", exception: Bool = false, direction: String = "both", destination: String = "1.1.1.1", identifier: String = "com.apple.mDNSResponder", port: Int = 53) throws -> PolicyRule {
        let json = """
        {"id":"\(direction)","source":"live","appKey":"signed:v1:apple::\(identifier)","identity":{"authority":"apple","teamId":"","identifier":"\(identifier)"},"app":"DNS","destination":"\(destination)","port":\(port),"direction":"\(direction)","action":"\(action)","updatedAt":1,"systemException":\(exception)}
        """
        return try JSONDecoder().decode(PolicyRule.self, from: Data(json.utf8))
    }
    func flow(_ direction: RuleDirection = .outbound, identity: SigningIdentity? = SigningIdentity(authority:"apple",teamId:"",identifier:"com.apple.mDNSResponder"), port: UInt16 = 53) -> FlowIdentity {
        FlowIdentity(transport: .tcp, executable:"/untrusted/path",destination:"1.1.1.1",port:port,direction:direction,identity:identity)
    }
    func testDefaultProtectionOverridesBlockAndAskAndMissingLegacyFlag() throws {
        for action in ["block","ask"] {
            XCTAssertNil(Policy(rules:[try rule(action:action)]).matchingRule(for:flow()))
        }
        let update = flow(identity:SigningIdentity(authority:"apple",teamId:"",identifier:"com.apple.softwareupdated"),port:443)
        XCTAssertNil(Policy(rules:[try rule(identifier:"com.apple.softwareupdated",port:443)]).decision(for:update))
    }
    func testExceptionIsExactAndWinningRuleControlsIt() throws {
        XCTAssertEqual(Policy(rules:[try rule(exception:true)]).decision(for:flow()),.block)
        XCTAssertEqual(Policy(rules:[try rule(action:"ask",exception:true)]).decision(for:flow()),.ask)
        XCTAssertNil(Policy(rules:[try rule(exception:true,destination:"9.9.9.9")]).decision(for:flow()))
        XCTAssertNil(Policy(rules:[try rule(exception:true),try rule(direction:"outbound")]).decision(for:flow()))
        XCTAssertEqual(Policy(rules:[try rule()]).decision(for:flow(.inbound)),.block)
    }
    func testTransportCannotInheritTCPProtectionOrExceptions() throws {
        let subject = SigningIdentity(authority:"apple",teamId:"",identifier:"com.apple.mDNSResponder")
        for transport in [FlowTransport.udp, .unknown] {
            for port: UInt16 in [53,5353,22,443,5900] {
                let value = FlowIdentity(transport:transport,executable:"",destination:"1.1.1.1",port:port,direction:.outbound,identity:subject)
                XCTAssertNil(SystemProtection.preset(for:value))
                for action in ["allow","block","ask"] {
                    XCTAssertNil(Policy(rules:[try rule(action:action,exception:true,port:Int(port))]).matchingRule(for:value))
                }
            }
        }
    }
    func testIdentityPortAndDirectionAloneNeverClaimProtection() {
        XCTAssertNil(SystemProtection.preset(for:flow(identity:nil)))
        XCTAssertNil(SystemProtection.preset(for:flow(identity:SigningIdentity(authority:"developer",teamId:"ABCDE12345",identifier:"com.apple.mDNSResponder"))))
        XCTAssertNil(SystemProtection.preset(for:flow(port:443)))
        XCTAssertNil(SystemProtection.preset(for:flow(.inbound)))
    }
}
