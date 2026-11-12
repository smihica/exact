import XCTest
#if canImport(UIKit)
import UIKit
#else
import AppKit
#endif
@testable import ExactKit

final class SessionLifetimeTests: XCTestCase {
    private final class Refuses: ExactSessionDelegate, ExactLeaveDelegate {
        var asks = 0
        func exactSession(_ session: ExactSession, shouldLeaveFor url: String, how: String) -> Bool {
            asks += 1
            return false
        }
    }

    private func ask(_ view: ExactView) -> Bool? {
        let request: NSMutableDictionary = ["url": "https://example.test/", "how": "external", "view": view]
        NotificationCenter.default.post(name: Notification.Name("exact.leave"), object: request)
        return request["allow"] as? Bool
    }

    func testReplacementSessionsAskTheirOwnLeaveDelegateOnce() {
        let refuses = Refuses()
        for index in 0..<16 {
            autoreleasepool {
                let session = ExactApp.shared.makeSession(delegate: refuses)
                let view = ExactView(session: session)
                session.observeLeaving()
                session.observeLeaving()
                XCTAssertEqual(ask(view), false, "replacement \(index)")
                XCTAssertEqual(refuses.asks, index + 1)
                session.destroy()
            }
        }
    }

    func testDestroyedSessionStopsAnsweringLeaveRequests() {
        let refuses = Refuses()
        let session = ExactApp.shared.makeSession(delegate: refuses)
        let view = ExactView(session: session)
        session.observeLeaving()
        XCTAssertEqual(ask(view), false)
        session.destroy()
        session.observeLeaving()
        XCTAssertNil(ask(view))
        XCTAssertEqual(refuses.asks, 1)
    }

    func testReplacementSessionsHaveTheirOwnLaunchAndSeenEntries() {
        let marks = NavigationMarks.shared
        for index in 0..<16 {
            autoreleasepool {
                let session = ExactApp.shared.makeSession()
                marks.routerChanged(session, payload: ["top": 1, "pattern": "/launch"])
                XCTAssertEqual(marks.recent.last, "launch /launch", "replacement \(index)")
                marks.routerChanged(session, payload: ["top": 2, "pattern": "/details"])
                XCTAssertEqual(marks.recent.last, "change /details cold program", "replacement \(index)")
                marks.routerChanged(session, payload: ["top": 1, "pattern": "/launch"])
                XCTAssertEqual(marks.recent.last, "change /launch warm program")
                session.destroy()
            }
        }
    }
}
