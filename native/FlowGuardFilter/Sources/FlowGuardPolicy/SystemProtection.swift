import Foundation
public enum SystemProtection {
    /// The caller supplies a Security.framework-verified subject and actual flow direction.
    public static func preset(for flow: FlowIdentity) -> String? {
        guard let identity = flow.identity, identity.valid, identity.authority == "apple", flow.direction == .outbound else { return nil }
        if flow.transport == .udp {
            guard identity.identifier=="com.apple.mDNSResponder",let ip=FlowEndpoint.normalized(flow.destination)else{return nil}
            if flow.port==53 && FlowEndpoint.unicast(ip){return "systemUDPDNS"}
            if flow.port==5353 && ["224.0.0.251","ff02::fb"].contains(ip) && flow.interfaceName?.isEmpty==false{return "systemMDNS"}
            return nil
        }
        guard flow.transport == .tcp else{return nil}
        switch (identity.identifier, flow.port) {
        case ("com.apple.mDNSResponder", 53): return "systemDNS"
        case ("com.apple.softwareupdated", 443): return "systemUpdates"
        default: return nil
        }
    }
}
