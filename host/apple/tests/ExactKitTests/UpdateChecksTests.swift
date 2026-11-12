import XCTest
@testable import ExactUpdates

final class UpdateChecksTests: XCTestCase {
    func testEveryPolicyDiscoversWhileActiveAndOnlyImmediateActivates() {
        for policy in ["next-launch", "app-decides", "immediate"] {
            var time: TimeInterval = 0
            var tick: (() -> Void)?
            var requests = 0, activations = 0, timers = 0
            let checks = ForegroundUpdateChecks(interval: 3, immediate: policy == "immediate", check: {
                requests += 1
            }, activate: {
                activations += 1
            }, now: { time }, schedule: { interval, callback in
                XCTAssertEqual(interval, 3)
                timers += 1
                tick = callback
                return { timers -= 1 }
            })
            checks.foreground(false)
            XCTAssertEqual(requests, 0)
            checks.foreground(true)
            checks.foreground(true)
            XCTAssertEqual(requests, 1)
            XCTAssertEqual(timers, 1)
            time += 3
            tick?()
            checks.completed("staged seq 2")
            XCTAssertEqual(requests, 2, policy)
            XCTAssertEqual(activations, policy == "immediate" ? 1 : 0, policy)
            checks.foreground(false)
            XCTAssertEqual(timers, 0)
            tick?()
            checks.completed("staged seq 3")
            XCTAssertEqual(requests, 2)
            XCTAssertEqual(activations, policy == "immediate" ? 1 : 0)
            time += 1
            checks.foreground(true)
            XCTAssertEqual(requests, 2, "foreground checks are debounced")
            XCTAssertEqual(activations, policy == "immediate" ? 2 : 0, "a check completed in the background waits for foreground")
            checks.foreground(false)
            time += 60
            checks.foreground(true)
            XCTAssertEqual(requests, 3)
        }
    }

    func testTheDefaultIsFifteenMinutesAndInvalidIntervalsFallBack() {
        for interval: TimeInterval? in [nil, 0, -1, .infinity, .nan] {
            let checks = ForegroundUpdateChecks(interval: interval, immediate: false, check: {}, activate: {})
            XCTAssertEqual(checks.interval, 900)
        }
    }

    /// The store's C entries share one input and one output buffer, so a
    /// call from a worker (a resolver's asset read) must not take the main
    /// thread's answer (an activation's `prepare`). No store is open here:
    /// every `asset` call for an unpinned token is refused the same way.
    func testConcurrentCallsEachReadTheirOwnAnswer() throws {
        try XCTSkipUnless(Updates.linked, "this test archive links no update store")
        let unknown = Updates.Selection(token: 999, entry: "x", seq: 0, plan: Data(), assets: [])
        let deadline = Date().addingTimeInterval(0.5)
        let done = DispatchGroup(), results = NSLock()
        var calls = 0, wrong = 0, sample = ""
        for _ in 0..<2 {
            done.enter()
            Thread {
                var n = 0, bad = 0, last = ""
                while Date() < deadline {
                    let answer: String
                    switch Updates.asset(unknown, name: "assets/photo.jpg") {
                    case .success(let data): answer = "success \(data?.count ?? -1) bytes"
                    case .failure(let error): answer = error.localizedDescription
                    }
                    n += 1
                    if answer != "unknown update generation" { bad += 1; last = answer }
                }
                results.withLock { calls += n; wrong += bad; if bad > 0 { sample = last } }
                done.leave()
            }.start()
        }
        var prepared = 0, selected = 0
        while done.wait(timeout: .now()) == .timedOut {
            if Updates.prepare() != nil { selected += 1 }
            prepared += 1
        }
        XCTAssertGreaterThan(prepared, 0)
        XCTAssertEqual(selected, 0, "a prepare with no store open selected a candidate")
        XCTAssertEqual(wrong, 0, "\(wrong) of \(calls) asset calls read another call's answer, e.g. \(sample)")
    }
}
