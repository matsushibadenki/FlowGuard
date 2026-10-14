import Foundation
public enum RuleAction: String, Codable { case allow, block, ask }
public enum RuleDirection: String, Codable { case outbound, inbound, both }
public struct PolicyRule: Codable, Equatable {
    public let systemException: Bool?
    public let timeoutAction: RuleAction?
    public let identity: SigningIdentity?
    public let id: String
    public let appKey: String
    public let app: String
    public let destination: String
    public let port: UInt16
    public let direction: RuleDirection
    public let action: RuleAction
    public let updatedAt: UInt64
    public let source: String
}
public struct FlowIdentity {
    public let identity: SigningIdentity?
    public let executable: String
    public let destination: String
    public let port: UInt16
    public let direction: RuleDirection
    public init(executable: String, destination: String, port: UInt16, direction: RuleDirection, identity: SigningIdentity? = nil) {
        self.identity = identity; self.executable = executable; self.destination = destination; self.port = port; self.direction = direction
    }
}
public struct Policy {
    public let rules: [PolicyRule]
    public init(rules: [PolicyRule] = []) { self.rules = rules.filter { $0.source == "live" && $0.identity?.valid == true && $0.appKey == $0.identity?.key } }
    public func matchingRule(for flow: FlowIdentity) -> PolicyRule? {
        // A specific direction overrides a both-directions fallback, newest first.
        let matches = rules.filter { $0.identity != nil && flow.identity?.valid == true && $0.identity == flow.identity && $0.destination == flow.destination && $0.port == flow.port && ($0.direction == .both || $0.direction == flow.direction) }
        let matched = matches.sorted { lhs, rhs in
            let leftSpecific = lhs.direction != .both, rightSpecific = rhs.direction != .both
            return leftSpecific == rightSpecific ? lhs.updatedAt > rhs.updatedAt : leftSpecific
        }.first
        if SystemProtection.preset(for: flow) != nil && matched?.systemException != true { return nil }
        return matched
    }
    public func decision(for flow: FlowIdentity) -> RuleAction? { matchingRule(for: flow)?.action }
}
