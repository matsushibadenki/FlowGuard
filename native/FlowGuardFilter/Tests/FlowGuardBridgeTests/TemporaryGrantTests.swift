import XCTest
import FlowGuardBridge
import FlowGuardPolicy
final class TemporaryGrantTests:XCTestCase {
 func query(_ instance:String,_ revision:UInt64?,_ grants:[TemporaryGrant]?)->GrantQuery {let dict:[String:Any] = ["version":1,"requestID":UUID().uuidString,"instanceID":instance,"expectedRevision":revision as Any? ?? NSNull(),"grants":grants.flatMap{try? JSONSerialization.jsonObject(with:JSONEncoder().encode($0))} ?? NSNull()];return try! JSONDecoder().decode(GrantQuery.self,from:JSONSerialization.data(withJSONObject:dict))}
 func testExactFlowUUIDDeadlineHeartbeatAndStaleRevocation()throws {
  let store=TemporaryGrantStore(interfaceResolver:{_ in "test0"}),instance=UUID().uuidString,flowID=UUID().uuidString,identity=SigningIdentity(authority:"apple",teamId:"",identifier:"com.apple.test")
  let flow=FlowIdentity(transport:.tcp,executable:"",destination:"192.0.2.1",port:443,direction:.outbound,identity:identity,localAddress:"192.0.2.2",localPort:50000,interfaceName:"test0",flowID:flowID)
  store.record(flow)
  let dict:[String:Any] = ["id":UUID().uuidString,"flowID":flowID,"identity":["authority":"apple","teamId":"","identifier":"com.apple.test"],"transport":"TCP","direction":"outbound","localAddress":"192.0.2.2","localPort":50000,"remoteAddress":"192.0.2.1","remotePort":443,"interfaceName":"test0","networkScope":String(repeating:"a",count:64),"peerEvidence":"tls:example.test","deadline":110]
  let grant=try JSONDecoder().decode(TemporaryGrant.self,from:JSONSerialization.data(withJSONObject:dict))
  for peer in ["tls:","tls:127.0.0.1","tls:<script>","ssh:SHA256:short"] {
   var invalid=dict;invalid["peerEvidence"]=peer
   XCTAssertFalse(try JSONDecoder().decode(TemporaryGrant.self,from:JSONSerialization.data(withJSONObject:invalid)).valid)
  }
  XCTAssertTrue(store.transact(query(instance,0,[grant]),instance:instance,now:100,second:1).accepted)
  XCTAssertTrue(store.allows(flow,now:104))
  // A paused exact flow can consume an acknowledged intention; another UUID cannot.
  let queue=AskQueue(clock:{100}); queue.activate()
  let ruleData:[String:Any] = ["id":"ask","transport":"TCP","appKey":identity.key,"identity":["authority":"apple","teamId":"","identifier":"com.apple.test"],"app":"test","destination":"192.0.2.1","port":443,"direction":"both","action":"ask","timeoutAction":"block","updatedAt":1,"source":"live"]
  let rule=try JSONDecoder().decode(PolicyRule.self,from:JSONSerialization.data(withJSONObject:ruleData))
  var resumed=false
  XCTAssertNotNil(queue.enqueue(rule:rule,flow:flow){action in resumed = action == .allow})
  // The production clock is independent of this synthetic 100-tick lease: expired grants must not resume.
  queue.allowTemporary(using:store); XCTAssertFalse(resumed)

  XCTAssertFalse(store.allows(FlowIdentity(transport:.tcp,executable:"",destination:"192.0.2.1",port:443,direction:.outbound,identity:identity,localAddress:"192.0.2.2",localPort:50000,interfaceName:"test0",flowID:UUID().uuidString),now:104))
  XCTAssertFalse(store.allows(flow,now:105))
  XCTAssertFalse(store.transact(query(instance,1,[grant]),instance:instance,now:106,second:1).accepted)
  XCTAssertTrue(store.transact(query(instance,2,[grant]),instance:instance,now:106,second:1).accepted)
  XCTAssertFalse(store.allows(flow,now:110))
  XCTAssertTrue(store.transact(query(instance,3,[]),instance:instance,now:110,second:1).accepted)
  XCTAssertFalse(store.transact(query(instance,1,[grant]),instance:instance,now:109,second:1).accepted)
  XCTAssertFalse(store.allows(flow,now:109))
  store.stop(); store.record(flow)
  let stopped=store.transact(query(instance,nil,nil),instance:instance,now:109,second:1)
  XCTAssertFalse(stopped.accepted); XCTAssertTrue(stopped.flows.isEmpty)
  XCTAssertFalse(store.transact(query(instance,stopped.revision,[grant]),instance:instance,now:109,second:1).accepted)
 }
 func testAcknowledgedExactGrantResumesOnlyItsPausedAsk() throws {
  let store=TemporaryGrantStore(interfaceResolver:{_ in "test0"}),id=UUID().uuidString,flowID=UUID().uuidString
  let identity=SigningIdentity(authority:"apple",teamId:"",identifier:"com.apple.test")
  func flow(_ uuid:String)->FlowIdentity {FlowIdentity(transport:.tcp,executable:"",destination:"192.0.2.1",port:443,direction:.outbound,identity:identity,localAddress:"192.0.2.2",localPort:50000,interfaceName:"test0",flowID:uuid)}
  store.record(flow(flowID))
  let deadline=FlowContinuousClock.now+60*FlowContinuousClock.second
  let data:[String:Any] = ["id":UUID().uuidString,"flowID":flowID,"identity":["authority":"apple","teamId":"","identifier":"com.apple.test"],"transport":"TCP","direction":"outbound","localAddress":"192.0.2.2","localPort":50000,"remoteAddress":"192.0.2.1","remotePort":443,"interfaceName":"test0","networkScope":String(repeating:"a",count:64),"peerEvidence":"tls:example.test","deadline":deadline]
  let grant=try JSONDecoder().decode(TemporaryGrant.self,from:JSONSerialization.data(withJSONObject:data))
  XCTAssertTrue(store.transact(query(id,0,[grant]),instance:id).accepted)
  let ruleData:[String:Any] = ["id":"ask","transport":"TCP","appKey":identity.key,"identity":["authority":"apple","teamId":"","identifier":"com.apple.test"],"app":"test","destination":"192.0.2.1","port":443,"direction":"both","action":"ask","timeoutAction":"block","updatedAt":1,"source":"live"]
  let rule=try JSONDecoder().decode(PolicyRule.self,from:JSONSerialization.data(withJSONObject:ruleData))
  let queue=AskQueue();queue.activate();var allowed=0
  XCTAssertNotNil(queue.enqueue(rule:rule,flow:flow(flowID)){if $0 == .allow {allowed+=1}})
  XCTAssertNotNil(queue.enqueue(rule:rule,flow:flow(UUID().uuidString)){if $0 == .allow {allowed+=100}})
  queue.allowTemporary(using:store)
  XCTAssertEqual(allowed,1);XCTAssertEqual(queue.snapshot().requests.count,1)
  XCTAssertEqual(queue.snapshot().outcomes.last?.reason,"temporary_allow")
  store.stop();queue.allowTemporary(using:store);XCTAssertEqual(allowed,1)
 }
 func testUDPPresetsRequireVerifiedSubjectAndMetadata(){
  let apple=SigningIdentity(authority:"apple",teamId:"",identifier:"com.apple.mDNSResponder")
  func flow(_ address:String,_ port:UInt16,_ interface:String?=nil)->FlowIdentity{FlowIdentity(transport:.udp,executable:"",destination:address,port:port,direction:.outbound,identity:apple,interfaceName:interface)}
  XCTAssertEqual(SystemProtection.preset(for:flow("1.1.1.1",53)),"systemUDPDNS")
  for address in ["0.0.0.0","225.0.0.1","239.1.1.1","::","ff02::fb","::ffff:239.1.1.1","not-an-ip"]{XCTAssertNil(SystemProtection.preset(for:flow(address,53)))}
  XCTAssertNil(SystemProtection.preset(for:flow("224.0.0.251",5353)))
  XCTAssertEqual(SystemProtection.preset(for:flow("224.0.0.251",5353,"test0")),"systemMDNS")
 }
}
