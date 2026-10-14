// パス: native/FlowGuardFilter/Tests/FlowGuardBridgeTests/AskQueueTests.swift
import XCTest
import FlowGuardBridge
import FlowGuardPolicy
final class AskQueueTests: XCTestCase {
    func rule(_ fallback: String = "allow") throws -> PolicyRule {
        let data = Data("""
        {"id":"ask","appKey":"signed:v1:developer:ABCDE12345:com.test.app","identity":{"authority":"developer","teamId":"ABCDE12345","identifier":"com.test.app"},"app":"Test","destination":"1.2.3.4","port":443,"direction":"both","action":"ask","timeoutAction":"\(fallback)","updatedAt":1,"source":"live"}
        """.utf8)
        return try JSONDecoder().decode(PolicyRule.self, from: data)
    }
    func flow(_ rule: PolicyRule) -> FlowIdentity {
        FlowIdentity(executable: "", destination: rule.destination, port: rule.port, direction: .outbound, identity: rule.identity)
    }
    func answer(_ request: AskRequest, _ action: RuleAction) -> AskAnswer {
        AskAnswer(instanceId: request.instanceId, policyId: request.policyId, requestId: request.requestId, action: action)
    }
    func testTimeoutModesAndExactDeadlineRejectLateAllow() throws {
        for fallback in ["allow", "block"] {
            var now: TimeInterval = 100
            let queue = AskQueue(clock: { now }); queue.activate()
            let rule = try rule(fallback); var actions: [RuleAction] = []
            let request = try XCTUnwrap(queue.enqueue(rule: rule, flow: flow(rule)) { actions.append($0) })
            now = 114.9; XCTAssertEqual(queue.snapshot().requests.first?.remainingSeconds, 1)
            now = 115
            let reply = queue.answer(answer(request, .allow))
            XCTAssertEqual(reply.status, "expired")
            XCTAssertEqual(actions, [RuleAction(rawValue: fallback)!])
            queue.expire(); XCTAssertEqual(actions.count, 1)
            XCTAssertEqual(queue.snapshot().outcomes.last?.reason, "timed_out")
        }
    }
    func testSingleAnswerAndInvalidOrWrongInstanceCannotResume() throws {
        let queue = AskQueue(); queue.activate(); let rule = try rule(); var actions: [RuleAction] = []
        let request = try XCTUnwrap(queue.enqueue(rule: rule, flow: flow(rule)) { actions.append($0) })
        XCTAssertEqual(queue.answer(AskAnswer(instanceId: UUID().uuidString, policyId: request.policyId, requestId: request.requestId, action: .allow)).status, "stale")
        XCTAssertEqual(queue.answer(answer(request, .ask)).status, "invalid")
        XCTAssertTrue(actions.isEmpty)
        XCTAssertEqual(queue.answer(answer(request, .block)).status, "answered")
        XCTAssertEqual(queue.answer(answer(request, .allow)).status, "already_handled")
        XCTAssertEqual(actions, [.block])
    }
    func testCapacityAndInactiveFallbackRemainBounded() throws {
        let queue = AskQueue(); let rule = try rule("block")
        XCTAssertNil(queue.enqueue(rule: rule, flow: flow(rule)) { _ in XCTFail() })
        XCTAssertEqual(queue.snapshot().outcomes.last?.action, .block)
        XCTAssertEqual(queue.snapshot().outcomes.last?.reason, "transport_unavailable")
        queue.activate()
        for _ in 0..<AskQueue.capacity { XCTAssertNotNil(queue.enqueue(rule: rule, flow: flow(rule)) { _ in }) }
        for _ in 0..<80 { XCTAssertNil(queue.enqueue(rule: rule, flow: flow(rule)) { _ in XCTFail() }) }
        XCTAssertEqual(queue.snapshot().requests.count, 128)
        XCTAssertEqual(queue.snapshot().outcomes.count, 64)
        XCTAssertEqual(queue.snapshot().outcomes.last?.reason, "capacity")
        XCTAssertEqual(queue.snapshot().outcomes.last?.action, .block)
    }
    func testPolicyChangeAndStopUseCapturedFallbackAndInvalidateAnswers() throws {
        let queue = AskQueue(); queue.activate(); let rule = try rule("block"); var actions: [RuleAction] = []
        let old = try XCTUnwrap(queue.enqueue(rule: rule, flow: flow(rule)) { actions.append($0) })
        queue.changePolicy().forEach { $0() }
        XCTAssertEqual(actions, [.block]); XCTAssertEqual(queue.answer(answer(old, .allow)).status, "stale")
        let next = try XCTUnwrap(queue.enqueue(rule: rule, flow: flow(rule)) { actions.append($0) })
        queue.shutdown(); XCTAssertEqual(actions, [.block, .block])
        XCTAssertEqual(queue.snapshot().outcomes.last?.reason, "provider_stopped")
        XCTAssertEqual(queue.answer(answer(next, .allow)).status, "stale")
    }
    func testUnsignedAndDifferentScopeNeverQueue() throws {
        let queue = AskQueue(); queue.activate(); let rule = try rule()
        XCTAssertNil(queue.enqueue(rule: rule, flow: FlowIdentity(executable: "/same/path", destination: rule.destination, port: rule.port, direction: .outbound)) { _ in XCTFail() })
        XCTAssertNil(queue.enqueue(rule: rule, flow: FlowIdentity(executable: "", destination: "other", port: rule.port, direction: .outbound, identity: rule.identity)) { _ in XCTFail() })
        XCTAssertTrue(queue.snapshot().requests.isEmpty)
    }
}
