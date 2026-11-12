import XCTest
@testable import ExactKit
#if os(macOS)
import AppKit
#endif

final class StickyConstraintTests: XCTestCase {
    func testOppositeInsetsKeepAnOversizedBoxAtTheStartUntilItsContainingBlockEnds() throws {
        // Chrome's positions from kernel/tests/it/fixtures/browser_sticky.tsv,
        // "both insets taller than port", also transposed onto the horizontal axis.
        let samples: [(CGFloat, CGFloat)] = [(0, 0), (20, 20), (50, 50), (100, 100), (400, 350)]
        let vertical = try XCTUnwrap(StickyConstraint([
            "scroller": 1, "natural": [0, 20, 50, 170], "limit": [0, 0, 200, 500],
            "port": [0, 0, 200, 100], "insets": [0, NSNull(), 0, NSNull()],
        ]))
        let horizontal = try XCTUnwrap(StickyConstraint([
            "scroller": 1, "natural": [20, 0, 170, 50], "limit": [0, 0, 500, 200],
            "port": [0, 0, 100, 200], "insets": [NSNull(), 0, NSNull(), 0],
        ]))
        for (scroll, position) in samples {
            XCTAssertEqual(vertical.offset(CGPoint(x: 0, y: scroll)), CGPoint(x: 0, y: position - 20), "scroll \(scroll)")
            XCTAssertEqual(horizontal.offset(CGPoint(x: scroll, y: 0)), CGPoint(x: position - 20, y: 0), "scroll \(scroll)")
        }
    }

    func testEndInsetIsReducedOnlyWhenTheStickyViewRectangleCannotFitTheBox() throws {
        let cases: [(CGFloat, [Any], CGFloat)] = [
            (150, [10, NSNull(), 20, NSNull()], 10),
            (150, [NSNull(), NSNull(), 0, NSNull()], 0),
            (150, [NSNull(), NSNull(), NSNull(), NSNull()], 20),
            (50, [0, NSNull(), 0, NSNull()], 20),
        ]
        for (height, insets, position) in cases {
            let constraint = try XCTUnwrap(StickyConstraint([
                "scroller": 1, "natural": [0, 20, 50, 20 + height], "limit": [0, 0, 200, 500],
                "port": [0, 0, 200, 100], "insets": insets,
            ]))
            XCTAssertEqual(20 + constraint.offset(.zero).y, position, "height \(height), insets \(insets)")
        }
    }
}

#if os(macOS)
final class StickyMacTests: XCTestCase {
    func testScrollingPlacesAnOversizedStickyBoxAtTheScrollportsTop() throws {
        let fixture = try SheetFixture(page: """
        component App
          view
            scroll width=200 height=100 testId="port"
              column height=500
                box height=20
                box position="sticky" top=0 bottom=0 height=150 width=50 testId="sticky"
                box height=330
        """)
        defer { fixture.close() }
        let port = try fixture.node("port"), sticky = try fixture.node("sticky")
        let scroll = try XCTUnwrap(port.scroll)
        let samples: [(CGFloat, CGFloat)] = [(0, 0), (20, 0), (50, 0), (100, 0), (400, -50)]
        for (offset, expected) in samples {
            scroll.contentView.scroll(to: CGPoint(x: 0, y: offset))
            fixture.settle()
            XCTAssertEqual(scroll.contentView.bounds.minY, offset, accuracy: 0.01)
            let drawn = scroll.convert(sticky.bounds, from: sticky)
            XCTAssertEqual(drawn.minY, expected, accuracy: 0.01, "scroll \(offset)")
        }
    }
}
#endif
