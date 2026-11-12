import Foundation
import XCTest
@testable import ExactKit

final class DateValueTests: XCTestCase {
    func testInitialValuesUseLocalFieldsAndTheSameRepresentationAsTheirBounds() throws {
        let instant = try XCTUnwrap(DateValue.parse("datetime-local", "2026-11-10T02:30"))
        let zone = try XCTUnwrap(TimeZone(secondsFromGMT: -8 * 60 * 60))
        for (kind, local, later, earlier) in [
            ("date", "2026-11-09", "2026-11-12", "2026-11-01"),
            ("time", "18:30", "19:00", "09:00"),
            ("datetime-local", "2026-11-09T18:30", "2026-11-12T19:00", "2026-11-01T09:00"),
        ] {
            let expected = try XCTUnwrap(DateValue.parse(kind, local))
            let minimum = try XCTUnwrap(DateValue.parse(kind, later))
            let maximum = try XCTUnwrap(DateValue.parse(kind, earlier))
            XCTAssertEqual(DateValue.initial(kind, now: instant, timeZone: zone), expected)
            XCTAssertEqual(DateValue.initial(kind, now: instant, timeZone: zone, minimum: minimum), minimum)
            XCTAssertEqual(DateValue.initial(kind, now: instant, timeZone: zone, maximum: maximum), maximum)
        }
    }
}
