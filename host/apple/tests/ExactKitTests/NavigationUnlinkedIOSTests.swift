#if os(iOS)
import UIKit
import XCTest
@testable import ExactKit
@testable import ExactCollections

/// LLP 1047.001 D10: a composition without navigation holds `NoNavigation`
/// and `NoModals`, and a page with no route observes at each point of its
/// life what it observes with navigation linked: mounted, after a batch,
/// unmounted and mounted again, and a list's scroll fills its rows alike.
final class NavigationUnlinkedIOSTests: XCTestCase {
    func testAPageWithoutRoutesObservesTheSameWithoutNavigation() throws {
        let unlinked = try lifecycle(linked: false, family: "navigation")
        let linked = try lifecycle(linked: true, family: "navigation")
        XCTAssertEqual(unlinked, linked)
        XCTAssertEqual(linked.map { $0.contains("count presses 1") }, [false, true, true, true], "the press ran")
    }

    /// The same page without the dialogs family (`NoDialogs`): no sheet is
    /// presented, owned or a parent at any point.
    func testAPageWithoutDialogsObservesTheSameWithoutTheDialogsFamily() throws {
        let unlinked = try lifecycle(linked: false, family: "dialogs")
        XCTAssertEqual(unlinked, try lifecycle(linked: true, family: "dialogs"))
        XCTAssertTrue(unlinked.allSatisfy { $0.hasSuffix("sheets 0 0 none") }, unlinked.joined(separator: "\n"))
        XCTAssertEqual(unlinked.map { $0.contains("count presses 1") }, [false, true, true, true], "the press ran")
    }

    private func lifecycle(linked: Bool, family: String) throws -> [String] {
        let path = try XCTUnwrap(ProcessInfo.processInfo.environment["EXACT_BUTTONS_PLAN"])
        let make = NavigationLink.make, makeModals = NavigationLink.makeModals, makeDialogs = DialogLink.make
        if !linked, family == "navigation" { NavigationLink.make = nil; NavigationLink.makeModals = nil }
        if !linked, family == "dialogs" { DialogLink.make = nil }
        defer { NavigationLink.make = make; NavigationLink.makeModals = makeModals; DialogLink.make = makeDialogs }
        let session = ExactApp.shared.makeSession(label: "\(family)-\(linked ? "linked" : "unlinked")")
        defer { session.destroy() }
        let p = session.presenter
        if family == "navigation" {
            XCTAssertEqual(p.navigation is NoNavigation, !linked)
            XCTAssertEqual(p.modals is NoModals, !linked)
        } else {
            XCTAssertEqual(p.dialogs is NoDialogs, !linked)
        }

        let view = ExactView(session: session)
        let host = UIViewController()
        let scene = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }.first
        let window = scene.map { UIWindow(windowScene: $0) } ?? UIWindow(frame: .zero)
        window.frame = CGRect(x: 0, y: 0, width: 400, height: 850)
        window.rootViewController = host
        window.makeKeyAndVisible()
        defer { window.isHidden = true }
        host.view.addSubview(view)
        XCTAssertNil(session.boot(plan: try Data(contentsOf: URL(fileURLWithPath: path)), size: window.bounds.size).error)
        view.frame = host.view.bounds

