// swift-tools-version: 5.9
import PackageDescription
let package = Package(name: "FlowGuardFilter", platforms: [.macOS(.v12)], products: [
    .library(name: "FlowGuardPolicy", targets: ["FlowGuardPolicy"]),
    .library(name: "FlowGuardExtension", targets: ["FlowGuardExtension"])
], targets: [
    .target(name: "FlowGuardPolicy"),
    .target(name: "FlowGuardExtension", dependencies: ["FlowGuardPolicy"]),
    .testTarget(name: "FlowGuardPolicyTests", dependencies: ["FlowGuardPolicy"])
])
