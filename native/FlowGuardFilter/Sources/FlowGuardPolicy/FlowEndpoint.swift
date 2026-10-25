import Foundation
import Darwin
public enum FlowEndpoint {
    public static func normalized(_ value:String)->String? {
        let raw=String(value.split(separator:"%",maxSplits:1).first ?? "")
        var buffer=[CChar](repeating:0,count:Int(INET6_ADDRSTRLEN));var a4=in_addr();var a6=in6_addr()
        if inet_pton(AF_INET,raw,&a4)==1 {guard inet_ntop(AF_INET,&a4,&buffer,socklen_t(buffer.count)) != nil else{return nil};return String(cString:buffer)}
        if inet_pton(AF_INET6,raw,&a6)==1 {let mapped:String?=withUnsafeBytes(of:a6){b in if b.prefix(10).allSatisfy({$0==0})&&b[10]==255&&b[11]==255{return b.suffix(4).map{String($0)}.joined(separator:".")};return nil};if let mapped{return mapped};guard inet_ntop(AF_INET6,&a6,&buffer,socklen_t(buffer.count)) != nil else{return nil};return String(cString:buffer)}
        return nil
    }
    public static func unicast(_ value:String)->Bool {
        guard let normalized=normalized(value)else{return false};var a4=in_addr();var a6=in6_addr()
        if inet_pton(AF_INET,normalized,&a4)==1 {let first=withUnsafeBytes(of:a4){$0[0]};return first>0&&first<224}
        if inet_pton(AF_INET6,normalized,&a6)==1 {return withUnsafeBytes(of:a6){b in if b.allSatisfy({$0==0})||b[0]==255{return false};if b.prefix(10).allSatisfy({$0==0})&&b[10]==255&&b[11]==255{return b[12]>0&&b[12]<224};return true}}
        return false
    }
    public static func sourceInterface(_ address:String)->String? {
        guard let expected=normalized(address)else{return nil};var first:UnsafeMutablePointer<ifaddrs>?
        guard getifaddrs(&first)==0,let start=first else{return nil};defer{freeifaddrs(first)}
        var item:UnsafeMutablePointer<ifaddrs>?=start;var names=Set<String>()
        while let current=item {defer{item=current.pointee.ifa_next};guard let addr=current.pointee.ifa_addr,[AF_INET,AF_INET6].contains(Int32(addr.pointee.sa_family)),current.pointee.ifa_flags & UInt32(IFF_UP) != 0 else{continue}
            var host=[CChar](repeating:0,count:Int(NI_MAXHOST));if getnameinfo(addr,socklen_t(addr.pointee.sa_len),&host,socklen_t(host.count),nil,0,NI_NUMERICHOST)==0,normalized(String(cString:host))==expected {names.insert(String(cString:current.pointee.ifa_name))}
        }
        return names.count==1 ? names.first:nil
    }
}
