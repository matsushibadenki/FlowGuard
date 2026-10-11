import XCTest
import Foundation
import FlowGuardBridge
/// Anonymous, test-only endpoint; never registers the production Mach service or a provider.
private final class Endpoint: NSObject, NSXPCListenerDelegate, FilterBridgeProtocol {
    let receiver = PolicyReceiver()
    var connection: NSXPCConnection?
    func listener(_ listener: NSXPCListener, shouldAcceptNewConnection connection: NSXPCConnection) -> Bool {
        self.connection = connection
        connection.exportedInterface = NSXPCInterface(with: FilterBridgeProtocol.self)
        connection.exportedObject = self; connection.resume(); return true
    }
    func status(withReply reply: @escaping (Data) -> Void) { reply(try! JSONEncoder().encode(receiver.status())) }
    func replacePolicy(_ data: Data, withReply reply: @escaping (Data) -> Void) { reply(try! JSONEncoder().encode(receiver.replace(data) { _ in })) }
}
final class TransportTests: XCTestCase {
    func testAnonymousXPCRoundtripUsesProductionProtocol() throws {
        let endpoint = Endpoint(); endpoint.receiver.setReady(true)
        let listener = NSXPCListener.anonymous(); listener.delegate = endpoint; listener.resume()
        let connection = NSXPCConnection(listenerEndpoint: listener.endpoint)
        connection.remoteObjectInterface = NSXPCInterface(with: FilterBridgeProtocol.self); connection.resume()
        defer { connection.invalidate(); endpoint.connection?.invalidate(); listener.invalidate() }
        let finished = expectation(description: "XPC policy acknowledgement")
        let remote = try XCTUnwrap(connection.remoteObjectProxyWithErrorHandler { error in XCTFail("\(error)"); finished.fulfill() } as? FilterBridgeProtocol)
        remote.status { data in
            do {
                let status = try JSONDecoder().decode(BridgeReply.self, from: data)
                let update = PolicyUpdate(requestID: UUID().uuidString, expectedInstanceID: status.instanceID, expectedRevision: status.revision, rules: [])
                remote.replacePolicy(try JSONEncoder().encode(update)) { data in
                    defer { finished.fulfill() }
                    do { let reply = try JSONDecoder().decode(BridgeReply.self, from: data); XCTAssertTrue(BridgeContract.validAcknowledgement(reply, for: update)) }
                    catch { XCTFail("\(error)") }
                }
            } catch { XCTFail("\(error)"); finished.fulfill() }
        }
        wait(for: [finished], timeout: 3)
    }
}
