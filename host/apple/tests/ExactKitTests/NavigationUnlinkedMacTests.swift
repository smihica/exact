#if os(macOS)
import AppKit
import XCTest
@testable import ExactKit

/// LLP 1047.001 D10: a composition without navigation holds `NoNavigation`,
/// `NoSidebar` and `NoBackSwipe`, and a page with no navigation root observes
/// what it observes with navigation linked: its input gates (`inert`,
/// `aria-hidden`, a disabled control) applied after each batch, the focus
/// leaving what input no longer reaches, and Back and Forward as a page with
/// no root has them.
final class NavigationUnlinkedMacTests: XCTestCase {
    private var window: NSWindow?
    override func tearDown() { window?.close(); window = nil }

    func testAPageWithoutRoutesObservesTheSameWithoutNavigation() throws {
        let unlinked = try observe(linked: false)
        let linked = try observe(linked: true)
        XCTAssertEqual(unlinked, linked)
        // What each point observes, independent of how either host gets there.
        let expected: [(focus: String, hidden: Bool, field: Bool)] = [
            ("middle", false, true), ("middle", true, true), ("none", true, true), ("none", true, false),
            ("middle", false, true), ("middle", false, false), ("none", true, false)]
        XCTAssertEqual(unlinked.count, expected.count)
        for (seen, want) in zip(unlinked, expected) {
            XCTAssertTrue(seen.contains("focus \(want.focus),"), seen)
            XCTAssertTrue(seen.contains("bare hidden from VoiceOver \(want.hidden),"), seen)
            XCTAssertTrue(seen.contains("field enabled \(want.field),"), seen)
            // No navigation root: no way back or forward, though the root that is not there offers both.
            XCTAssertTrue(seen.contains("back false [1] forward false,"), seen)
            XCTAssertTrue(seen.contains("offers true,"), seen)
        }
    }

    private func observe(linked: Bool) throws -> [String] {
        let make = NavigationLink.make, sidebar = NavigationLink.makeSidebar, swipe = NavigationLink.makeBackSwipe
        if !linked { NavigationLink.make = nil; NavigationLink.makeSidebar = nil; NavigationLink.makeBackSwipe = nil }
        defer { NavigationLink.make = make; NavigationLink.makeSidebar = sidebar; NavigationLink.makeBackSwipe = swipe }
        _ = NSApplication.shared
        let p = Presenter()
        XCTAssertEqual(p.navigation is NoNavigation, !linked)
        XCTAssertEqual(p.sidebar is NoSidebar, !linked)
        XCTAssertEqual(p.backSwipe is NoBackSwipe, !linked)
        var face = ButtonFace(); face.title = "Button"
        p.buttonFace = { _ in face }
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 400, height: 300), styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        self.window = window
        window.contentView = p.viewport
        p.apply(wireBatch([
            ["op": "create", "id": 1, "kind": "view"],
            ["op": "frame", "id": 1, "x": 0, "y": 0, "w": 400, "h": 300],
            ["op": "create", "id": 5, "kind": "view", "props": ["id": "column"]],
            ["op": "frame", "id": 5, "x": 0, "y": 0, "w": 400, "h": 100],
            ["op": "create", "id": 3, "kind": "button", "props": ["id": "bare"], "handlers": ["press", "blur"]],
            ["op": "frame", "id": 3, "x": 0, "y": 0, "w": 120, "h": 30],
            ["op": "create", "id": 4, "kind": "control", "props": ["type": "button", "id": "middle"], "handlers": ["press", "blur"], "style": ["appearance": "auto"]],
            ["op": "frame", "id": 4, "x": 0, "y": 50, "w": 120, "h": 30],
            ["op": "create", "id": 6, "kind": "input", "props": ["id": "field", "value": ""]],
            ["op": "frame", "id": 6, "x": 0, "y": 150, "w": 200, "h": 30],
            ["op": "children", "id": 5, "ids": [3, 4]],
            ["op": "children", "id": 1, "ids": [5, 6]], ["op": "roots", "ids": [1]]]))
        p.focusElement(["middle"])

