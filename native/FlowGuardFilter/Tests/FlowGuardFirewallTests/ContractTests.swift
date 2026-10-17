import XCTest
@testable import FlowGuardFirewall
final class ContractTests: XCTestCase {
    func testFixedCommands() {
        XCTAssertEqual(FirewallAction.enable.arguments, ["--setglobalstate", "on"])
        XCTAssertEqual(FirewallAction.disable.arguments, ["--setglobalstate", "off"])
        for action in [FirewallAction.status, .register, .unregister] { XCTAssertNil(action.arguments) }
        XCTAssertNil(FirewallAction(rawValue: "--setappblocked /tmp/app"))
    }
    func testReadbackEvidence() {
        XCTAssertEqual(FirewallContract.parseState("Firewall is enabled. (State = 1)"), true)
        XCTAssertEqual(FirewallContract.parseState("Firewall is disabled. (State = 0)"), false)
        for invalid in ["", "enabled", "State = 1", "Firewall is enabled. (State = 2)", "Firewall is enabled. (State = 1)\nextra"] { XCTAssertNil(FirewallContract.parseState(invalid)) }
    }
    func testWritesNeedAuthorizationAndMatchingKnownState() {
        for action in FirewallAction.allCases {
            for expected in [true, false, nil] as [Bool?] {
                for actual in [true, false, nil] as [Bool?] {
                    for auth in [true, false] {
                        XCTAssertEqual(FirewallContract.canWrite(action: action, expected: expected, actual: actual, authorized: auth), action.target != nil && auth && expected != nil && actual == expected)
                    }
                }
            }
        }
    }
}
extension ContractTests {
    func testTransactionChecksBeforeWritingAndReadsBack() {
        var calls: [[String]] = []
        let reply = FirewallTransaction.perform(action: .enable, expected: false, authorized: { true }, current: { true }) { args in
            calls.append(args)
            if calls.count == 1 { return "Firewall is disabled. (State = 0)" }
            return args == ["--getglobalstate"] ? "Firewall is enabled. (State = 1)" : ""
        }
        XCTAssertNil(reply.error); XCTAssertEqual(reply.enabled, true)
        XCTAssertEqual(calls, [["--getglobalstate"], ["--setglobalstate", "on"], ["--getglobalstate"]])
        for denied in [true, false] {
            calls = []
            _ = FirewallTransaction.perform(action: .disable, expected: true, authorized: { !denied }, current: { denied }) { calls.append($0); return nil }
            XCTAssertTrue(calls.isEmpty)
        }
    }
    func testStateConflictAndUnverifiedWritesNeverSucceed() {
        var writes = 0
        let stale = FirewallTransaction.perform(action: .enable, expected: false, authorized: { true }, current: { true }) { args in
            if args.first == "--setglobalstate" { writes += 1 }
            return "Firewall is enabled. (State = 1)"
        }
        XCTAssertEqual(stale.error, "state_changed"); XCTAssertEqual(writes, 0)
        for readback in ["", "Firewall is disabled. (State = 0)"] {
            let result = FirewallTransaction.perform(action: .enable, expected: false, authorized: { true }, current: { true }) { args in
                if args.first == "--setglobalstate" { return "" }
                return readback.isEmpty ? readback : "Firewall is disabled. (State = 0)"
            }
            XCTAssertNotNil(result.error)
        }
        var alive = true
        writes = 0
        let disconnected = FirewallTransaction.perform(action: .disable, expected: true, authorized: { true }, current: { alive }) { args in
            if args.first == "--setglobalstate" { writes += 1 }
            alive = false; return "Firewall is enabled. (State = 1)"
        }
        XCTAssertEqual(disconnected.error, "state_changed"); XCTAssertEqual(writes, 0)
    }
}
