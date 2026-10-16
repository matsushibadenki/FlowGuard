import XCTest
import Foundation
import FlowGuardBridge
import FlowGuardPolicy
/// Anonymous, test-only endpoint; never registers the production Mach service or a provider.
private final class Endpoint: NSObject, NSXPCListenerDelegate, FilterBridgeProtocol {
    let asks = AskQueue()
    lazy var receiver = PolicyReceiver(instanceID: asks.instanceId)
    var connection: NSXPCConnection?
    func listener(_ listener: NSXPCListener, shouldAcceptNewConnection connection: NSXPCConnection) -> Bool {
        self.connection = connection
        connection.exportedInterface = NSXPCInterface(with: FilterBridgeProtocol.self)
        connection.exportedObject = self; connection.resume(); return true
    }
    func pendingAsks(withReply reply: @escaping (Data) -> Void) { reply(try! JSONEncoder().encode(asks.snapshot())) }
    func answerAsk(_ data: Data, withReply reply: @escaping (Data) -> Void) { reply(try! JSONEncoder().encode(asks.answer(try! JSONDecoder().decode(AskAnswer.self, from: data)))) }
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
    func testAskMetadataAndOneFlowAnswerCrossTheProductionXPCInterface() throws {
        let endpoint = Endpoint(); endpoint.receiver.setReady(true); endpoint.asks.activate()
        let data = Data(#"{"id":"ask","appKey":"signed:v1:developer:ABCDE12345:com.test.app","identity":{"authority":"developer","teamId":"ABCDE12345","identifier":"com.test.app"},"app":"Test","destination":"1.2.3.4","port":443,"direction":"both","action":"ask","timeoutAction":"block","updatedAt":1,"source":"live"}"#.utf8)
        let rule = try JSONDecoder().decode(PolicyRule.self, from: data)
        let flow = FlowIdentity(transport: .tcp, executable: "", destination: rule.destination, port: rule.port, direction: .outbound, identity: rule.identity)
        let resumed = expectation(description: "flow callback")
        XCTAssertNotNil(endpoint.asks.enqueue(rule: rule, flow: flow) { action in XCTAssertEqual(action, .allow); resumed.fulfill() })
        let listener = NSXPCListener.anonymous(); listener.delegate = endpoint; listener.resume()
        let connection = NSXPCConnection(listenerEndpoint: listener.endpoint)
        connection.remoteObjectInterface = NSXPCInterface(with: FilterBridgeProtocol.self); connection.resume()
        defer { connection.invalidate(); endpoint.connection?.invalidate(); listener.invalidate() }
        let completed = expectation(description: "answer receipt")
        let remote = try XCTUnwrap(connection.remoteObjectProxyWithErrorHandler { error in XCTFail("\(error)"); completed.fulfill() } as? FilterBridgeProtocol)
        remote.pendingAsks { data in
            do {
                let snapshot = try JSONDecoder().decode(AskSnapshot.self, from: data)
                let request = try XCTUnwrap(snapshot.requests.first)
                XCTAssertEqual(request.timeoutAction, .block)
                let answer = AskAnswer(instanceId: request.instanceId, policyId: request.policyId, requestId: request.requestId, action: .allow)
                remote.answerAsk(try JSONEncoder().encode(answer)) { data in
                    defer { completed.fulfill() }
                    do {
                        let reply = try JSONDecoder().decode(AskReply.self, from: data)
                        XCTAssertEqual(reply.status, "answered"); XCTAssertEqual(reply.requestId, request.requestId)
                    } catch { XCTFail("\(error)") }
                }
            } catch { XCTFail("\(error)"); completed.fulfill() }
        }
        wait(for: [completed, resumed], timeout: 3)
    }

}
