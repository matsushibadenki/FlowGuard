import Foundation
import FlowGuardPolicy
public struct FlowNameQuery: Codable {
    public let version: Int
    public let requestID: String
    public let instanceID: String
    public let enabled: Bool
}
public struct FlowNameRecord: Codable {
    public let localAddress: String
    public let localPort: UInt16
    public let remoteAddress: String
    public let remotePort: UInt16
    public let transport: String
    public let direction: String
    public let identity: SigningIdentity
    public let name: String
    public let observedAt: UInt64
    public init(localAddress:String,localPort:UInt16,remoteAddress:String,remotePort:UInt16,transport:String,direction:String,identity:SigningIdentity,name:String,observedAt:UInt64){self.localAddress=localAddress;self.localPort=localPort;self.remoteAddress=remoteAddress;self.remotePort=remotePort;self.transport=transport;self.direction=direction;self.identity=identity;self.name=name;self.observedAt=observedAt}
}
public struct FlowNameReply: Codable {public let version:Int;public let requestID:String;public let instanceID:String;public let rows:[FlowNameRecord]}
/// A short capture lease, independent of policy and ask. No DNS request is made.
public final class FlowNameStore {
    private let lock=NSLock()
    private var until:TimeInterval=0
    private var rows:[FlowNameRecord]=[]
    public init(){}
    public func expire(uptime:TimeInterval=ProcessInfo.processInfo.systemUptime){lock.lock();if uptime>until{until=0;rows=[]};lock.unlock()}
    public func stop(){lock.lock();until=0;rows=[];lock.unlock()}
    public func query(_ request:FlowNameQuery,instance:String,uptime:TimeInterval=ProcessInfo.processInfo.systemUptime,now:UInt64=UInt64(Date().timeIntervalSince1970))->FlowNameReply?{
        guard request.version==1,UUID(uuidString:request.requestID) != nil,request.instanceID==instance else{return nil}
        lock.lock();defer{lock.unlock()}
        if !request.enabled {until=0;rows=[]}else{if uptime>until{rows=[]};until=uptime+15;rows.removeAll{$0.observedAt>now||now-$0.observedAt>15}}
        return FlowNameReply(version:1,requestID:request.requestID,instanceID:instance,rows:rows)
    }
    public func record(_ row:FlowNameRecord,uptime:TimeInterval=ProcessInfo.processInfo.systemUptime){
        lock.lock();defer{lock.unlock()}
        guard uptime<=until,until>0,row.identity.valid,row.transport=="TCP",row.localPort>0,row.remotePort>0,["inbound","outbound"].contains(row.direction),Self.validName(row.name),row.localAddress.utf8.count<=64,row.remoteAddress.utf8.count<=64 else{return}
        rows.removeAll{$0.localAddress==row.localAddress&&$0.localPort==row.localPort&&$0.remoteAddress==row.remoteAddress&&$0.remotePort==row.remotePort&&$0.identity==row.identity&&$0.name==row.name}
        rows.append(row);if rows.count>128{rows.removeFirst(rows.count-128)}
    }
    public static func validName(_ value:String)->Bool{
        guard !value.isEmpty,value.utf8.count<=253,value.unicodeScalars.allSatisfy({$0.isASCII})else{return false}
        return value.split(separator:".",omittingEmptySubsequences:false).allSatisfy{p in
            guard !p.isEmpty,p.count<=63,let first=p.first,let last=p.last,first.isLetter||first.isNumber,last.isLetter||last.isNumber else{return false}
            return p.allSatisfy{$0.isLetter||$0.isNumber||$0=="-"}
        }
    }
}
