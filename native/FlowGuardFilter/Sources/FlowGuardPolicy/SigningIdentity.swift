// パス: native/FlowGuardFilter/Sources/FlowGuardPolicy/SigningIdentity.swift
import Foundation
import Security
public struct SigningIdentity: Codable, Equatable {
    public let authority: String
    public let teamId: String
    public let identifier: String
    public init(authority: String, teamId: String, identifier: String) {
        self.authority = authority; self.teamId = teamId; self.identifier = identifier
    }
    public var valid: Bool {
        let allowed = CharacterSet(charactersIn: "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.-_")
        guard !identifier.isEmpty, identifier.utf8.count <= 256,
              identifier.unicodeScalars.allSatisfy({ allowed.contains($0) }) else { return false }
        if authority == "apple" { return teamId.isEmpty }
        return authority == "developer" && teamId.utf8.count == 10 && teamId.unicodeScalars.allSatisfy {
            CharacterSet(charactersIn: "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789").contains($0)
        }
    }
    public var key: String { "signed:v1:\(authority):\(teamId):\(identifier)" }
    public var requirement: String? {
        guard valid else { return nil }
        return authority == "apple" ? "anchor apple and identifier \"\(identifier)\"" :
            "anchor apple generic and certificate leaf[subject.OU] = \"\(teamId)\" and identifier \"\(identifier)\""
    }
    private static func satisfies(_ code: SecCode, _ identity: SigningIdentity) -> Bool {
        guard let text = identity.requirement else { return false }
        var requirement: SecRequirement?
        guard SecRequirementCreateWithString(text as CFString, [], &requirement) == errSecSuccess, let requirement else { return false }
        return SecCodeCheckValidity(code, [], requirement) == errSecSuccess
    }
    /// Never derive the security subject from a path or the flow's claimed bundle identifier.
    public static func verified(auditToken: Data?) -> SigningIdentity? {
        guard let auditToken, auditToken.count == 32 else { return nil }
        var code: SecCode?
        guard SecCodeCopyGuestWithAttributes(nil, [kSecGuestAttributeAudit as String: auditToken] as CFDictionary, [], &code) == errSecSuccess,
              let code, SecCodeCheckValidity(code, [], nil) == errSecSuccess else { return nil }
        var staticCode: SecStaticCode?
        var information: CFDictionary?
        guard SecCodeCopyStaticCode(code, [], &staticCode) == errSecSuccess, let staticCode,
              SecCodeCopySigningInformation(staticCode, SecCSFlags(rawValue: kSecCSSigningInformation), &information) == errSecSuccess,
              let info = information as? [String: Any], let identifier = info[kSecCodeInfoIdentifier as String] as? String else { return nil }
        let apple = SigningIdentity(authority: "apple", teamId: "", identifier: identifier)
        if satisfies(code, apple) { return apple }
        guard let team = info[kSecCodeInfoTeamIdentifier as String] as? String else { return nil }
        let developer = SigningIdentity(authority: "developer", teamId: team, identifier: identifier)
        return satisfies(code, developer) ? developer : nil
    }
}
