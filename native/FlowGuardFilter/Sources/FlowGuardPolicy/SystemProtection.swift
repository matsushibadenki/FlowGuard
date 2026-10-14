import Foundation
public enum SystemProtection {
    /// The caller supplies a Security.framework-verified subject and actual flow direction.
    public static func preset(for flow: FlowIdentity) -> String? {
        guard let identity = flow.identity, identity.valid, identity.authority == "apple", flow.direction == .outbound else { return nil }
        switch (identity.identifier, flow.port) {
        case ("com.apple.mDNSResponder", 53): return "systemDNS"
        case ("com.apple.softwareupdated", 443): return "systemUpdates"
        default: return nil
        }
    }
}
