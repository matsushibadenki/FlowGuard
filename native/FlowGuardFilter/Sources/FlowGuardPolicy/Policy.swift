import Foundation
public enum RuleAction: String, Codable { case allow, block, ask }
public enum RuleDirection: String, Codable { case outbound, inbound, both }
public struct PolicyRule: Codable, Equatable {
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
    public let executable: String
    public let destination: String
    public let port: UInt16
    public let direction: RuleDirection
    public init(executable: String, destination: String, port: UInt16, direction: RuleDirection) {
        self.executable = executable; self.destination = destination; self.port = port; self.direction = direction
    }
}
public struct Policy {
    public let rules: [PolicyRule]
    public init(rules: [PolicyRule] = []) { self.rules = rules.filter { $0.source == "live" } }
    public func decision(for flow: FlowIdentity) -> RuleAction? {
        // A specific direction overrides a both-directions fallback, newest first.
        let matches = rules.filter { $0.appKey == flow.executable && $0.destination == flow.destination && $0.port == flow.port && ($0.direction == .both || $0.direction == flow.direction) }
        return matches.sorted { lhs, rhs in
            let leftSpecific = lhs.direction != .both, rightSpecific = rhs.direction != .both
            return leftSpecific == rightSpecific ? lhs.updatedAt > rhs.updatedAt : leftSpecific
        }.first?.action
    }
}
