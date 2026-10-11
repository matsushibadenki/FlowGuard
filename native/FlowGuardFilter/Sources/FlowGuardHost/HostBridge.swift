import Foundation
import FlowGuardBridge
import FlowGuardPolicy
public struct BridgePreflight: Encodable {
    public let protocolVersion = 1
    public let packaged: Bool
    public let signed: Bool
    public let status: String
    public let serviceName: String?
}
public final class HostBridge {
    public init() {}
    public func preflight(extensionURL: URL) -> BridgePreflight {
        let packaged = FileManager.default.fileExists(atPath: extensionURL.appendingPathComponent("Contents/MacOS/FlowGuardSystemExtension").path)
        let team = BridgeSigning.teamIdentifier()
        return BridgePreflight(packaged: packaged, signed: team != nil,
            status: !packaged ? "not_packaged" : team == nil ? "signature_required" : "requires_activation",
            serviceName: team.map { "\($0).com.flowguard.filter" })
    }
    public func sync(rules: [PolicyRule], completion: @escaping (Result<BridgeReply, Error>) -> Void) {
        transact(update: nil) { [self] result in
            do {
                let status = try JSONDecoder().decode(BridgeReply.self, from: result.get())
                guard status.protocolVersion == 1, status.status == "ready", status.error == nil else { throw NSError(domain: "FlowGuardBridge.not_ready", code: 6) }
                let update = PolicyUpdate(requestID: UUID().uuidString, expectedInstanceID: status.instanceID, expectedRevision: status.revision, rules: rules)
                let payload = try JSONEncoder().encode(update)
                transact(update: payload) { result in
                    do {
                        let reply = try JSONDecoder().decode(BridgeReply.self, from: result.get())
                        guard BridgeContract.validAcknowledgement(reply, for: update) else { throw NSError(domain: "FlowGuardBridge.rejected_or_mismatched_ack", code: 7) }
                        completion(.success(reply))
                    } catch { completion(.failure(error)) }
                }
            } catch { completion(.failure(error)) }
        }
    }
    /// Each transaction has a bounded lifetime. Reconnect starts with status, never a cached revision.
    public func transact(update: Data?, completion: @escaping (Result<Data, Error>) -> Void) {
        guard let team = BridgeSigning.teamIdentifier() else {
            completion(.failure(NSError(domain: "FlowGuardBridge.signature_required", code: 1))); return
        }
        let connection = NSXPCConnection(machServiceName: "\(team).com.flowguard.filter", options: [])
        connection.remoteObjectInterface = NSXPCInterface(with: FilterBridgeProtocol.self)
        connection.setCodeSigningRequirement(BridgeSigning.requirement(identifier: "com.flowguard.filter", team: team))
        let finish = CompletionGate(connection: connection, completion: completion)
        connection.interruptionHandler = { finish.end(.failure(NSError(domain: "FlowGuardBridge.interrupted", code: 2))) }
        connection.invalidationHandler = { finish.end(.failure(NSError(domain: "FlowGuardBridge.disconnected", code: 3))) }
        connection.resume()
        DispatchQueue.global().asyncAfter(deadline: .now() + 5) { finish.end(.failure(NSError(domain: "FlowGuardBridge.timeout", code: 4))) }
        guard let remote = connection.remoteObjectProxyWithErrorHandler({ finish.end(.failure($0)) }) as? FilterBridgeProtocol else {
            finish.end(.failure(NSError(domain: "FlowGuardBridge.invalid_proxy", code: 5))); return
        }
        let reply: (Data) -> Void = { finish.end(.success($0)) }
        if let update { remote.replacePolicy(update, withReply: reply) } else { remote.status(withReply: reply) }
    }
}
private final class CompletionGate {
    private let lock = NSLock()
    private var completed = false
    private let connection: NSXPCConnection
    private let completion: (Result<Data, Error>) -> Void
    init(connection: NSXPCConnection, completion: @escaping (Result<Data, Error>) -> Void) { self.connection = connection; self.completion = completion }
    func end(_ result: Result<Data, Error>) {
        lock.lock(); guard !completed else { lock.unlock(); return }; completed = true; lock.unlock()
        connection.invalidationHandler = nil; connection.interruptionHandler = nil; connection.invalidate(); completion(result)
    }
}
