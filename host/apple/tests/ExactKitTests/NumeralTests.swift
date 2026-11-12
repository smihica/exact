#if os(iOS) || os(tvOS)
import Combine
import UIKit
import XCTest
@testable import ExactNumerals

final class NumeralTests: XCTestCase {
    func testUnchangedDigitsTakeNewAttributesWithoutRolling() throws {
        let roll = NumeralRoll()
        let text = NSMutableAttributedString(string: "41", attributes: [
            .foregroundColor: UIColor.black,
            .font: UIFont.systemFont(ofSize: 17),
        ])
        let range = NSRange(location: 0, length: text.length)
        roll.show(text, align: 0, mode: "numeric")
        let version = roll.model.version
        XCTAssertEqual(version, 1)

        for (attribute, value): (NSAttributedString.Key, Any) in [
            (.foregroundColor, UIColor.white),
            (.font, UIFont.systemFont(ofSize: 28)),
            (.font, UIFont.boldSystemFont(ofSize: 28)),
            (.kern, 2.0),
            (.underlineStyle, NSUnderlineStyle.single.rawValue),
        ] {
            text.addAttribute(attribute, value: value, range: range)
            roll.show(text, align: 1, mode: "numeric")
            XCTAssertEqual(roll.model.text, try AttributedString(text, including: \.uiKit), "\(attribute)")
            XCTAssertEqual(roll.model.version, version, "restyling does not start a roll")
            XCTAssertFalse(roll.model.down)
            XCTAssertEqual(roll.model.alignment, .center)
        }

        var paints = 0
        let observation = roll.model.$text.dropFirst().sink { _ in paints += 1 }
        defer { observation.cancel() }
        roll.show(NSAttributedString(attributedString: text), align: 2, mode: "numeric")
        XCTAssertEqual(paints, 0, "an equal attributed string needs no new paint")
        XCTAssertEqual(roll.model.alignment, .trailing)
        XCTAssertEqual(roll.model.version, version)
    }

    func testChangedDigitsKeepTheirDirectionAndCurrentStyle() throws {
        let roll = NumeralRoll()
        let initial = NSAttributedString(string: "41", attributes: [.foregroundColor: UIColor.black])
        roll.show(initial, align: 0, mode: "numeric")
        for (index, item) in [("42", "numeric", false), ("40", "numeric", true),
                              ("43", "numeric-countdown", true)].enumerated() {
            let (digits, mode, down) = item
            let text = NSAttributedString(string: digits, attributes: [.foregroundColor: UIColor.white])
            roll.show(text, align: 0, mode: mode)
            XCTAssertEqual(roll.model.text, try AttributedString(text, including: \.uiKit))
            XCTAssertEqual(roll.model.plain, digits)
            XCTAssertEqual(roll.model.version, index + 2)
            XCTAssertEqual(roll.model.down, down)
        }
    }
}
#endif
