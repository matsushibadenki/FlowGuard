import Foundation
import Security
import Darwin
public enum BridgeSigning {
    public static func teamIdentifier() -> String? {
        var code: SecCode?
        guard SecCodeCopySelf([], &code) == errSecSuccess, let code, SecCodeCheckValidity(code, [], nil) == errSecSuccess else { return nil }
        var staticCode: SecStaticCode?
        guard SecCodeCopyStaticCode(code, [], &staticCode) == errSecSuccess, let staticCode else { return nil }
        var info: CFDictionary?
        guard SecCodeCopySigningInformation(staticCode, [], &info) == errSecSuccess,
              let team = (info as? [String: Any])?[kSecCodeInfoTeamIdentifier as String] as? String,
              team.range(of: "^[A-Z0-9]{10}$", options: .regularExpression) != nil else { return nil }
        return team
    }
    public static func trustedDesktopParent() -> Bool {
        guard let team = teamIdentifier() else { return false }
        var parent: SecCode?
        let attributes = [kSecGuestAttributePid as String: NSNumber(value: getppid())] as CFDictionary
        guard SecCodeCopyGuestWithAttributes(nil, attributes, [], &parent) == errSecSuccess, let parent else { return false }
        var expected: SecRequirement?
        guard SecRequirementCreateWithString(requirement(identifier: "com.flowguard.desktop", team: team) as CFString, [], &expected) == errSecSuccess, let expected else { return false }
        return SecCodeCheckValidity(parent, [], expected) == errSecSuccess
    }
    public static func requirement(identifier: String, team: String) -> String {
        "anchor apple generic and identifier \"\(identifier)\" and certificate leaf[subject.OU] = \"\(team)\""
    }
}
