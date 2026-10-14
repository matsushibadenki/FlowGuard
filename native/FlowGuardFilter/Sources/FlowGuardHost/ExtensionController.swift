import Foundation
import SystemExtensions
import NetworkExtension
import FlowGuardBridge
/// Compiled host lifecycle API. The unsigned CLI never invokes it.
/// Call from the signed containing desktop app on explicit user activation.
public final class ExtensionController: NSObject, OSSystemExtensionRequestDelegate {
    private var completion: ((String, Error?) -> Void)?
    private var initialPolicy: Data?
    private var removing = false
    public func activate(policy: Data, completion: @escaping (String, Error?) -> Void) {
        guard BridgeSigning.teamIdentifier() != nil, Bundle.main.bundleIdentifier == "com.flowguard.desktop" else {
            completion("signature_required", NSError(domain: "FlowGuardBridge", code: 1)); return
        }
        guard self.completion == nil else { completion("busy", NSError(domain: "FlowGuardBridge", code: 2)); return }
        guard (try? InitialPolicy.decode(policy)) != nil else { completion("invalid_initial_policy", nil); return }
        initialPolicy = policy; removing = false
        self.completion = completion
        let request = OSSystemExtensionRequest.activationRequest(forExtensionWithIdentifier: "com.flowguard.filter", queue: .main)
        request.delegate = self; OSSystemExtensionManager.shared.submitRequest(request)
    }
    public func requestNeedsUserApproval(_ request: OSSystemExtensionRequest) { completion?("approval_required", nil) }
    public func request(_ request: OSSystemExtensionRequest, actionForReplacingExtension existing: OSSystemExtensionProperties, withExtension ext: OSSystemExtensionProperties) -> OSSystemExtensionRequest.ReplacementAction { .replace }
    public func request(_ request: OSSystemExtensionRequest, didFailWithError error: Error) {
        let callback = completion; completion = nil; callback?("failed", error)
    }
    public func request(_ request: OSSystemExtensionRequest, didFinishWithResult result: OSSystemExtensionRequest.Result) {
        if result == .willCompleteAfterReboot {
            let callback = completion; completion = nil; callback?("restart_required", nil); return
        }
        if removing { finish("removed", nil); return }
        let manager = NEFilterManager.shared()
        manager.loadFromPreferences { [weak self] error in
            guard let self else { return }
            if let error { let callback = self.completion; self.completion = nil; callback?("failed", error); return }
            // Do not overwrite another product's existing filter configuration.
            if !FilterConfigurationOwnership.mayModify(hasConfiguration: manager.providerConfiguration != nil, providerIdentifier: manager.providerConfiguration?.filterDataProviderBundleIdentifier, enabled: manager.isEnabled) {
                let callback = self.completion; self.completion = nil
                callback?("configuration_conflict", NSError(domain: "FlowGuardBridge", code: 3)); return
            }
            let config = NEFilterProviderConfiguration()
            config.filterSockets = true; config.filterPackets = false
            config.filterDataProviderBundleIdentifier = "com.flowguard.filter"
            guard let initial = self.initialPolicy else { self.finish("invalid_initial_policy", nil); return }
            config.vendorConfiguration = [InitialPolicy.key: initial]
            manager.providerConfiguration = config; manager.localizedDescription = "FlowGuard"
            manager.isEnabled = true
            manager.saveToPreferences { error in
                let callback = self.completion; self.completion = nil
                // Saved configuration is not evidence of provider readiness or rule acceptance.
                callback?(error == nil ? "configuration_saved" : "failed", error)
            }
        }
    }
}

