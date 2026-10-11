import Foundation
import NetworkExtension
import FlowGuardExtension
// Ensure the provider module is linked into this executable for Objective-C runtime lookup.
_ = FilterDataProvider.self
NEProvider.startSystemExtensionMode()
dispatchMain()
