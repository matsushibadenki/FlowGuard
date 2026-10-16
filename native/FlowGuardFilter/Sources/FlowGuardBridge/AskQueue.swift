// パス: native/FlowGuardFilter/Sources/FlowGuardBridge/AskQueue.swift
import Foundation
import FlowGuardPolicy

public struct AskRequest: Codable {
    public let requestId: String
    public let instanceId: String
    public let policyId: String
    public let ruleId: String
    public let identity: SigningIdentity
    public let app: String
    public let destination: String
    public let port: UInt16
    public let direction: RuleDirection
    public let timeoutAction: RuleAction
    public var remainingSeconds: Int
}
public struct AskOutcome: Codable {
    public let request: AskRequest
    public let action: RuleAction
    public let reason: String
}
public struct AskSnapshot: Codable {
    public let protocolVersion: Int
    public let instanceId: String
    public let policyId: String
    public let requests: [AskRequest]
    public let outcomes: [AskOutcome]
}
public struct AskAnswer: Codable {
    public let protocolVersion: Int
    public let instanceId: String
    public let policyId: String
    public let requestId: String
    public let action: RuleAction
    public init(instanceId: String, policyId: String, requestId: String, action: RuleAction) {
        protocolVersion = 4; self.instanceId = instanceId; self.policyId = policyId
        self.requestId = requestId; self.action = action
    }
}
public struct AskReply: Codable {
    public let protocolVersion: Int
    public let instanceId: String
    public let policyId: String
    public let requestId: String
    public let status: String
    public let action: RuleAction?
}
/// The provider owns flow callbacks; the UI receives only bounded metadata.
/// Continuous deadlines advance across sleep; wall-clock changes cannot extend a paused flow.
public final class AskQueue {
    public static let timeoutSeconds = 15
    public static let capacity = 128
    public let instanceId: String
    private static let continuousClock = ContinuousClock()
    private static let origin = continuousClock.now
    public static func monotonicNow() -> TimeInterval {
        let elapsed = origin.duration(to: continuousClock.now).components
        return Double(elapsed.seconds) + Double(elapsed.attoseconds) / 1_000_000_000_000_000_000
    }
    private let lock = NSLock()
    private var policyId = UUID().uuidString
    private var active = false
    private let clock: () -> TimeInterval
    private struct Pending {
        var request: AskRequest
        let deadline: TimeInterval
        let completion: (RuleAction) -> Void
    }
    private var pending: [String: Pending] = [:]
    private var outcomes: [AskOutcome] = []
    public init(instanceId: String = UUID().uuidString, clock: @escaping () -> TimeInterval = { AskQueue.monotonicNow() }) {
        self.instanceId = instanceId; self.clock = clock
    }
    public func activate() { lock.lock(); active = true; lock.unlock() }
    private func record(_ item: Pending, action: RuleAction, reason: String) {
        outcomes.append(AskOutcome(request: item.request, action: action, reason: reason))
        if outcomes.count > 64 { outcomes.removeFirst(outcomes.count - 64) }
    }
    /// Caller runs callbacks after releasing its own policy lock.
    public func changePolicy(reason: String = "policy_changed") -> [() -> Void] {
        lock.lock(); defer { lock.unlock() }
        policyId = UUID().uuidString
        let old = Array(pending.values); pending.removeAll()
        return old.map { item in
            record(item, action: item.request.timeoutAction, reason: reason)
            return { item.completion(item.request.timeoutAction) }
        }
    }
    public func shutdown() {
        lock.lock(); active = false; lock.unlock()
        let callbacks = changePolicy(reason: "provider_stopped"); callbacks.forEach { $0() }
    }
    public func enqueue(rule: PolicyRule, flow: FlowIdentity, completion: @escaping (RuleAction) -> Void) -> AskRequest? {
        lock.lock(); defer { lock.unlock() }
        guard flow.transport == .tcp, rule.action == .ask,
              let identity = flow.identity, identity.valid, rule.identity == identity,
              rule.appKey == identity.key, rule.destination == flow.destination, rule.port == flow.port,
              flow.direction != .both, rule.direction == .both || rule.direction == flow.direction,
              rule.timeoutAction == nil || rule.timeoutAction == .allow || rule.timeoutAction == .block else { return nil }
        let request = AskRequest(requestId: UUID().uuidString, instanceId: instanceId, policyId: policyId,
            ruleId: rule.id, identity: identity, app: rule.app, destination: flow.destination,
            port: flow.port, direction: flow.direction, timeoutAction: rule.timeoutAction ?? .allow,
            remainingSeconds: Self.timeoutSeconds)
        let item = Pending(request: request, deadline: clock() + Double(Self.timeoutSeconds), completion: completion)
        guard active, pending.count < Self.capacity else {
            record(item, action: request.timeoutAction, reason: active ? "capacity" : "transport_unavailable")
            return nil
        }
        pending[request.requestId] = item
        return request
    }
    private func expireLocked() -> [() -> Void] {
        let expired = pending.values.filter { $0.deadline <= clock() }
        for item in expired {
            pending.removeValue(forKey: item.request.requestId)
            record(item, action: item.request.timeoutAction, reason: "timed_out")
        }
        return expired.map { item in { item.completion(item.request.timeoutAction) } }
    }
    public func expire() {
        lock.lock(); let callbacks = expireLocked(); lock.unlock()
        callbacks.forEach { $0() }
    }
    public func snapshot() -> AskSnapshot {
        lock.lock()
        let callbacks = expireLocked()
        let requests = pending.values.map { item -> AskRequest in
            var request = item.request
            request.remainingSeconds = max(0, Int(ceil(item.deadline - clock())))
            return request
        }.sorted { $0.requestId < $1.requestId }
        let snapshot = AskSnapshot(protocolVersion: 4, instanceId: instanceId, policyId: policyId, requests: requests, outcomes: outcomes)
        lock.unlock(); callbacks.forEach { $0() }; return snapshot
    }
    public func answer(_ answer: AskAnswer) -> AskReply {
        lock.lock()
        var callbacks = expireLocked()
        var status = "invalid"
        var action: RuleAction?
        if answer.protocolVersion == 4, answer.action != .ask, UUID(uuidString: answer.requestId) != nil {
            if answer.instanceId != instanceId || answer.policyId != policyId || !active { status = "stale" }
            else if let item = pending.removeValue(forKey: answer.requestId) {
                record(item, action: answer.action, reason: "answered")
                callbacks.append { item.completion(answer.action) }; action = answer.action; status = "answered"
            } else if let outcome = outcomes.last(where: { $0.request.requestId == answer.requestId }) {
                status = outcome.reason == "timed_out" ? "expired" : "already_handled"; action = outcome.action
            } else { status = "not_found" }
        }
        let reply = AskReply(protocolVersion: 4, instanceId: instanceId, policyId: policyId,
            requestId: answer.requestId, status: status, action: action)
        lock.unlock(); callbacks.forEach { $0() }; return reply
    }
}
