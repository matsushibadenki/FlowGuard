import Foundation
import Security
import ServiceManagement
import FlowGuardBridge
import FlowGuardFirewall
import Darwin

func encode(_ value: FirewallReply) -> Data { (try? JSONEncoder().encode(value)) ?? Data() }
func output(_ value: FirewallReply) { FileHandle.standardOutput.write(encode(value)); FileHandle.standardOutput.write(Data("\n".utf8)) }
func authorized(_ data: Data) -> Bool {
    guard data.count == MemoryLayout<AuthorizationExternalForm>.size else { return false }
    var form = AuthorizationExternalForm()
    _ = withUnsafeMutableBytes(of: &form) { data.copyBytes(to: $0) }
    var reference: AuthorizationRef?
    guard AuthorizationCreateFromExternalForm(&form, &reference) == errAuthorizationSuccess, let reference else { return false }
    defer { AuthorizationFree(reference, []) }
    return FirewallContract.authorizationRight.withCString { name in
        var item = AuthorizationItem(name: name, valueLength: 0, value: nil, flags: 0)
        return withUnsafeMutablePointer(to: &item) { pointer in
            var rights = AuthorizationRights(count: 1, items: pointer)
            // Validate an already granted right; the daemon never initiates authentication.
            return AuthorizationCopyRights(reference, &rights, nil, [], nil) == errAuthorizationSuccess
        }
    }
}
final class Client: NSObject, FirewallProtocol {
    private let lock = NSLock(); private var consumed = false
    private var alive = true
    private let deadline = ProcessInfo.processInfo.systemUptime + 15
    func invalidate() { lock.lock(); alive = false; lock.unlock() }
    private func current() -> Bool { lock.lock(); defer { lock.unlock() }; return alive && ProcessInfo.processInfo.systemUptime < deadline }
    func change(_ value: String, expected: Bool, authorization: Data, withReply reply: @escaping (Data) -> Void) {
        lock.lock(); let used = consumed; consumed = true; lock.unlock()
        guard !used, let action = FirewallAction(rawValue: value), action.target != nil, authorized(authorization) else {
            reply(encode(FirewallReply(service: "enabled", error: "authorization_failed"))); return
        }
        // One serial transaction across all clients: compare, fixed write, then independent readback.
        Server.queue.async {
            reply(encode(FirewallTransaction.perform(action: action, expected: expected,
                authorized: { authorized(authorization) }, current: { self.current() }, run: FirewallTool.run)))
        }
    }
}
final class Server: NSObject, NSXPCListenerDelegate {
    static let queue = DispatchQueue(label: "com.flowguard.firewall.write")
    private let lock = NSLock(); private var clients: [NSXPCConnection] = []
    func listener(_ listener: NSXPCListener, shouldAcceptNewConnection connection: NSXPCConnection) -> Bool {
        lock.lock(); defer { lock.unlock() }
        guard clients.count < 4 else { return false }
        connection.exportedInterface = NSXPCInterface(with: FirewallProtocol.self)
        let client = Client()
        connection.exportedObject = client
        connection.invalidationHandler = { [weak self, weak connection] in
            client.invalidate()
            guard let self, let connection else { return }
            self.lock.lock(); self.clients.removeAll { $0 === connection }; self.lock.unlock()
        }
        clients.append(connection); connection.resume()
        DispatchQueue.global().asyncAfter(deadline: .now() + 20) { [weak connection] in connection?.invalidate() }
        return true
    }
}
let args = Array(CommandLine.arguments.dropFirst())
guard let team = BridgeSigning.teamIdentifier() else { output(FirewallReply(service: "unavailable", error: "signature_required")); exit(1) }
if args == ["--daemon"] {
    guard geteuid() == 0 else { exit(1) }
    let server = Server(), listener = NSXPCListener(machServiceName: FirewallContract.identifier)
    listener.setConnectionCodeSigningRequirement(BridgeSigning.requirement(identifier: FirewallContract.identifier, team: team))
    listener.delegate = server; listener.resume(); dispatchMain()
}
guard BridgeSigning.trustedDesktopParent(), args.count >= 1,
      let action = FirewallAction(rawValue: String(args[0].dropFirst(2))), args[0].hasPrefix("--"),
      args.count == (action.target == nil ? 1 : 2) else { output(FirewallReply(service: "unavailable", error: "trusted_desktop_required")); exit(1) }
