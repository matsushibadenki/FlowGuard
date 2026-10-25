import Foundation
import FlowGuardBridge

public typealias LifecycleCallback = @convention(c) (UnsafePointer<CChar>?, UInt64) -> Void
private let controller = ExtensionController()
private let submissions = RecoveryBarrier()

/// Called inside the containing Rust process. Never grants a signing exception.
@_cdecl("flowguard_host_can_activate")
public func canActivate() -> Int32 {
    BridgeSigning.teamIdentifier() != nil && Bundle.main.bundleIdentifier == "com.flowguard.desktop" ? 1 : 0
}
@_cdecl("flowguard_host_activate")
public func activate(_ bytes: UnsafePointer<UInt8>?, _ length: UInt64, _ generation: UInt64, _ callback: @escaping LifecycleCallback, _ token: UInt64) {
    // The unsigned path completes synchronously and never enters SystemExtensions/NEFilterManager.
    guard canActivate() == 1 else { "signature_required".withCString { callback($0, token) }; return }
    guard let bytes, length <= 4 * 1024 * 1024 else { "invalid_initial_policy".withCString { callback($0, token) }; return }
    let policy = Data(bytes: bytes, count: Int(length))
    guard (try? InitialPolicy.decode(policy)) != nil else { "invalid_initial_policy".withCString { callback($0, token) }; return }
    DispatchQueue.main.async {
        guard submissions.admit(generation) else { "cancelled".withCString { callback($0, token) }; return }
        controller.activate(policy: policy) { status, _ in status.withCString { callback($0, token) } }
    }
}
@_cdecl("flowguard_host_abi_version")
public func abiVersion() -> Int32 { 5 }

@_cdecl("flowguard_host_disable")
public func disable(_ remove: Int32, _ generation: UInt64, _ callback: @escaping LifecycleCallback, _ token: UInt64) {
    guard canActivate() == 1 else { "signature_required".withCString { callback($0, token) }; return }
    DispatchQueue.main.async {
        guard submissions.admit(generation) else { "cancelled".withCString { callback($0, token) }; return }
        controller.disable(remove: remove != 0) { status, _ in status.withCString { callback($0, token) } } }
}

@_cdecl("flowguard_host_persist_policy")
public func persistPolicy(_ bytes: UnsafePointer<UInt8>?, _ length: UInt64, _ generation: UInt64, _ callback: @escaping LifecycleCallback, _ token: UInt64) {
    guard canActivate() == 1 else { "signature_required".withCString { callback($0, token) }; return }
    guard let bytes, length <= 4 * 1024 * 1024 else { "invalid_initial_policy".withCString { callback($0, token) }; return }
    let policy = Data(bytes: bytes, count: Int(length))
    guard (try? InitialPolicy.decode(policy)) != nil else { "invalid_initial_policy".withCString { callback($0, token) }; return }
    DispatchQueue.main.async {
        guard submissions.admit(generation) else { "cancelled".withCString { callback($0, token) }; return }
        controller.persist(policy: policy) { status, _ in status.withCString { callback($0, token) } } }
}
