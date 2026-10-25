import Foundation
import FlowGuardPolicy
public struct GrantQuery:Codable {public let version:Int;public let requestID:String;public let instanceID:String;public let expectedRevision:UInt64?;public let grants:[TemporaryGrant]?}
public struct ObservedGrantFlow:Codable {
    public let flowID:String;public let identity:SigningIdentity;public let direction:String;public let localAddress:String;public let localPort:UInt16;public let remoteAddress:String;public let remotePort:UInt16;public let interfaceName:String;public let observedAt:UInt64
}
public struct GrantReply:Codable {public let version:Int;public let requestID:String;public let instanceID:String;public let revision:UInt64;public let accepted:Bool;public let grantIDs:[String];public let flows:[ObservedGrantFlow];public let leaseDeadline:UInt64;public let error:String?}
public final class TemporaryGrantStore {
    private var closed=false
    private let lock=NSLock();private var revision:UInt64=0;private var grants:[TemporaryGrant]=[];private var heartbeat:UInt64=0
    private var flows:[ObservedGrantFlow]=[]
    private let interfaceResolver:(String)->String?
    public init(interfaceResolver:@escaping(String)->String?=FlowEndpoint.sourceInterface){self.interfaceResolver=interfaceResolver}
    public func record(_ flow:FlowIdentity){
        guard flow.transport == .tcp,let id=flow.flowID,let identity=flow.identity,identity.valid,let local=flow.localAddress,let port=flow.localPort,let remote=FlowEndpoint.normalized(flow.destination)else{return}
        lock.lock();defer{lock.unlock()};guard !closed else{return};flows.removeAll{$0.flowID==id};flows.append(ObservedGrantFlow(flowID:id,identity:identity,direction:flow.direction.rawValue,localAddress:local,localPort:port,remoteAddress:remote,remotePort:flow.port,interfaceName:"",observedAt:UInt64(Date().timeIntervalSince1970)));if flows.count>256{flows.removeFirst(flows.count-256)}
    }

    public func transact(_ q:GrantQuery,instance:String,now:UInt64=FlowContinuousClock.now,second:UInt64=FlowContinuousClock.second)->GrantReply {
        lock.lock();defer{lock.unlock()}
        if heartbeat>0,now>=heartbeat,!grants.isEmpty {
            grants=[];heartbeat=0;if revision<UInt64.max{revision+=1}
        }
        var error:String?=nil
        if closed {error="provider_stopped"}
        else if q.version != 1||UUID(uuidString:q.requestID)==nil||q.instanceID != instance {error="invalid_grant_request"}
        else if let proposed=q.grants {
            if q.expectedRevision != revision {error="grant_revision_conflict"}
            else if proposed.count>16||Set(proposed.map(\.id)).count != proposed.count || !proposed.allSatisfy({$0.valid&&$0.deadline>now&&$0.deadline-now<=600*second&&matchesRecorded($0)}){error="invalid_grant"}
            else if revision==UInt64.max {error="grant_revision_conflict"}
            else{grants=proposed;revision+=1;heartbeat=now+5*second}
        }
        if now>=heartbeat {grants=[]}
        grants.removeAll{$0.deadline<=now}
        let wall=UInt64(Date().timeIntervalSince1970);flows.removeAll{f in f.observedAt>wall||(wall-f.observedAt>60 && !grants.contains(where:{$0.flowID==f.flowID}))}
        let observations=q.grants == nil ? flows.suffix(128).compactMap{f->ObservedGrantFlow? in guard let interface=interfaceResolver(f.localAddress)else{return nil};return ObservedGrantFlow(flowID:f.flowID,identity:f.identity,direction:f.direction,localAddress:f.localAddress,localPort:f.localPort,remoteAddress:f.remoteAddress,remotePort:f.remotePort,interfaceName:interface,observedAt:f.observedAt)}:[]
        return GrantReply(version:1,requestID:q.requestID,instanceID:instance,revision:revision,accepted:error==nil,grantIDs:grants.map(\.id),flows:observations,leaseDeadline:heartbeat,error:error)
    }
    private func matchesRecorded(_ grant:TemporaryGrant)->Bool {flows.contains{f in f.flowID==grant.flowID&&f.identity==grant.identity&&f.direction==grant.direction&&f.localAddress==grant.localAddress&&f.localPort==grant.localPort&&f.remoteAddress==grant.remoteAddress&&f.remotePort==grant.remotePort&&interfaceResolver(f.localAddress)==grant.interfaceName}}
    public func allows(_ flow:FlowIdentity,now:UInt64=FlowContinuousClock.now)->Bool {lock.lock();defer{lock.unlock()};guard !closed,now<heartbeat else{if !grants.isEmpty {grants=[];heartbeat=0;if revision<UInt64.max{revision+=1}};return false};grants.removeAll{$0.deadline<=now};return grants.contains{$0.matches(flow)}}
    public var active:Bool {lock.lock();defer{lock.unlock()};return !closed && !grants.isEmpty&&FlowContinuousClock.now<heartbeat}
    public func stop(){lock.lock();closed=true;grants=[];flows=[];heartbeat=0;if revision<UInt64.max{revision+=1};lock.unlock()}
}
