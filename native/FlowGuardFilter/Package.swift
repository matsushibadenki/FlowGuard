// swift-tools-version: 5.9
import PackageDescription
let package = Package(name: "FlowGuardFilter", platforms: [.macOS(.v13)], products: [
    .library(name: "FlowGuardHostRuntime", type: .dynamic, targets: ["FlowGuardHost"]),
    .library(name: "FlowGuardPolicy", targets: ["FlowGuardPolicy"]),
    .library(name: "FlowGuardExtension", targets: ["FlowGuardExtension"]),
    .executable(name: "FlowGuardSystemExtension", targets: ["FlowGuardSystemExtension"]),
    .executable(name: "FlowGuardBridgeCLI", targets: ["FlowGuardBridgeCLI"])
], targets: [
    .target(name: "FlowGuardPolicy"),
    .target(name: "FlowGuardBridge", dependencies: ["FlowGuardPolicy"]),
    .target(name: "FlowGuardExtension", dependencies: ["FlowGuardPolicy", "FlowGuardBridge"]),
    .target(name: "FlowGuardHost", dependencies: ["FlowGuardBridge", "FlowGuardPolicy"]),
    .executableTarget(name: "FlowGuardSystemExtension", dependencies: ["FlowGuardExtension"]),
    .executableTarget(name: "FlowGuardBridgeCLI", dependencies: ["FlowGuardHost", "FlowGuardBridge", "FlowGuardPolicy"]),
    .testTarget(name: "FlowGuardBridgeTests", dependencies: ["FlowGuardBridge", "FlowGuardPolicy"]),
    .testTarget(name: "FlowGuardHostTests", dependencies: ["FlowGuardHost", "FlowGuardBridge"]),
    .testTarget(name: "FlowGuardPolicyTests", dependencies: ["FlowGuardPolicy"])
])
