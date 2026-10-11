import Foundation
import FlowGuardPolicy

/// Stored in the OS-owned provider configuration, never read from an arbitrary file.
public struct InitialPolicy: Codable {
    public static let key = "flowguard.initialPolicy.v1"
    public let rules: [PolicyRule]
    public static func decode(_ data: Data) throws -> InitialPolicy {
        guard data.count <= 4 * 1024 * 1024 else { throw NSError(domain: "FlowGuard.invalid_initial_policy", code: 1) }
        let policy = try JSONDecoder().decode(Self.self, from: data)
        guard BridgeContract.validRules(policy.rules) else { throw NSError(domain: "FlowGuard.invalid_initial_policy", code: 2) }
        return policy
    }
}

public enum FilterConfigurationOwnership {
    public static func mayModify(hasConfiguration: Bool, providerIdentifier: String?, enabled: Bool) -> Bool {
        if hasConfiguration { return providerIdentifier == "com.flowguard.filter" }
        return !enabled
    }
}
