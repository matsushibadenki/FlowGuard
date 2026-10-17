import Foundation
import Darwin

public enum FirewallTool {
    public static func run(_ arguments: [String]) -> String? {
        guard arguments == ["--getglobalstate"] || arguments == ["--setglobalstate", "on"] || arguments == ["--setglobalstate", "off"] else { return nil }
        let process = Process(), pipe = Pipe()
        process.executableURL = URL(fileURLWithPath: "/usr/libexec/ApplicationFirewall/socketfilterfw")
        process.arguments = arguments
        process.environment = ["PATH": "/usr/bin:/bin", "LANG": "C", "LC_ALL": "C"]
        process.standardInput = FileHandle.nullDevice
        process.standardOutput = pipe; process.standardError = FileHandle.nullDevice
        // Fixed tool normally emits one line; drain concurrently with a strict retention cap.
        let result = Output()
        let group = DispatchGroup(); group.enter()
        DispatchQueue.global().async {
            defer { group.leave() }
            while let data = try? pipe.fileHandleForReading.read(upToCount: 1024), !data.isEmpty { result.append(data) }
        }
        do { try process.run() } catch { try? pipe.fileHandleForWriting.close(); return nil }
        try? pipe.fileHandleForWriting.close()
        let deadline = ProcessInfo.processInfo.systemUptime + 5
        while process.isRunning && ProcessInfo.processInfo.systemUptime < deadline { Thread.sleep(forTimeInterval: 0.02) }
        if process.isRunning { kill(process.processIdentifier, SIGKILL); process.waitUntilExit(); return nil }
        guard group.wait(timeout: .now() + 1) == .success, process.terminationStatus == 0 else { return nil }
        return result.text()
    }
    public static func state() -> Bool? { run(["--getglobalstate"]).flatMap(FirewallContract.parseState) }
}
private final class Output: @unchecked Sendable {
    private let lock = NSLock(); private var data = Data(); private var overflow = false
    func append(_ part: Data) { lock.lock(); defer { lock.unlock() }; if data.count + part.count > 4096 { overflow = true } else { data.append(part) } }
    func text() -> String? { lock.lock(); defer { lock.unlock() }; return overflow ? nil : String(data: data, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines) }
}
