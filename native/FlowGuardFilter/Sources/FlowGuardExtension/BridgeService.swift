import Foundation
import FlowGuardBridge
import FlowGuardPolicy
@available(macOS 13.0, *)
final class BridgeService: NSObject, NSXPCListenerDelegate, FilterBridgeProtocol {
    private let receiver: PolicyReceiver
    private let asks: AskQueue
    private let apply: ([PolicyRule]) throws -> Void
    private var listener: NSXPCListener?
    private var clients: [NSXPCConnection] = []
    private let lock = NSLock()
    init(asks: AskQueue, apply: @escaping ([PolicyRule]) throws -> Void) { self.asks = asks; receiver = PolicyReceiver(instanceID: asks.instanceId); self.apply = apply }
    func start(initial: [PolicyRule]) throws {
        guard let team = BridgeSigning.teamIdentifier(),
              let configuration = Bundle.main.object(forInfoDictionaryKey: "NetworkExtension") as? [String: Any],
              let service = configuration["NEMachServiceName"] as? String,
              service == "\(team).com.flowguard.filter" else { throw NSError(domain: "FlowGuardBridge", code: 1) }
        let listener = NSXPCListener(machServiceName: service)
        listener.setConnectionCodeSigningRequirement(BridgeSigning.requirement(identifier: "com.flowguard.bridge", team: team))
        listener.delegate = self; self.listener = listener
        receiver.seed(initial); asks.activate(); receiver.setReady(true); listener.resume()
    }
    func stop() {
        receiver.setReady(false); asks.shutdown(); listener?.invalidate(); listener = nil
        lock.lock(); let connections = clients; clients.removeAll(); lock.unlock()
        connections.forEach { $0.invalidate() }
    }
    func listener(_ listener: NSXPCListener, shouldAcceptNewConnection connection: NSXPCConnection) -> Bool {
        lock.lock(); defer { lock.unlock() }
        guard clients.count < 8 else { return false }
        connection.exportedInterface = NSXPCInterface(with: FilterBridgeProtocol.self)
        connection.exportedObject = self
        connection.invalidationHandler = { [weak self, weak connection] in
            guard let self, let connection else { return }
            self.lock.lock(); self.clients.removeAll { $0 === connection }; self.lock.unlock()
        }
        clients.append(connection); connection.resume(); return true
    }
    func status(withReply reply: @escaping (Data) -> Void) { reply(encode(receiver.status())) }
    func replacePolicy(_ data: Data, withReply reply: @escaping (Data) -> Void) { reply(encode(receiver.replace(data, apply: apply))) }
    func pendingAsks(withReply reply: @escaping (Data) -> Void) { reply((try? JSONEncoder().encode(asks.snapshot())) ?? Data()) }
    func answerAsk(_ data: Data, withReply reply: @escaping (Data) -> Void) {
        guard data.count <= 4096, let answer = try? JSONDecoder().decode(AskAnswer.self, from: data) else { reply(Data()); return }
        reply((try? JSONEncoder().encode(asks.answer(answer))) ?? Data())
    }
    private func encode(_ value: BridgeReply) -> Data { (try? JSONEncoder().encode(value)) ?? Data() }
}
