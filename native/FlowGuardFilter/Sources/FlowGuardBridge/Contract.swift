import Foundation
import CryptoKit
import FlowGuardPolicy

public struct PolicyUpdate: Codable {
    public let protocolVersion: Int
    public let requestID: String
    public let expectedInstanceID: String
    public let expectedRevision: UInt64
    public let rules: [PolicyRule]
    public init(requestID: String, expectedInstanceID: String, expectedRevision: UInt64, rules: [PolicyRule]) {
        protocolVersion = 1; self.expectedInstanceID = expectedInstanceID; self.requestID = requestID; self.expectedRevision = expectedRevision; self.rules = rules
    }
}
public struct BridgeReply: Codable {
    public let protocolVersion: Int
    public let requestID: String
    public let instanceID: String
    public let revision: UInt64
    public let status: String
    public let ruleCount: Int
    public let error: String?
    public let policyDigest: String?
}
@objc public protocol FilterBridgeProtocol {
    func status(withReply reply: @escaping (Data) -> Void)
    func replacePolicy(_ data: Data, withReply reply: @escaping (Data) -> Void)
}
/// Production and unsigned tests share this exact state machine. No mock transport can claim OS enforcement.
public final class PolicyReceiver {
    private let lock = NSLock()
    private let instanceID = UUID().uuidString
    private var revision: UInt64 = 0
    private var rules: [PolicyRule] = []
    private var ready = false
    public init() {}
    public func seed(_ initial: [PolicyRule]) { lock.lock(); defer { lock.unlock() }; rules = initial }
    public func setReady(_ value: Bool) { lock.lock(); ready = value; lock.unlock() }
    public func status() -> BridgeReply {
        lock.lock(); defer { lock.unlock() }
        return response("", ready ? "ready" : "unavailable", nil)
    }
    private func response(_ id: String, _ status: String, _ error: String?) -> BridgeReply {
        BridgeReply(protocolVersion: 1, requestID: id, instanceID: instanceID, revision: revision, status: status, ruleCount: rules.count, error: error, policyDigest: ready ? BridgeContract.digest(rules) : nil)
    }
    public func replace(_ data: Data, apply: ([PolicyRule]) throws -> Void) -> BridgeReply {
        lock.lock(); defer { lock.unlock() }
        guard data.count <= 4 * 1024 * 1024,
              let update = try? JSONDecoder().decode(PolicyUpdate.self, from: data) else { return response("", "rejected", "invalid_payload") }
        guard update.protocolVersion == 1, UUID(uuidString: update.requestID) != nil else { return response(update.requestID, "rejected", "invalid_protocol") }
        guard ready else { return response(update.requestID, "rejected", "provider_unavailable") }
        guard revision < UInt64.max, update.expectedInstanceID == instanceID, update.expectedRevision == revision else { return response(update.requestID, "rejected", "revision_conflict") }
        guard BridgeContract.validRules(update.rules) else { return response(update.requestID, "rejected", "unsupported_or_invalid_rules") }
        do {
            try apply(update.rules)
            rules = update.rules; revision += 1
            return response(update.requestID, "accepted", nil)
        } catch { return response(update.requestID, "rejected", "apply_failed") }
    }
}

public enum BridgeContract {
    public static func digest(_ rules: [PolicyRule]) -> String {
        var data = Data("FlowGuardPolicy/v1".utf8)
        for rule in rules {
            for field in [rule.id, rule.appKey, rule.app, rule.destination, String(rule.port), rule.direction.rawValue, rule.action.rawValue, String(rule.updatedAt), rule.source] {
                let bytes = Data(field.utf8)
                var count = UInt64(bytes.count).bigEndian
                withUnsafeBytes(of: &count) { data.append(contentsOf: $0) }; data.append(bytes)
            }
        }
        return SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
    }
    public static func validRules(_ rules: [PolicyRule]) -> Bool {
        rules.count <= 2000 && Set(rules.map(\.id)).count == rules.count &&
              rules.allSatisfy({ rule in
                  rule.source == "live" && !rule.id.isEmpty && rule.id.utf8.count <= 8192 &&
                  rule.appKey.hasPrefix("/") && rule.appKey.utf8.count <= 4096 && !rule.appKey.contains("\0") &&
                  !rule.destination.isEmpty && rule.destination.utf8.count <= 512 && !rule.destination.contains("\0") &&
                  rule.port > 0 && rule.app.utf8.count <= 256 && rule.action != .ask
              })
    }
    public static func validAcknowledgement(_ reply: BridgeReply, for update: PolicyUpdate) -> Bool {
        reply.protocolVersion == 1 && reply.requestID == update.requestID &&
        reply.instanceID == update.expectedInstanceID && reply.status == "accepted" && reply.error == nil &&
        update.expectedRevision < UInt64.max && reply.revision == update.expectedRevision + 1 && reply.ruleCount == update.rules.count && reply.policyDigest == digest(update.rules)
    }
}
