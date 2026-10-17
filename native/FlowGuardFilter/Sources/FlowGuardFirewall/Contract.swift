import Foundation

/// Fixed global switch only. No executable paths, per-app rules, shell text or arbitrary arguments.
public enum FirewallAction: String, Codable, CaseIterable {
    case status, register, unregister, approvalSettings = "approval_settings", enable, disable
    public var target: Bool? { self == .enable ? true : self == .disable ? false : nil }
    public var arguments: [String]? { target.map { ["--setglobalstate", $0 ? "on" : "off"] } }
}
public struct FirewallReply: Codable {
    public let service: String
    public let enabled: Bool?
    public let error: String?
    public init(service: String, enabled: Bool? = nil, error: String? = nil) {
        self.service = service; self.enabled = enabled; self.error = error
    }
}
public enum FirewallContract {
    public static let identifier = "com.flowguard.firewall-helper"
    public static let plist = identifier + ".plist"
    public static let authorizationRight = "system.privilege.admin"
    public static func parseState(_ text: String) -> Bool? {
        // socketfilterfw's numeric state is the evidence; never infer success from exit status alone.
        let lines = text.split(whereSeparator: \.isNewline)
        guard lines.count == 1 else { return nil }
        if lines[0] == "Firewall is enabled. (State = 1)" { return true }
        if lines[0] == "Firewall is disabled. (State = 0)" { return false }
        return nil
    }
    public static func canWrite(action: FirewallAction, expected: Bool?, actual: Bool?, authorized: Bool) -> Bool {
        action.target != nil && authorized && expected != nil && actual == expected
    }
}
@objc public protocol FirewallProtocol {
    func change(_ action: String, expected: Bool, authorization: Data, withReply reply: @escaping (Data) -> Void)
}

public enum FirewallTransaction {
    /// The runner is injected for tests; production supplies only the fixed system tool.
    public static func perform(action: FirewallAction, expected: Bool, authorized: () -> Bool, current: () -> Bool, run: ([String]) -> String?) -> FirewallReply {
        guard action.target != nil, authorized(), current() else { return FirewallReply(service: "enabled", error: "authorization_failed") }
        let before = run(["--getglobalstate"]).flatMap(FirewallContract.parseState)
        guard current(), authorized(), FirewallContract.canWrite(action: action, expected: expected, actual: before, authorized: true), let args = action.arguments else {
            return FirewallReply(service: "enabled", enabled: before, error: "state_changed")
        }
        let wrote = run(args) != nil
        let after = run(["--getglobalstate"]).flatMap(FirewallContract.parseState)
        return FirewallReply(service: "enabled", enabled: after, error: wrote && after == action.target ? nil : "verification_failed")
    }
}
