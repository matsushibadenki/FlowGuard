import Foundation
import Darwin
public enum FlowContinuousClock {
    public static var now:UInt64{mach_continuous_time()}
    public static var second:UInt64{var info=mach_timebase_info_data_t();mach_timebase_info(&info);return 1_000_000_000*UInt64(info.denom)/UInt64(info.numer)}
}
public struct TemporaryGrant:Codable,Equatable {
    public let flowID:String
    public let id:String
    public let identity:SigningIdentity
    public let transport:String
    public let direction:String
    public let localAddress:String
    public let localPort:UInt16
    public let remoteAddress:String
    public let remotePort:UInt16
    public let interfaceName:String
    public let networkScope:String
    public let peerEvidence:String
    public let deadline:UInt64
    private var validPeer: Bool {
        if peerEvidence.hasPrefix("ssh:SHA256:") {
            let value=peerEvidence.dropFirst(11)
            return value.count==43 && value.unicodeScalars.allSatisfy { $0.isASCII && (CharacterSet.alphanumerics.contains($0) || $0=="+" || $0=="/") }
        }
        guard peerEvidence.hasPrefix("tls:") else { return false }
        let value=String(peerEvidence.dropFirst(4))
        guard !value.isEmpty,value.utf8.count<=253,FlowEndpoint.normalized(value)==nil,value.unicodeScalars.allSatisfy({$0.isASCII}) else{return false}
        return value.split(separator:".",omittingEmptySubsequences:false).allSatisfy { label in
            guard !label.isEmpty,label.count<=63,let first=label.first,let last=label.last,first.isLetter||first.isNumber,last.isLetter||last.isNumber else{return false}
            return label.allSatisfy{$0.isLetter||$0.isNumber||$0=="-"}
        }
    }
    public var valid:Bool {
        UUID(uuidString:id) != nil && UUID(uuidString:flowID) != nil && identity.valid && transport=="TCP" &&
        ["outbound","inbound"].contains(direction) && localPort>0 && remotePort>0 &&
        FlowEndpoint.normalized(localAddress)==localAddress && FlowEndpoint.normalized(remoteAddress)==remoteAddress &&
        !interfaceName.isEmpty && interfaceName.utf8.count<=32 && interfaceName.unicodeScalars.allSatisfy{$0.isASCII && (CharacterSet.alphanumerics.contains($0) || "_.-".unicodeScalars.contains($0))} &&
        networkScope.utf8.count==64 && networkScope.allSatisfy{$0.isHexDigit} && validPeer
    }
    public func matches(_ flow:FlowIdentity)->Bool {flow.flowID==flowID && identity==flow.identity && flow.transport == .tcp&&flow.direction.rawValue==direction&&FlowEndpoint.normalized(flow.destination)==remoteAddress&&flow.port==remotePort&&flow.localAddress==localAddress&&flow.localPort==localPort&&flow.interfaceName==interfaceName}
}
