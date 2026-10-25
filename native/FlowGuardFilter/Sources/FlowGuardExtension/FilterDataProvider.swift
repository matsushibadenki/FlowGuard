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
    private final class Scope {
        let asks = AskQueue()
        let flowNames = FlowNameStore()
        let temporaryGrants = TemporaryGrantStore()
        func stop() { asks.shutdown(); flowNames.stop(); temporaryGrants.stop() }
    }
    private var scope = Scope()
    private func currentScope() -> Scope { lock.lock(); defer { lock.unlock() }; return scope }
    private var askTimer: DispatchSourceTimer?
    private let work = DispatchQueue(label: "com.flowguard.filter.decisions")
    public override func startFilter(completionHandler: @escaping (Error?) -> Void) {
        let fresh = Scope()
        lock.lock(); let old = scope; let oldBridge = bridge; let oldTimer = askTimer
        scope = fresh; bridge = nil; askTimer = nil; policy = Policy(); lock.unlock()
        oldTimer?.cancel(); oldBridge?.stop(); old.stop()
        let initial: [PolicyRule]
        do {
            guard let data = filterConfiguration.vendorConfiguration?[InitialPolicy.key] as? Data else { throw NSError(domain: "FlowGuard.missing_initial_policy", code: 1) }
            initial = try InitialPolicy.decode(data).rules
            try replacePolicy(initial, expected: fresh)
        } catch { completionHandler(error); return }
        let network = NENetworkRule(remoteNetwork: nil, remotePrefix: 0, localNetwork: nil, localPrefix: 0, protocol: .TCP, direction: .any)
        let udp=NENetworkRule(remoteNetwork:nil,remotePrefix:0,localNetwork:nil,localPrefix:0,protocol:.UDP,direction:.any)
        let settings = NEFilterSettings(rules: [NEFilterRule(networkRule: network, action: .filterData),NEFilterRule(networkRule:udp,action:.filterData)], defaultAction: .allow)
        apply(settings) { [weak self] error in
            guard let self, error == nil else { completionHandler(error); return }
            guard self.currentScope() === fresh else { completionHandler(NSError(domain: "FlowGuard.stale_start", code: 1)); return }
            let service = BridgeService(asks: fresh.asks,names:fresh.flowNames,grants:fresh.temporaryGrants) { [weak self] rules in
                guard let self else { throw NSError(domain: "FlowGuardBridge", code: 2) }
                try self.replacePolicy(rules, expected: fresh)
            }
            do {
                self.lock.lock()
                guard self.scope === fresh else { self.lock.unlock(); completionHandler(NSError(domain: "FlowGuard.stale_start", code: 1)); return }
                do { try service.start(initial: initial) } catch { self.lock.unlock(); throw error }
                self.bridge = service
                let timer = DispatchSource.makeTimerSource(queue: self.work)
                timer.schedule(deadline: .now() + .milliseconds(250), repeating: .milliseconds(250), leeway: .milliseconds(20))
                timer.setEventHandler { fresh.asks.expire(); fresh.flowNames.expire() }
                timer.resume(); self.askTimer = timer; self.lock.unlock()
                completionHandler(nil)
            }
            catch { completionHandler(error) }
        }
    }
    public override func stopFilter(with reason: NEProviderStopReason, completionHandler: @escaping () -> Void) {
        let inactive = Scope(); inactive.stop()
        lock.lock(); let stopped = scope; let oldBridge = bridge; let oldTimer = askTimer
        scope = inactive; policy = Policy(); bridge = nil; askTimer = nil; lock.unlock()
        oldTimer?.cancel(); oldBridge?.stop(); stopped.stop()
        work.async { completionHandler() }
    }
    /// Call only from a transport that verifies the signing identity of the host.
    public func replacePolicy(_ rules: [PolicyRule]) throws { try replacePolicy(rules, expected: currentScope()) }
    private func replacePolicy(_ rules: [PolicyRule], expected: Scope) throws {
        guard BridgeContract.validRules(rules) else { throw NSError(domain: "FlowGuard", code: 1) }
        lock.lock()
        guard scope === expected else { lock.unlock(); throw NSError(domain: "FlowGuard.stale_policy", code: 1) }
        policy = Policy(rules: rules); let cancelled = expected.asks.changePolicy(); lock.unlock()
        cancelled.forEach { $0() }
    }
    public override func handleNewFlow(_ flow: NEFilterFlow) -> NEFilterNewFlowVerdict {
        let captured = currentScope()
        guard let socket = flow as? NEFilterSocketFlow,
              (socket.socketProtocol == IPPROTO_TCP || socket.socketProtocol == IPPROTO_UDP),
              let remote = socket.remoteEndpoint as? NWHostEndpoint,
              let port = UInt16(remote.port),
              flow.direction == .inbound || flow.direction == .outbound,
              let signingIdentity = SigningIdentity.verified(auditToken: flow.sourceAppAuditToken) else { return .allow() }
        if socket.socketProtocol == IPPROTO_TCP,let name=socket.remoteHostname,let local=socket.localEndpoint as? NWHostEndpoint,let localPort=UInt16(local.port){
            captured.flowNames.record(FlowNameRecord(localAddress:local.hostname,localPort:localPort,remoteAddress:remote.hostname,remotePort:port,transport:"TCP",direction:flow.direction == .inbound ? "inbound":"outbound",identity:signingIdentity,name:name,observedAt:UInt64(Date().timeIntervalSince1970)))
        }
        let local=socket.localEndpoint as? NWHostEndpoint
        let localAddress=local.flatMap{FlowEndpoint.normalized($0.hostname)}
        let localPort=local.flatMap{UInt16($0.port)}
        let interfaceName=captured.temporaryGrants.active ? localAddress.flatMap{FlowEndpoint.sourceInterface($0)}:nil
        let identity = FlowIdentity(transport: socket.socketProtocol == IPPROTO_TCP ? .tcp:.udp, executable: "", destination: remote.hostname, port: port,
                                    direction: flow.direction == .inbound ? .inbound : .outbound, identity: signingIdentity,localAddress:localAddress,localPort:localPort,interfaceName:interfaceName,flowID:flow.identifier.uuidString)
        captured.temporaryGrants.record(identity)
        if identity.transport == .udp {
            // Initial-flow DNS protection only. No UDP rule/ask/socket-wide exception is inherited.
            if SystemProtection.preset(for:identity)=="systemUDPDNS" {return .allow()}
            return .allow()
        }
        if captured.temporaryGrants.allows(identity){return .allow()}
        lock.lock()
        guard scope === captured, let rule = policy.matchingRule(for: identity) else { lock.unlock(); return .allow() }
        if rule.action == .ask {
            let request = captured.asks.enqueue(rule: rule, flow: identity) { [weak self] action in
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
