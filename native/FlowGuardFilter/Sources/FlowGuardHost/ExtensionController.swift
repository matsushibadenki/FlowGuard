import Foundation
import SystemExtensions
import NetworkExtension
import FlowGuardBridge
/// Compiled host lifecycle API. The unsigned CLI never invokes it.
/// Call from the signed containing desktop app on explicit user activation.
public final class ExtensionController: NSObject, OSSystemExtensionRequestDelegate {
    private var completion: ((String, Error?) -> Void)?
    public func activate(completion: @escaping (String, Error?) -> Void) {
        guard BridgeSigning.teamIdentifier() != nil, Bundle.main.bundleIdentifier == "com.flowguard.desktop" else {
            completion("signature_required", NSError(domain: "FlowGuardBridge", code: 1)); return
        }
        guard self.completion == nil else { completion("busy", NSError(domain: "FlowGuardBridge", code: 2)); return }
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
        let manager = NEFilterManager.shared()
        manager.loadFromPreferences { [weak self] error in
            guard let self else { return }
            if let error { let callback = self.completion; self.completion = nil; callback?("failed", error); return }
            // Do not overwrite another product's existing filter configuration.
            if (manager.providerConfiguration != nil && manager.providerConfiguration?.filterDataProviderBundleIdentifier != "com.flowguard.filter") || (manager.isEnabled && manager.providerConfiguration == nil) {
                let callback = self.completion; self.completion = nil
                callback?("configuration_conflict", NSError(domain: "FlowGuardBridge", code: 3)); return
            }
            let config = NEFilterProviderConfiguration()
            config.filterSockets = true; config.filterPackets = false
            config.filterDataProviderBundleIdentifier = "com.flowguard.filter"
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
