import XCTest
@testable import FlowGuardBridge
final class RecoveryBarrierTests: XCTestCase {
    func testOutstandingSaveCannotBeReleasedByTimeoutAndRecoveryWinsNextStep() {
        let gate = RecoveryBarrier()
        XCTAssertTrue(gate.admit(1)); gate.beginIO()
        gate.requestRecovery()
        XCTAssertFalse(gate.takeRecovery())
        XCTAssertFalse(gate.explicitActivation())
        // Caller timeout does not call endIO: no second OS write can start.
        XCTAssertTrue(gate.ioInFlight)
        gate.endIO()
        XCTAssertTrue(gate.takeRecovery())
        XCTAssertTrue(gate.latched)
        gate.beginIO(); gate.endIO() // disable load/save run only after the first callback.
        XCTAssertFalse(gate.takeRecovery())
    }
    func testQueuedOldSubmissionsCannotUndoRecoveryAndExplicitActivationIsRequired() {
        let gate = RecoveryBarrier()
        XCTAssertTrue(gate.admit(5)); gate.requestRecovery()
        XCTAssertTrue(gate.takeRecovery())
        XCTAssertFalse(gate.admit(4)) // delayed activation/persistence worker
        XCTAssertTrue(gate.latched)
        XCTAssertTrue(gate.admit(6)); XCTAssertTrue(gate.explicitActivation())
        XCTAssertFalse(gate.latched)
        XCTAssertFalse(gate.admit(5))
    }
    func testApprovalWaitCanYieldImmediatelyButRecoveryIOStillSerializes() {
        let gate = RecoveryBarrier()
        gate.requestRecovery(); XCTAssertTrue(gate.takeRecovery())
        gate.beginIO(); gate.requestRecovery()
        XCTAssertFalse(gate.takeRecovery())
        gate.endIO(); XCTAssertTrue(gate.takeRecovery())
    }
}
