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
    private let asks = AskQueue()
    private var askTimer: DispatchSourceTimer?
    private let work = DispatchQueue(label: "com.flowguard.filter.decisions")
    public override func startFilter(completionHandler: @escaping (Error?) -> Void) {
        let initial: [PolicyRule]
        do {
            guard let data = filterConfiguration.vendorConfiguration?[InitialPolicy.key] as? Data else { throw NSError(domain: "FlowGuard.missing_initial_policy", code: 1) }
            initial = try InitialPolicy.decode(data).rules
            try replacePolicy(initial)
        } catch { completionHandler(error); return }
        let network = NENetworkRule(remoteNetwork: nil, remotePrefix: 0, localNetwork: nil, localPrefix: 0, protocol: .TCP, direction: .any)
        let settings = NEFilterSettings(rules: [NEFilterRule(networkRule: network, action: .filterData)], defaultAction: .allow)
        apply(settings) { [weak self] error in
            guard let self, error == nil else { completionHandler(error); return }
            let service = BridgeService(asks: self.asks) { [weak self] rules in
                guard let self else { throw NSError(domain: "FlowGuardBridge", code: 2) }
                try self.replacePolicy(rules)
            }
            do {
                try service.start(initial: initial); self.bridge = service
                let timer = DispatchSource.makeTimerSource(queue: self.work)
                timer.schedule(deadline: .now() + .milliseconds(250), repeating: .milliseconds(250), leeway: .milliseconds(20))
                timer.setEventHandler { [weak self] in self?.asks.expire() }
                timer.resume(); self.askTimer = timer
                completionHandler(nil)
            }
            catch { completionHandler(error) }
        }
    }
    public override func stopFilter(with reason: NEProviderStopReason, completionHandler: @escaping () -> Void) {
        askTimer?.cancel(); askTimer = nil
        bridge?.stop(); bridge = nil
        work.async { completionHandler() }
    }
    /// Call only from a transport that verifies the signing identity of the host.
    public func replacePolicy(_ rules: [PolicyRule]) throws {
        guard BridgeContract.validRules(rules) else { throw NSError(domain: "FlowGuard", code: 1) }
        lock.lock(); policy = Policy(rules: rules); let cancelled = asks.changePolicy(); lock.unlock()
        cancelled.forEach { $0() }
    }
    public override func handleNewFlow(_ flow: NEFilterFlow) -> NEFilterNewFlowVerdict {
        guard let socket = flow as? NEFilterSocketFlow,
              socket.socketProtocol == IPPROTO_TCP,
              let remote = socket.remoteEndpoint as? NWHostEndpoint,
              let port = UInt16(remote.port),
              flow.direction == .inbound || flow.direction == .outbound,
              let signingIdentity = SigningIdentity.verified(auditToken: flow.sourceAppAuditToken) else { return .allow() }
        let identity = FlowIdentity(transport: .tcp, executable: "", destination: remote.hostname, port: port,
                                    direction: flow.direction == .inbound ? .inbound : .outbound, identity: signingIdentity)
        lock.lock()
        guard let rule = policy.matchingRule(for: identity) else { lock.unlock(); return .allow() }
        if rule.action == .ask {
            let request = asks.enqueue(rule: rule, flow: identity) { [weak self] action in
                self?.work.async { [weak self] in self?.resumeFlow(flow, with: action == .block ? NEFilterNewFlowVerdict.drop() : NEFilterNewFlowVerdict.allow()) }
            }
            lock.unlock()
            guard request != nil else { return rule.timeoutAction == .block ? .drop() : .allow() }
            return .pause()
        }
        lock.unlock()
        return rule.action == .block ? .drop() : .allow()
    }
}