        var seen: [String] = []
        func observe(_ point: String) {
            window.layoutIfNeeded()
            RunLoop.main.run(until: Date().addingTimeInterval(0.2))
            let count = p.views.values.first { $0.props["testId"] == "count" }
            seen.append([
                point,
                "count \(count?.accessibilityLabel ?? "none")",
                "shown \(count?.window != nil)",
                "document in viewport \(p.root.superview === p.viewport)",
                "syncOwed \(p.navigation.syncOwed)",
                "defersFocus \(p.navigation.defersFocus)",
                "pendingFocus \(p.pendingFocusNode != nil)",
                "blocks \(p.owners.told.count)",
                "scrollsToTop \(scrollers(in: p.viewport).filter(\.scrollsToTop).count)",
                "surfaces \(p.modals.surfaces(includingRetiring: true).count)",
                "dialog sheets own \(count.map(p.dialogs.owns) ?? false) parent \(count.flatMap(p.dialogs.authoredParent) != nil) shows \(count.map(p.dialogs.shows) ?? false) surfaces \(p.dialogs.surfaces(includingRetiring: true).count) resume \(p.dialogs.resumeAfterRouteDismissal())",
                "dialog top \(p.dialogs.top != nil) moving \(p.dialogs.inTransition) press lifetime \(count.flatMap(p.dialogs.pressLifetime) != nil) keeps focus \(count.map { p.dialogs.preservesFocus($0.id) } ?? false)",
                "sheets \(p.dialogs.presented.count) \(p.dialogs.containingBlocks.count) \(p.dialogs.topDialog.map { "#\($0.id)" } ?? "none")",
            ].joined(separator: ", "))
        }

        observe("mounted")
        let go = try XCTUnwrap(p.views.values.first { $0.props["testId"] == "sign-in" })
        p.press(go.id)
        observe("after a batch")
        view.removeFromSuperview()
        observe("unmounted")
        host.view.addSubview(view)
        observe("mounted again")
        return seen
    }

    /// A virtualized list's scroll fills rows in rows-only batches: they take
    /// the row-fill shortcut once a sync has settled the page, linked or not.
    func testAListScrollFillsRowsTheSameWithoutNavigation() throws {
        let unlinked = try scroll(linked: false)
        XCTAssertEqual(unlinked, try scroll(linked: true))
        XCTAssertTrue(unlinked.hasPrefix("rows built true"), unlinked)
    }

    private func scroll(linked: Bool) throws -> String {
        let make = NavigationLink.make, makeModals = NavigationLink.makeModals
        if !linked { NavigationLink.make = nil; NavigationLink.makeModals = nil }
        defer { NavigationLink.make = make; NavigationLink.makeModals = makeModals }
        let session = ExactApp.shared.makeSession(label: "navigation-rows-\(linked ? "linked" : "unlinked")")
        session.clock = 0
        let window = UIWindow(frame: CGRect(x: 0, y: 0, width: 400, height: 240))
        let host = UIViewController(), view = ExactView(session: session)
        window.rootViewController = host
        window.makeKeyAndVisible()
        host.view.addSubview(view)
        view.frame = window.bounds
        defer { session.destroy(); window.isHidden = true }
        let path = try XCTUnwrap(ProcessInfo.processInfo.environment["EXACT_LAYOUT_ONLY_ROW_PLAN"])
        XCTAssertNil(session.boot(plan: try Data(contentsOf: URL(fileURLWithPath: path)), size: window.bounds.size).error)
        view.layoutIfNeeded()
        let p = session.presenter
        XCTAssertEqual(p.navigation is NoNavigation, !linked)
        p.listHost.settle()
        session.drainFill()
        let scroll = try XCTUnwrap(p.views.values.first { $0.props["testId"] == "list" }?.scroll)
        var hits = 0, owed = 0
        session.onApplyForTests = { batch in
            hits += batch.ops.filter { $0.op == .layoutHits }.count
            if p.navigation.syncOwed { owed += 1 }
        }
        for y in [300, 600, 900, 1200, 900, 600, 300, 0] {
            scroll.contentOffset.y = CGFloat(y)
            p.listHost.settle()
            session.drainFill()
        }
        session.onApplyForTests = nil
        return "rows built \(hits > 0), sync owed \(owed), dependencies kept \(p.listRowDependencies != nil)"
    }

    private func scrollers(in view: UIView) -> [UIScrollView] {
        ((view as? UIScrollView).map { [$0] } ?? []) + view.subviews.flatMap(scrollers(in:))
    }
}
#endif
