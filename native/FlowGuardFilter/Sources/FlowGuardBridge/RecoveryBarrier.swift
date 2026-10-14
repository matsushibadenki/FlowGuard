/// Main-queue lifecycle serialization. A timeout never releases outstanding OS I/O.
public final class RecoveryBarrier {
    public private(set) var ioInFlight = false
    public private(set) var latched = false
    public private(set) var pending = false
    public private(set) var generation: UInt64 = 0
    public init() {}
    public func admit(_ value: UInt64) -> Bool {
        guard value >= generation else { return false }
        generation = value; return true
    }
    public func beginIO() { precondition(!ioInFlight); ioInFlight = true }
    public func endIO() { precondition(ioInFlight); ioInFlight = false }
    public func requestRecovery() { latched = true; pending = true }
    public func takeRecovery() -> Bool {
        guard pending, !ioInFlight else { return false }
        pending = false; return true
    }
    public func explicitActivation() -> Bool {
        guard !ioInFlight, !pending else { return false }
        latched = false; return true
    }
}
