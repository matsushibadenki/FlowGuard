import Foundation
import NetworkExtension
import Security
import FlowGuardPolicy
import FlowGuardBridge

/// Extension source only. Not installed or activated by the desktop observer build.
/// The host must provide authenticated rule IPC and an acknowledgement before marking rules active.
@objc(FlowGuardFilterDataProvider)
public final class FilterDataProvider: NEFilterDataProvider {
    private var bridge: BridgeService?
    private let lock = NSLock()
    private var policy = Policy()
    private var pending: [UUID: NEFilterFlow] = [:]
    private let work = DispatchQueue(label: "com.flowguard.filter.decisions")
    /// The extension host must supply an authenticated decision transport before asking is enabled.
    private var requestDecision: ((UUID, FlowIdentity) -> Void)?
    public func setDecisionTransport(_ callback: ((UUID, FlowIdentity) -> Void)?) {
        lock.lock(); requestDecision = callback; lock.unlock()
    }

    public override func startFilter(completionHandler: @escaping (Error?) -> Void) {
        do {
            guard let data = filterConfiguration.vendorConfiguration?[InitialPolicy.key] as? Data else { throw NSError(domain: "FlowGuard.missing_initial_policy", code: 1) }
            try replacePolicy(InitialPolicy.decode(data).rules)
        } catch { completionHandler(error); return }
        let network = NENetworkRule(remoteNetwork: nil, remotePrefix: 0, localNetwork: nil, localPrefix: 0, protocol: .TCP, direction: .any)
        let settings = NEFilterSettings(rules: [NEFilterRule(networkRule: network, action: .filterData)], defaultAction: .allow)
        apply(settings) { [weak self] error in
            guard let self, error == nil else { completionHandler(error); return }
            let service = BridgeService { [weak self] rules in
                guard let self else { throw NSError(domain: "FlowGuardBridge", code: 2) }
                try self.replacePolicy(rules)
            }
            do { try service.start(); self.bridge = service; completionHandler(nil) }
            catch { completionHandler(error) }
        }
    }
    public override func stopFilter(with reason: NEProviderStopReason, completionHandler: @escaping () -> Void) {
        bridge?.stop(); bridge = nil
        lock.lock(); let paused = Array(pending.values); pending.removeAll(); lock.unlock()
        for flow in paused { resumeFlow(flow, with: NEFilterNewFlowVerdict.allow()) }
        completionHandler()
    }
    /// Call only from a transport that verifies the signing identity of the host.
    public func replacePolicy(_ rules: [PolicyRule]) throws {
        guard rules.count <= 2000 else { throw NSError(domain: "FlowGuard", code: 1) }
        lock.lock(); policy = Policy(rules: rules); lock.unlock()
    }
    public override func handleNewFlow(_ flow: NEFilterFlow) -> NEFilterNewFlowVerdict {
        guard let socket = flow as? NEFilterSocketFlow,
              socket.socketProtocol == IPPROTO_TCP,
              let remote = socket.remoteEndpoint as? NWHostEndpoint,
              let port = UInt16(remote.port),
              let executable = executablePath(from: flow.sourceAppAuditToken) else { return .allow() }
        let identity = FlowIdentity(executable: executable, destination: remote.hostname, port: port,
                                    direction: flow.direction == .inbound ? .inbound : .outbound)
        lock.lock(); let decision = policy.decision(for: identity); let callback = requestDecision; lock.unlock()
        switch decision {
        case .block: return .drop()
        case .ask:
            // Without a decision transport the host must report this rule as unsupported.
            guard let callback else { return .allow() }
            let id = UUID()
            lock.lock()
            guard pending.count < 128 else { lock.unlock(); return .allow() }
            pending[id] = flow; lock.unlock()
            work.async { callback(id, identity) }
            // A lost UI cannot indefinitely hang system networking. Timeout policy: allow.
            work.asyncAfter(deadline: .now() + 15) { [weak self] in self?.resolve(id, allow: true) }
            return .pause()
        default: return .allow()
        }
    }
    public func resolve(_ id: UUID, allow: Bool) {
        lock.lock(); let flow = pending.removeValue(forKey: id); lock.unlock()
        if let flow { resumeFlow(flow, with: allow ? NEFilterNewFlowVerdict.allow() : NEFilterNewFlowVerdict.drop()) }
    }
    private func executablePath(from auditToken: Data?) -> String? {
        guard let auditToken else { return nil }
        var code: SecCode?
        let attributes = [kSecGuestAttributeAudit as String: auditToken] as CFDictionary
        guard SecCodeCopyGuestWithAttributes(nil, attributes, [], &code) == errSecSuccess, let code else { return nil }
        var staticCode: SecStaticCode?
        guard SecCodeCopyStaticCode(code, [], &staticCode) == errSecSuccess, let staticCode else { return nil }
        var information: CFDictionary?
        guard SecCodeCopySigningInformation(staticCode, [], &information) == errSecSuccess,
              let info = information as? [String: Any],
              let executable = info[kSecCodeInfoMainExecutable as String] as? URL else { return nil }
        return executable.path
    }
}
