import Foundation
import FlowGuardHost
import FlowGuardBridge
import FlowGuardPolicy
struct RulePayload: Decodable { let rules: [PolicyRule] }
let arguments = Array(CommandLine.arguments.dropFirst())
let bridge = HostBridge()
func output(_ data: Data) { FileHandle.standardOutput.write(data); FileHandle.standardOutput.write(Data("\n".utf8)) }
if arguments.count == 2 && arguments[0] == "--preflight" {
    output(try JSONEncoder().encode(bridge.preflight(extensionURL: URL(fileURLWithPath: arguments[1]))))
} else if arguments == ["--status"] || arguments == ["--push"] || arguments == ["--sync"] {
    // A signed helper must not become a proxy for arbitrary local CLI callers.
    guard BridgeSigning.trustedDesktopParent() else { fputs("trusted_desktop_required\n", stderr); exit(1) }
    let data = arguments != ["--status"] ? FileHandle.standardInput.readData(ofLength: 4 * 1024 * 1024 + 1) : nil
    if let data, data.count > 4 * 1024 * 1024 { fputs("invalid_payload\n", stderr); exit(2) }
    if arguments == ["--sync"] {
        do {
            let payload = try JSONDecoder().decode(RulePayload.self, from: data ?? Data())
            bridge.sync(rules: payload.rules) { result in
                switch result {
                case .success(let reply): output(try! JSONEncoder().encode(reply)); exit(0)
                case .failure(let error): fputs("\(error.localizedDescription)\n", stderr); exit(1)
                }
            }
        } catch { fputs("invalid_payload\n", stderr); exit(2) }
        dispatchMain()
    }
    bridge.transact(update: data) { result in
        switch result {
        case .success(let reply): output(reply); exit(0)
        case .failure(let error): fputs("\(error.localizedDescription)\n", stderr); exit(1)
        }
    }
    dispatchMain()
} else {
    fputs("Usage: FlowGuardBridgeCLI --preflight EXTENSION | --status | --push < update.json | --sync < rules.json\n", stderr); exit(2)
}