        var seen: [String] = []
        func note(_ point: String) throws {
            let bare = try XCTUnwrap(p.views[3]), field = try XCTUnwrap(p.views[6])
            seen.append([
                point,
                "focus \(p.focusedNode?.props["id"] ?? "none")",
                "bare hidden from VoiceOver \(bare.isAccessibilityHidden())",
                "field enabled \(field.field?.isEnabled ?? false)",
                "back \(p.navigation.canGoBack) \(p.navigation.backFacts) forward \(p.navigation.canGoForward)",
                "toolbar back \(p.navigation.backInToolbar) bar route \(p.navigation.barRoute) split \(p.navigation.stackSplit.active)",
                "back possible \(p.navigation.backPossible) offers \(p.navigation.offers(back: true))",
                "sidebar wanted \(p.sidebar.wanted)",
                "split ground \(bare.standsOnSplitGround) sheet \(p.dialogs.routeObservation == nil ? "none" : "some")",
            ].joined(separator: ", "))
        }
        try note("mounted")
        p.apply(wireBatch([["op": "props", "id": 5, "set": ["accessibilityElementsHidden": "true"]]]))
        try note("aria-hidden")
        p.apply(wireBatch([["op": "props", "id": 5, "set": ["inert": "true", "accessibilityElementsHidden": "false"]]]))
        try note("column inert")
        p.apply(wireBatch([["op": "props", "id": 1, "set": ["inert": "true"]]]))
        try note("root inert")
        p.apply(wireBatch([["op": "props", "id": 5, "set": ["inert": "false"]], ["op": "props", "id": 1, "set": ["inert": "false"]]]))
        p.focusElement(["middle"])
        try note("inert removed")
        p.apply(wireBatch([["op": "props", "id": 6, "set": ["disabled": "true"]]]))
        try note("field disabled")
        // A view AppKit hides is no batch: the presenter's refresh is what asks again.
        try XCTUnwrap(p.views[5]).isHidden = true
        p.inputGates.refresh(Array(p.views.values))
        try note("column hidden")
        return seen
    }
    /// A `dialog` shown modally is on the core's top-layer stack, as a sheet
    /// or painted, the same with navigation or without it: the page behind
    /// it blocked, its field focused, and all of it given back as it closes.
    func testADialogOpensAndClosesTheSameWithoutNavigation() throws {
        for painted in [true, false] {
            let unlinked = try dialog(linked: false, painted: painted)
            XCTAssertEqual(unlinked, try dialog(linked: true, painted: painted))
            XCTAssertEqual(unlinked, ["open #3 sheet \(!painted) blocks page true button inert true focus field true in page false",
                                      "open none sheet false blocks page false button inert false focus field false in page true"], "painted \(painted)")
        }
    }

    private func dialog(linked: Bool, painted: Bool) throws -> [String] {
        let make = NavigationLink.make, sidebar = NavigationLink.makeSidebar, swipe = NavigationLink.makeBackSwipe
        if !linked { NavigationLink.make = nil; NavigationLink.makeSidebar = nil; NavigationLink.makeBackSwipe = nil }
        defer { NavigationLink.make = make; NavigationLink.makeSidebar = sidebar; NavigationLink.makeBackSwipe = swipe }
        _ = NSApplication.shared
        let p = Presenter()
        XCTAssertEqual(p.navigation is NoNavigation, !linked)
        p.dialogs.painted = painted
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 500, height: 400), styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        self.window = window
        window.contentView = p.viewport
        window.makeKeyAndOrderFront(nil)
        p.apply(wireBatch([
            ["op": "create", "id": 1, "kind": "view"],
            ["op": "create", "id": 2, "kind": "button", "props": ["commandfor": "form", "command": "show-modal"]],
            ["op": "create", "id": 3, "kind": "view", "props": ["semanticTag": "dialog", "id": "form", "closedby": "any"]],
            ["op": "create", "id": 4, "kind": "input", "props": ["value": "draft", "autofocus": "true"]],
            ["op": "children", "id": 1, "ids": [2, 3]], ["op": "children", "id": 3, "ids": [4]], ["op": "roots", "ids": [1]],
            ["op": "frame", "id": 1, "x": 0, "y": 0, "w": 500, "h": 400],
            ["op": "frame", "id": 2, "x": 10, "y": 10, "w": 80, "h": 30],
            ["op": "frame", "id": 3, "x": 0, "y": 0, "w": 240, "h": 150],
            ["op": "frame", "id": 4, "x": 10, "y": 10, "w": 200, "h": 30],
        ]))
        let page = try XCTUnwrap(p.views[1]), button = try XCTUnwrap(p.views[2]), dialog = try XCTUnwrap(p.views[3]), field = try XCTUnwrap(p.views[4])
        func focused() -> Bool {
            p.focusedNode === field || (field.window?.firstResponder as? NSView).map { $0 === field || $0.isDescendant(of: field) || field.field?.currentEditor() === $0 } == true
        }
        func settle(_ done: () -> Bool) {
            let deadline = Date().addingTimeInterval(5)
            while !done(), Date() < deadline { RunLoop.main.run(mode: .default, before: Date().addingTimeInterval(0.02)) }
        }
        func note() -> String {
            let sheet = p.dialogs.entries.last?.sheet.map { $0.window.isVisible } ?? false
            return "open \(p.dialogs.active.map { "#\($0.id)" } ?? "none") sheet \(sheet) blocks page \(p.dialogs.blocks(button)) button inert \(button.inert) focus field \(focused()) in page \(dialog.superview === page.container || dialog.superview === page)"
        }
        var seen: [String] = []
        p.dialogs.show(dialog)
        settle { p.dialogs.active === dialog && focused() && (painted || p.dialogs.entries.last?.sheet?.window.isVisible == true) }
        seen.append(note())
        p.dialogs.close(dialog)
        settle { p.dialogs.active == nil && (dialog.superview === page.container || dialog.superview === page) }
        seen.append(note())
        return seen
    }
}
#endif
