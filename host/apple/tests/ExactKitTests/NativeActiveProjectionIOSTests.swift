#if os(iOS)
import UIKit
import XCTest
@testable import ExactKit
@testable import ExactNavigation

extension NavigationBarIOSTests {
    func testCachedNativeBarMenuRejectsNewActivePropertiesWithoutReprojection() throws {
        let session = try fixture("cached-bar-menu-refusal", module: false)
        let p = session.presenter
        let original = try node(session, "menu-item"), row = try node(session, "bar-menu-bump")
        let header = try node(session, "header-home")
        let pop = try XCTUnwrap(p.carrying("popover").first { $0.props["id"] == "bar-menu" })
        let nav = try XCTUnwrap(p.navigationHost.primaryNavigation)
        let controller = try XCTUnwrap(nav.topViewController as? RouteController)
        let nativeID: UInt32 = 900001
        var props = original.props
        props["type"] = "button"
        let oldFace = p.buttonFace
        p.buttonFace = { id in
            if id == nativeID { var face = ButtonFace(); face.title = "More"; return face }
            return oldFace?(id) ?? ButtonFace()
        }
        let children = header.container.subviews.compactMap { $0 as? NodeView }.map { $0 === original ? nativeID : $0.id }
        p.apply(wireBatch([
            ["op": "create", "id": nativeID, "kind": "control", "props": props],
            ["op": "frame", "id": nativeID, "x": original.frame.minX, "y": original.frame.minY, "w": 44, "h": 44],
            ["op": "props", "id": original.id, "set": [:], "clear": ["popovertarget"]],
            ["op": "children", "id": header.id, "ids": children],
            ["op": "props", "id": pop.id, "set": [:], "clear": ["accessibilityRole"]],
        ]))
        let source = try XCTUnwrap(p.views[nativeID])
        XCTAssertTrue(source.isNativeButton)
        p.navigationHost.prepareRoutes([controller], in: nav)
        XCTAssertNil(pop.props["accessibilityRole"])
        XCTAssertTrue(pop.menuShaped)
        let item = try XCTUnwrap(controller.barItemsMade[source.id]?.bar)
        item.menu = UIMenu(children: p.menus.items(of: pop))
        let menu = try XCTUnwrap(item.menu), action = try XCTUnwrap(menu.children.first as? UIAction)
        let signature = controller.projectedSource
        XCTAssertNil(row.nativeActiveRefusals["UIAction"])
        p.apply(wireBatch([["op": "style", "id": row.id, "style": ["press_opacity_to": 0.4]]]))
        p.navigationHost.prepareRoutes([controller], in: nav)
        XCTAssertEqual(controller.projectedSource, signature)
        XCTAssertTrue(controller.barItemsMade[source.id]?.bar === item)
        XCTAssertNil(item.customView)
        XCTAssertTrue(item.menu === menu)
        XCTAssertTrue(item.menu?.children.first === action)
        XCTAssertTrue(row.nativeActiveRefusals["UIAction"]?.contains("opacity") == true, "the cached native action refuses opacity")
    }

    func testUnsupportedActivePropertiesKeepTheNativeBarItemAndDiagnoseTheProperty() throws {
        let session = try fixture("active-bar-refusal", module: false)
        let source = try node(session, "compose-home")
        let p = session.presenter
        let nav = try XCTUnwrap(p.navigationHost.primaryNavigation)
        let controller = try XCTUnwrap(nav.topViewController as? RouteController)
        let original = try XCTUnwrap(controller.barItemsMade[source.id]?.bar)
        p.apply(wireBatch([["op": "style", "id": source.id, "style": ["press_opacity_to": 0.4]]]))
        p.navigationHost.prepareRoutes([controller], in: nav)
        XCTAssertTrue(controller.barItemsMade[source.id]?.bar === original)
        XCTAssertNil(original.customView)
        let logs = session.agent(#"{"op":"logs","since":0}"#)
        XCTAssertTrue(logs.contains(":active") && logs.contains("opacity") && logs.contains("UIBarButtonItem"), logs)
    }

    func testTitleBarItemUsesOnlyItsHighlightedForegroundAndClearsIt() throws {
        let session = try fixture("active-title-bar", module: false)
        let source = try node(session, "compose-home")
        let p = session.presenter
        let nav = try XCTUnwrap(p.navigationHost.primaryNavigation)
        let controller = try XCTUnwrap(nav.topViewController as? RouteController)
        p.apply(wireBatch([
            ["op": "create", "id": 900001, "kind": "text", "props": ["text": "Go"], "style": [:]],
            ["op": "children", "id": source.id, "ids": [900001]],
            ["op": "style", "id": source.id, "style": ["press_color": [0, 0, 255, 255]]],
        ]))
        p.navigationHost.prepareRoutes([controller], in: nav)
        let item = try XCTUnwrap(controller.barItemsMade[source.id]?.bar)
        XCTAssertNil(item.image)
        XCTAssertNil(item.customView)
        XCTAssertNil(item.titleTextAttributes(for: .normal)?[.foregroundColor])
        let color = try XCTUnwrap(item.titleTextAttributes(for: .highlighted)?[.foregroundColor] as? UIColor)
        var blue: CGFloat = 0
        XCTAssertTrue(color.getRed(nil, green: nil, blue: &blue, alpha: nil))
        XCTAssertEqual(blue, 1)
        p.apply(wireBatch([["op": "style", "id": source.id, "style": ["press_color": [255, 0, 0, 255], "press_opacity_to": 0.4]]]))
        p.navigationHost.prepareRoutes([controller], in: nav)
        XCTAssertTrue(controller.barItemsMade[source.id]?.bar === item)
        XCTAssertNil(item.titleTextAttributes(for: .highlighted)?[.foregroundColor], "an unsupported combination refuses the whole active override")
        p.apply(wireBatch([["op": "style", "id": source.id, "style": [:]]]))
        p.navigationHost.prepareRoutes([controller], in: nav)
        XCTAssertNil(item.titleTextAttributes(for: .highlighted)?[.foregroundColor])
    }

    func testUnsupportedActivePropertiesKeepTheNativeHeaderSegment() throws {
        let session = try fixture("active-segment-refusal", module: false)
        let source = try node(session, "segment-all")
        let p = session.presenter
        let nav = try XCTUnwrap(p.navigationHost.primaryNavigation)
        let controller = try XCTUnwrap(nav.topViewController as? RouteController)
        let original = try XCTUnwrap(controller.navigationItem.titleView as? UISegmentedControl)
        p.apply(wireBatch([["op": "style", "id": source.id, "style": ["press_background_color": [255, 0, 0, 255]]]]))
        p.navigationHost.prepareRoutes([controller], in: nav)
        XCTAssertTrue(controller.navigationItem.titleView === original)
        let logs = session.agent(#"{"op":"logs","since":0}"#)
        XCTAssertTrue(logs.contains(":active") && logs.contains("background-color") && logs.contains("UISegmentedControl"), logs)
    }
}
#endif
