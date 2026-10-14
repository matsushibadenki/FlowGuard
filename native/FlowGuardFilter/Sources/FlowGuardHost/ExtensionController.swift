import Foundation
import SystemExtensions
import NetworkExtension
import FlowGuardBridge
protocol FilterPreferences: AnyObject {
    var providerConfiguration: NEFilterProviderConfiguration? { get set }
    var localizedDescription: String? { get set }
    var isEnabled: Bool { get set }
    func loadFromPreferences(completionHandler: @escaping (Error?) -> Void)
    func saveToPreferences(completionHandler: @escaping (Error?) -> Void)
    func removeFromPreferences(completionHandler: @escaping (Error?) -> Void)
}
private final class OSFilterPreferences: FilterPreferences {
    let manager = NEFilterManager.shared()
    var providerConfiguration: NEFilterProviderConfiguration? { get { manager.providerConfiguration } set { manager.providerConfiguration = newValue } }
    var localizedDescription: String? { get { manager.localizedDescription } set { manager.localizedDescription = newValue } }
    var isEnabled: Bool { get { manager.isEnabled } set { manager.isEnabled = newValue } }
    func loadFromPreferences(completionHandler: @escaping (Error?) -> Void) { manager.loadFromPreferences { completionHandler($0) } }
    func saveToPreferences(completionHandler: @escaping (Error?) -> Void) { manager.saveToPreferences { completionHandler($0) } }
    func removeFromPreferences(completionHandler: @escaping (Error?) -> Void) { manager.removeFromPreferences { completionHandler($0) } }
}
/// All entries and OS completion continuations run on the main queue.
public final class ExtensionController: NSObject, OSSystemExtensionRequestDelegate {
    private var completion: ((String, Error?) -> Void)?
    private var emergency: ((String, Error?) -> Void)?
    private var initialPolicy: Data?
    private var activeRequest: OSSystemExtensionRequest?
    private var removing = false
    private var disabling = false
    private let barrier = RecoveryBarrier()
    private var authorize: () -> Bool = { BridgeSigning.teamIdentifier() != nil && Bundle.main.bundleIdentifier == "com.flowguard.desktop" }
    private var preferences: () -> FilterPreferences = { OSFilterPreferences() }
    private var submit: (OSSystemExtensionRequest) -> Void = { OSSystemExtensionManager.shared.submitRequest($0) }
    public override init() { super.init() }
    // Internal injection exercises the production coordinator without OS preferences or signing bypass in the public API.
    init(authorize: @escaping () -> Bool, preferences: @escaping () -> FilterPreferences, submit: @escaping (OSSystemExtensionRequest) -> Void) {
        self.authorize = authorize; self.preferences = preferences; self.submit = submit; super.init()
    }
    private var authorized: Bool { authorize() }
    private func finish(_ status: String, _ error: Error? = nil) {
        let callback = completion; completion = nil; initialPolicy = nil; activeRequest = nil; disabling = false
        callback?(status, error)
    }
    @discardableResult private func interrupt() -> Bool {
        guard let callback = emergency, barrier.takeRecovery() else { return false }
        emergency = nil
        finish("cancelled")
        startDisable(remove: false, completion: callback)
        return true
    }
    /// Never overlap OS preference operations, including after the Rust caller times out.
    private func io(_ operation: (@escaping (Error?) -> Void) -> Void, then continuation: @escaping (Error?) -> Void) {
        barrier.beginIO()
        operation { error in
            DispatchQueue.main.async {
                self.barrier.endIO()
                if self.interrupt() { return }
                continuation(error)
            }
        }
    }
    public func activate(policy: Data, completion: @escaping (String, Error?) -> Void) {
        guard authorized else { completion("signature_required", nil); return }
        guard self.completion == nil, emergency == nil, barrier.explicitActivation() else { completion("busy", nil); return }
        guard (try? InitialPolicy.decode(policy)) != nil else { completion("invalid_initial_policy", nil); return }
        initialPolicy = policy; removing = false; self.completion = completion
        let request = OSSystemExtensionRequest.activationRequest(forExtensionWithIdentifier: "com.flowguard.filter", queue: .main)
        activeRequest = request; request.delegate = self; submit(request)
    }
    public func requestNeedsUserApproval(_ request: OSSystemExtensionRequest) {
        guard request === activeRequest else { return }; completion?("approval_required", nil)
    }
    public func request(_ request: OSSystemExtensionRequest, actionForReplacingExtension existing: OSSystemExtensionProperties, withExtension ext: OSSystemExtensionProperties) -> OSSystemExtensionRequest.ReplacementAction {
        request === activeRequest ? .replace : .cancel
    }
    public func request(_ request: OSSystemExtensionRequest, didFailWithError error: Error) {
        guard request === activeRequest else { return }; finish("failed", error)
    }
    public func request(_ request: OSSystemExtensionRequest, didFinishWithResult result: OSSystemExtensionRequest.Result) {
        guard request === activeRequest else { return }
        activeRequest = nil
        if result == .willCompleteAfterReboot { finish("restart_required"); return }
        if removing { finish("removed"); return }
        let manager = preferences()
        io(manager.loadFromPreferences) { error in
            if let error { self.finish("failed", error); return }
            guard FilterConfigurationOwnership.mayModify(hasConfiguration: manager.providerConfiguration != nil, providerIdentifier: manager.providerConfiguration?.filterDataProviderBundleIdentifier, enabled: manager.isEnabled) else { self.finish("configuration_conflict"); return }
            guard !self.barrier.latched, let initial = self.initialPolicy else { self.finish("cancelled"); return }
            let config = NEFilterProviderConfiguration()
            config.filterSockets = true; config.filterPackets = false
            config.filterDataProviderBundleIdentifier = "com.flowguard.filter"
            config.vendorConfiguration = InitialPolicy.vendorConfiguration(previous: manager.providerConfiguration?.vendorConfiguration ?? [:], policy: initial)
            manager.providerConfiguration = config; manager.localizedDescription = "FlowGuard"; manager.isEnabled = true
            self.io(manager.saveToPreferences) { error in self.finish(error == nil ? "configuration_saved" : "failed", error) }
        }
    }
    public func disable(remove: Bool, completion: @escaping (String, Error?) -> Void) {
        guard authorized else { completion("signature_required", nil); return }
        if !remove {
            guard emergency == nil, !disabling else { completion("busy", nil); return }
            emergency = completion; barrier.requestRecovery(); activeRequest = nil
            // Approval requests can finish later, but their delegate identity is fenced out.
            interrupt(); return
        }
        guard self.completion == nil, emergency == nil else { completion("busy", nil); return }
        startDisable(remove: true, completion: completion)
    }
    private func startDisable(remove: Bool, completion: @escaping (String, Error?) -> Void) {
        self.completion = completion; removing = remove; disabling = !remove
        let manager = preferences()
        io(manager.loadFromPreferences) { error in
            if let error { self.finish("failed", error); return }
            guard FilterConfigurationOwnership.mayModify(hasConfiguration: manager.providerConfiguration != nil, providerIdentifier: manager.providerConfiguration?.filterDataProviderBundleIdentifier, enabled: manager.isEnabled) else { self.finish("configuration_conflict"); return }
            if manager.providerConfiguration == nil && !manager.isEnabled {
                if remove { self.deactivate() } else { self.finish("disabled") }; return
            }
            manager.isEnabled = false
            self.io(manager.saveToPreferences) { error in
                if let error { self.finish("failed", error); return }
                self.io(manager.loadFromPreferences) { error in
                    if let error { self.finish("failed", error); return }
                    guard !manager.isEnabled, FilterConfigurationOwnership.mayModify(hasConfiguration: manager.providerConfiguration != nil, providerIdentifier: manager.providerConfiguration?.filterDataProviderBundleIdentifier, enabled: manager.isEnabled) else { self.finish("configuration_conflict"); return }
                    guard remove else { self.finish("disabled"); return }
                    self.io(manager.removeFromPreferences) { error in
                        if let error { self.finish("failed", error) } else { self.deactivate() }
                    }
                }
            }
        }
    }
    private func deactivate() {
        let request = OSSystemExtensionRequest.deactivationRequest(forExtensionWithIdentifier: "com.flowguard.filter", queue: .main)
        activeRequest = request; request.delegate = self; submit(request)
    }
    public func persist(policy: Data, completion: @escaping (String, Error?) -> Void) {
        guard authorized else { completion("signature_required", nil); return }
        guard !barrier.latched else { completion("cancelled", nil); return }
        guard self.completion == nil, emergency == nil else { completion("busy", nil); return }
        guard (try? InitialPolicy.decode(policy)) != nil else { completion("invalid_initial_policy", nil); return }
        self.completion = completion
        let manager = preferences()
        io(manager.loadFromPreferences) { error in
            if let error { self.finish("failed", error); return }
            guard manager.isEnabled, let config = manager.providerConfiguration, config.filterDataProviderBundleIdentifier == "com.flowguard.filter" else { self.finish("configuration_conflict"); return }
            config.vendorConfiguration = InitialPolicy.vendorConfiguration(previous: config.vendorConfiguration ?? [:], policy: policy)
            manager.providerConfiguration = config
            self.io(manager.saveToPreferences) { error in
                if let error { self.finish("failed", error); return }
                self.io(manager.loadFromPreferences) { error in
                    if let error { self.finish("failed", error); return }
                    guard InitialPolicy.obsoleteKeys.allSatisfy({ manager.providerConfiguration?.vendorConfiguration?[$0] == nil }), InitialPolicy.persisted(enabled: manager.isEnabled, provider: manager.providerConfiguration?.filterDataProviderBundleIdentifier, stored: manager.providerConfiguration?.vendorConfiguration?[InitialPolicy.key] as? Data, requested: policy) else { self.finish("failed"); return }
                    self.finish("policy_persisted")
                }
            }
        }
    }
}