extension ExtensionController {
    private func finish(_ status: String, _ error: Error?) {
        let callback = completion; completion = nil; initialPolicy = nil
        callback?(status, error)
    }
    /// Disable our configuration first. Never removes another product's configuration.
    public func disable(remove: Bool, completion: @escaping (String, Error?) -> Void) {
        guard BridgeSigning.teamIdentifier() != nil, Bundle.main.bundleIdentifier == "com.flowguard.desktop" else { completion("signature_required", nil); return }
        guard self.completion == nil else { completion("busy", nil); return }
        self.completion = completion; removing = remove
        let manager = NEFilterManager.shared()
        manager.loadFromPreferences { [weak self] error in
            DispatchQueue.main.async {
                guard let self else { return }
                if let error { self.finish("failed", error); return }
                guard FilterConfigurationOwnership.mayModify(hasConfiguration: manager.providerConfiguration != nil, providerIdentifier: manager.providerConfiguration?.filterDataProviderBundleIdentifier, enabled: manager.isEnabled) else { self.finish("configuration_conflict", nil); return }
                if manager.providerConfiguration == nil && !manager.isEnabled {
                    if remove {
                        let request = OSSystemExtensionRequest.deactivationRequest(forExtensionWithIdentifier: "com.flowguard.filter", queue: .main)
                        request.delegate = self; OSSystemExtensionManager.shared.submitRequest(request)
                    } else { self.finish("disabled", nil) }
                    return
                }
                manager.isEnabled = false
                manager.saveToPreferences { error in
                    DispatchQueue.main.async {
                        if let error { self.finish("failed", error); return }
                        manager.loadFromPreferences { error in
                            DispatchQueue.main.async {
                                if let error { self.finish("failed", error); return }
                                guard !manager.isEnabled else { self.finish("failed", nil); return }
                                guard remove else { self.finish("disabled", nil); return }
                                guard manager.providerConfiguration == nil || manager.providerConfiguration?.filterDataProviderBundleIdentifier == "com.flowguard.filter" else { self.finish("configuration_conflict", nil); return }
                                manager.removeFromPreferences { error in
                                    DispatchQueue.main.async {
                                        if let error { self.finish("failed", error); return }
                                        let request = OSSystemExtensionRequest.deactivationRequest(forExtensionWithIdentifier: "com.flowguard.filter", queue: .main)
                                        request.delegate = self; OSSystemExtensionManager.shared.submitRequest(request)
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

extension ExtensionController {
    /// Persist before live synchronization so a provider restart cannot restore an older policy.
    public func persist(policy: Data, completion: @escaping (String, Error?) -> Void) {
        guard BridgeSigning.teamIdentifier() != nil, Bundle.main.bundleIdentifier == "com.flowguard.desktop" else { completion("signature_required", nil); return }
        guard self.completion == nil else { completion("busy", nil); return }
        guard (try? InitialPolicy.decode(policy)) != nil else { completion("invalid_initial_policy", nil); return }
        self.completion = completion
        let manager = NEFilterManager.shared()
        manager.loadFromPreferences { [weak self] error in
            DispatchQueue.main.async {
                guard let self else { return }
                if let error { self.finish("failed", error); return }
                guard manager.isEnabled, let config = manager.providerConfiguration,
                      config.filterDataProviderBundleIdentifier == "com.flowguard.filter" else { self.finish("configuration_conflict", nil); return }
                config.vendorConfiguration = InitialPolicy.vendorConfiguration(previous: config.vendorConfiguration ?? [:], policy: policy)
                manager.providerConfiguration = config
                manager.saveToPreferences { error in
                    DispatchQueue.main.async {
                        if let error { self.finish("failed", error); return }
                        manager.loadFromPreferences { error in
                            DispatchQueue.main.async {
                                if let error { self.finish("failed", error); return }
                                guard InitialPolicy.obsoleteKeys.allSatisfy({ manager.providerConfiguration?.vendorConfiguration?[$0] == nil }), InitialPolicy.persisted(enabled: manager.isEnabled,
                                      provider: manager.providerConfiguration?.filterDataProviderBundleIdentifier,
                                      stored: manager.providerConfiguration?.vendorConfiguration?[InitialPolicy.key] as? Data,
                                      requested: policy) else { self.finish("failed", nil); return }
                                self.finish("policy_persisted", nil)
                            }
                        }
                    }
                }
            }
        }
    }
}