let service = SMAppService.daemon(plistName: FirewallContract.plist)
func serviceState() -> String {
    switch service.status { case .enabled: return "enabled"; case .requiresApproval: return "requires_approval"; case .notRegistered: return "not_registered"; default: return "unavailable" }
}
if action == .approvalSettings {
    SMAppService.openSystemSettingsLoginItems()
    output(FirewallReply(service: serviceState(), enabled: FirewallTool.state())); exit(0)
}
if action == .register || action == .unregister {
    do {
        if action == .register { try service.register() } else { try service.unregister() }
        output(FirewallReply(service: serviceState(), enabled: FirewallTool.state()))
    } catch { output(FirewallReply(service: serviceState(), error: "registration_failed")) }
    exit(0)
}
if action == .status { output(FirewallReply(service: serviceState(), enabled: FirewallTool.state())); exit(0) }
guard service.status == .enabled, ["on", "off"].contains(args[1]) else { output(FirewallReply(service: serviceState(), error: "approval_required")); exit(0) }
var auth: AuthorizationRef?
guard AuthorizationCreate(nil, nil, [], &auth) == errAuthorizationSuccess, let auth else { output(FirewallReply(service: serviceState(), error: "authorization_failed")); exit(0) }
let status = FirewallContract.authorizationRight.withCString { name in
    var item = AuthorizationItem(name: name, valueLength: 0, value: nil, flags: 0)
    return withUnsafeMutablePointer(to: &item) { pointer in
        var rights = AuthorizationRights(count: 1, items: pointer)
        return AuthorizationCopyRights(auth, &rights, nil, [.interactionAllowed, .extendRights], nil)
    }
}
guard status == errAuthorizationSuccess else { AuthorizationFree(auth, [.destroyRights]); output(FirewallReply(service: serviceState(), error: "authorization_cancelled")); exit(0) }
var external = AuthorizationExternalForm()
guard AuthorizationMakeExternalForm(auth, &external) == errAuthorizationSuccess else { AuthorizationFree(auth, [.destroyRights]); exit(1) }
let data = withUnsafeBytes(of: &external) { Data($0) }
let connection = NSXPCConnection(machServiceName: FirewallContract.identifier, options: .privileged)
connection.remoteObjectInterface = NSXPCInterface(with: FirewallProtocol.self)
connection.setCodeSigningRequirement(BridgeSigning.requirement(identifier: FirewallContract.identifier, team: team))
let finishLock = NSLock(); var finished = false
func finish(_ reply: FirewallReply) {
    finishLock.lock(); guard !finished else { finishLock.unlock(); return }; finished = true; finishLock.unlock()
    AuthorizationFree(auth, [.destroyRights]); output(reply); connection.invalidate(); exit(0)
}
connection.invalidationHandler = { finish(FirewallReply(service: "unavailable", error: "connection_lost")) }
connection.resume()
DispatchQueue.global().asyncAfter(deadline: .now() + 15) { finish(FirewallReply(service: serviceState(), error: "result_unknown")) }
let remote = connection.remoteObjectProxyWithErrorHandler { _ in finish(FirewallReply(service: serviceState(), error: "connection_lost")) } as? FirewallProtocol
remote?.change(action.rawValue, expected: args[1] == "on", authorization: data) { bytes in
    finish((try? JSONDecoder().decode(FirewallReply.self, from: bytes)) ?? FirewallReply(service: serviceState(), error: "invalid_reply"))
}
dispatchMain()
