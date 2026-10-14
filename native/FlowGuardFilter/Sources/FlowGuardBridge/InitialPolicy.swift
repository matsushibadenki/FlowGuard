import Foundation
import FlowGuardPolicy

/// Stored in the OS-owned provider configuration, never read from an arbitrary file.
public struct InitialPolicy: Codable {
    public static let key = "flowguard.initialPolicy.v3"
    public static let obsoleteKeys = ["flowguard.initialPolicy.v1", "flowguard.initialPolicy.v2"]
    public static let legacyKey = "flowguard.initialPolicy.v1"
    public static func vendorConfiguration(previous: [String: Any], policy: Data) -> [String: Any] {
        var result = previous
        obsoleteKeys.forEach { result.removeValue(forKey: $0) }
        result[key] = policy
        return result
    }
    public let rules: [PolicyRule]
    public static func persisted(enabled: Bool, provider: String?, stored: Data?, requested: Data) -> Bool {
        enabled && provider == "com.flowguard.filter" && stored == requested && (try? decode(requested)) != nil
    }
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
