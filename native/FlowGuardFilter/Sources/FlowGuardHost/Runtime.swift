import Foundation
import FlowGuardBridge

public typealias LifecycleCallback = @convention(c) (UnsafePointer<CChar>?, UInt64) -> Void
private let controller = ExtensionController()

/// Called inside the containing Rust process. Never grants a signing exception.
@_cdecl("flowguard_host_can_activate")
public func canActivate() -> Int32 {
    BridgeSigning.teamIdentifier() != nil && Bundle.main.bundleIdentifier == "com.flowguard.desktop" ? 1 : 0
}
@_cdecl("flowguard_host_activate")
public func activate(_ callback: @escaping LifecycleCallback, _ token: UInt64) {
    // The unsigned path completes synchronously and never enters SystemExtensions/NEFilterManager.
    guard canActivate() == 1 else { "signature_required".withCString { callback($0, token) }; return }
    DispatchQueue.main.async {
        controller.activate { status, _ in status.withCString { callback($0, token) } }
    }
}
@_cdecl("flowguard_host_abi_version")
public func abiVersion() -> Int32 { 1 }
