import XCTest
import FlowGuardBridge
import FlowGuardPolicy
final class FlowNameTests:XCTestCase {
 func testConsentCaptureLeaseAndIdentityBounds() throws {
  let store=FlowNameStore(),id=UUID().uuidString
  let json="{\"version\":1,\"requestID\":\"\(UUID().uuidString)\",\"instanceID\":\"\(id)\",\"enabled\":true}"
  let query=try JSONDecoder().decode(FlowNameQuery.self,from:Data(json.utf8))
  let record=FlowNameRecord(localAddress:"192.0.2.2",localPort:50000,remoteAddress:"192.0.2.1",remotePort:443,transport:"TCP",direction:"outbound",identity:SigningIdentity(authority:"apple",teamId:"",identifier:"com.apple.test"),name:"example.test",observedAt:100)
  store.record(record,uptime:1);XCTAssertEqual(store.query(query,instance:id,uptime:1,now:100)?.rows.count,0)
  store.record(record,uptime:2);XCTAssertEqual(store.query(query,instance:id,uptime:2,now:100)?.rows.count,1)
  store.record(record,uptime:20);XCTAssertEqual(store.query(query,instance:id,uptime:20,now:120)?.rows.count,0)
  XCTAssertNil(store.query(query,instance:UUID().uuidString))
  store.stop(); XCTAssertNil(store.query(query,instance:id,uptime:21,now:120))
  for invalid in ["<script>","-bad.example","bad-.example","bad..example","bad name","127.0.0.1\n"]{XCTAssertFalse(FlowNameStore.validName(invalid))}
 }
}
