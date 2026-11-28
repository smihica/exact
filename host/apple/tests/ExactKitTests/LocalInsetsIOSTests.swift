#if os(iOS)
import UIKit
import XCTest
@testable import ExactKit
@testable import ExactNavigation

/// LLP 1123 D9 over `scripts/fixtures/bar-insets.contract`: a route's content
/// reads its controller's safe area, the navigation bar's height in it, as
/// `var(--exact-safe-area-inset-*)`, so a background reaches up under the bar
/// without a hard-coded height (expo/exact#36), on an iPhone and an iPad
/// alike; a box outside every route reads the screen's insets, and a sheet's
/// content its sheet's. And over `scripts/fixtures/bar-hero.contract`, a
/// route whose scroller starts at its top opts that edge out of its
/// clearance (D4), so the scroller's first box reaches under the bar. And
/// over `scripts/fixtures/bar-control.contract`, a box between the bar and
/// the scroller is content: the scroller gets no inset for it.
final class LocalInsetsIOSTests: XCTestCase {
    private var window: UIWindow?
    private var sessions: [ExactSession] = []

    override func tearDown() {
        for session in sessions { session.destroy() }
        sessions = []
        window?.isHidden = true
        window = nil
        super.tearDown()
    }

    private func until(_ what: String, _ seconds: Double = 5, _ done: () -> Bool) {
        let deadline = Date().addingTimeInterval(seconds)
        while !done(), Date() < deadline { RunLoop.main.run(mode: .default, before: Date().addingTimeInterval(0.02)) }
        XCTAssertTrue(done(), what)
    }

    /// The fixture in a window of the test host's scene, the screen's size.
    private func fixture(_ scene: UIWindowScene, _ key: String = "BAR_INSETS") throws -> (ExactSession, ExactView) {
        let session = ExactApp.shared.makeSession(label: "bar-insets")
        sessions.append(session)
        let view = ExactView(session: session)
        let host = UIViewController()
        let window = UIWindow(windowScene: scene)
        window.frame = scene.screen.bounds
        window.rootViewController = host
        window.makeKeyAndVisible()
        host.view.addSubview(view)
        view.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        self.window = window
        XCTAssertNil(session.boot(plan: try fixturePlan(key), size: window.bounds.size).error)
        view.frame = host.view.bounds
        view.layoutIfNeeded()
        return (session, view)
    }

    private func node(_ session: ExactSession, _ testId: String) -> NodeView? {
        session.presenter.views.values.first { $0.props["testId"] == testId && $0.window != nil }
    }

    private func onScreen(_ view: UIView) -> CGRect { view.convert(view.bounds, to: nil) }

    func testATransparentBarsContentReadsItsLocalInsetWhileTheDocumentInsetIsZero() throws {
        guard let scene = hostedWindowScene() else { throw missing("scene", "a transparent bar requires UIKit containment") }
        let (session, _) = try fixture(scene, "TRANSPARENT_BAR")
        let p = session.presenter
        waitFor("the transparent route appears") {
            guard let route = self.node(session, "route-home"), let c = p.navigationHost.controllers[route.id] else { return false }
            return c.viewIfLoaded?.window != nil && c.view.safeAreaInsets.top > 0 && !p.navigationHost.inTransition
        }
        let route = try XCTUnwrap(node(session, "route-home"))
        let controller = try XCTUnwrap(p.navigationHost.controllers[route.id])
        let scroll = try XCTUnwrap(node(session, "home-scroll")?.scrollView)
        XCTAssertEqual(scroll.contentInsetAdjustmentBehavior, .never)
        waitFor("the hero's control clears the bar using the route's inset", facts: {
            "control=\(self.node(session, "hero-control").map(self.onScreen) ?? .zero), hero=\(self.node(session, "hero").map(self.onScreen) ?? .zero), safe=\(controller.view.safeAreaInsets), local=\(p.navigationHost.localInsets.wanted[route.id] ?? []), offset=\(scroll.contentOffset)"
        }) {
            guard let control = self.node(session, "hero-control") else { return false }
            return abs(self.onScreen(control).minY - controller.view.safeAreaInsets.top) < 0.5
        }
        XCTAssertGreaterThan(controller.view.safeAreaInsets.top, try XCTUnwrap(window).safeAreaInsets.top)
        XCTAssertEqual(onScreen(try XCTUnwrap(node(session, "hero"))).minY, 0, accuracy: 0.5)
        XCTAssertEqual(try XCTUnwrap(node(session, "document-inset")).bounds.height, 0)
        XCTAssertEqual(p.insets.top, 0, "env() remains the document's inset under auto")
    }

    func testARoutesContentReadsItsBarsAndABackgroundReachesUnderThem() throws {
        guard let scene = UIApplication.shared.connectedScenes.first as? UIWindowScene else {
            throw missing("scene", "the bars and the safe area are a scene window's")
        }
        let (session, view) = try fixture(scene)
        until("the route shows under its bar") {
            guard let route = node(session, "route"), let c = session.presenter.navigationHost.controllers[route.id] else { return false }
            return c.viewIfLoaded?.window != nil && c.view.safeAreaInsets.top > view.safeAreaInsets.top
        }
        let route = try XCTUnwrap(node(session, "route"))
        let c = try XCTUnwrap(session.presenter.navigationHost.controllers[route.id])
        let bars = c.view.safeAreaInsets
        until("the content reads the route's safe area") {
            guard let first = self.node(session, "sidebar-first") else { return false }
            return abs(self.onScreen(first).minY - bars.top) < 0.5
        }
        let sidebar = try XCTUnwrap(node(session, "sidebar")), list = try XCTUnwrap(node(session, "list"))
        XCTAssertEqual(onScreen(sidebar).minY, onScreen(route).minY, accuracy: 0.5, "the sidebar's background starts at the route's top, under the bar")
        XCTAssertEqual(onScreen(sidebar).minY, 0, accuracy: 0.5, "which is the screen's")
        let first = try XCTUnwrap(node(session, "sidebar-first"))
        XCTAssertEqual(onScreen(first).minY, bars.top, accuracy: 0.5, "its content starts below the bar: \(bars.top) points, read, not written")
        XCTAssertEqual(onScreen(list).minY, bars.top, accuracy: 0.5, "the list beside it already cleared the bar and stays")
        let absolute = try XCTUnwrap(node(session, "absolute-probe"))
        XCTAssertEqual(onScreen(absolute).minY, bars.top, accuracy: 0.5, "the absolute child's local inset clears the route's cover once")
        let probe = try XCTUnwrap(node(session, "root-probe"))
        XCTAssertEqual(probe.bounds.height, view.safeAreaInsets.top, accuracy: 0.5, "outside every route, the screen's top inset, though env() is 0")
        XCTAssertEqual(session.presenter.insets.top, 0, "env() stays the document's")
    }

    func testASheetsContentReadsTheSheetsInsets() throws {
        guard let scene = UIApplication.shared.connectedScenes.first as? UIWindowScene else {
            throw missing("scene", "the bars and the safe area are a scene window's")
        }
        let (session, _) = try fixture(scene)
        until("the route shows") { self.node(session, "open") != nil }
        _ = Agent(session: session).tap(["id": Int(try XCTUnwrap(node(session, "open")).id)])
        until("the sheet is presented") {
            guard let sheet = self.node(session, "sheet"), let c = session.presenter.navigationHost.controllers[sheet.id] else { return false }
            return c.presentingViewController != nil && c.viewIfLoaded?.window != nil
        }
        let sheet = try XCTUnwrap(node(session, "sheet"))
        let c = try XCTUnwrap(session.presenter.navigationHost.controllers[sheet.id])
        until("the probe reads the sheet's own bottom inset") {
            guard let probe = self.node(session, "sheet-probe") else { return false }
            return abs(probe.bounds.height - c.view.safeAreaInsets.bottom) < 0.5
        }
    }

    /// The sides a sheet's content reads while the sheet moves in are the ones it lands with, so nothing in it moves
    /// as it lands. Only a screen with a side inset (the iPhone Duo's button strip) can fail it: elsewhere both are 0.
    func testASheetsContentReadsTheSidesItLandsWithFromItsFirstFrame() throws {
        guard let scene = UIApplication.shared.connectedScenes.first as? UIWindowScene else {
            throw missing("scene", "the bars and the safe area are a scene window's")
        }
        let (session, _) = try fixture(scene)
        let navigation = session.presenter.navigationHost
        until("the route shows") { self.node(session, "open") != nil }
        _ = Agent(session: session).tap(["id": Int(try XCTUnwrap(node(session, "open")).id)])
        var first: [CGFloat]?
        until("the sheet's first insets are reported") {
            first = self.node(session, "sheet").flatMap { navigation.localInsets.wanted[$0.id] }
            return first != nil
        }
        let sheet = try XCTUnwrap(node(session, "sheet"))
        let c = try XCTUnwrap(navigation.controllers[sheet.id])
        until("the sheet lands") { c.presentingViewController != nil && c.transitionCoordinator == nil }
        let box = sheet.convert(sheet.bounds, to: c.surface), safe = c.surface.safeAreaInsets
        let reported = try XCTUnwrap(first)
        XCTAssertEqual(reported[1], max(0, safe.right - max(0, c.surface.bounds.maxX - box.maxX)), accuracy: 0.5, "right")
        XCTAssertEqual(reported[3], max(0, safe.left - max(0, box.minX)), accuracy: 0.5, "left")
    }

    /// A full-screen route reads its landed top and bottom insets from its first frame and keeps them steady
    /// across its animated presentation and dismissal, both barless (expo/exact#2126) and with a native
    /// navigation bar, while preserving route-owned `additionalSafeAreaInsets`.
    func testAFullScreenRouteReadsItsTopInsetFromItsFirstFrameThroughPresentationAndDismissal() throws {
        guard let scene = UIApplication.shared.connectedScenes.first as? UIWindowScene else {
            throw missing("scene", "a full-screen presentation requires a window scene")
        }
        let (session, view) = try fixture(scene)
        let navigation = session.presenter.navigationHost
        until("the home route shows") { self.node(session, "open-cover") != nil }
        try XCTSkipUnless(view.safeAreaInsets.top > 1, "simulator reports no nonzero status-bar top inset")
        let openTap = Agent(session: session).tap(["id": Int(try XCTUnwrap(node(session, "open-cover")).id)])
        XCTAssertNil(openTap["error"], "open-cover tap succeeded: \(openTap)")

        var first: [CGFloat]?
        var sawInFlight = false
        var observedTops: [CGFloat] = []
        var observedBarHeights: [CGFloat] = []
        let deadline = Date().addingTimeInterval(5)
        var coverNode: NodeView?
        var coverController: RouteController?
        while Date() < deadline {
            if let cover = node(session, "cover"), let c = navigation.controllers[cover.id] {
                coverNode = cover
                coverController = c
                let inFlight = c.inPresentation || c.transitionCoordinator != nil
                if let insets = navigation.localInsets.wanted[cover.id] {
                    if first == nil { first = insets }
                    observedTops.append(insets[0])
                    if inFlight { sawInFlight = true }
                }
                if let bar = node(session, "cover-bar"), bar.bounds.height > 0 {
                    observedBarHeights.append(bar.bounds.height)
                }
                if c.presentingViewController != nil && !inFlight { break }
            }
            RunLoop.main.run(mode: .default, before: Date().addingTimeInterval(0.005))
        }
        let cover = try XCTUnwrap(coverNode, "the full-screen cover appeared")
        let c = try XCTUnwrap(coverController, "the full-screen cover has a controller")
        XCTAssertTrue(c.presentingViewController != nil && c.transitionCoordinator == nil && !c.inPresentation, "the full-screen cover landed")
        let landedTop = c.surface.safeAreaInsets.top
        let landedBottom = c.surface.safeAreaInsets.bottom
        XCTAssertGreaterThan(landedTop, 1, "the full-screen cover sits under the status bar")
        XCTAssertTrue(sawInFlight, "the test must observe the cover while its animated presentation is in flight")
        let reported = try XCTUnwrap(first, "the cover reported local insets during presentation")
        XCTAssertEqual(reported[0], landedTop, accuracy: 0.5, "top inset matches from the very first frame")
        XCTAssertEqual(reported[2], landedBottom, accuracy: 0.5, "bottom inset matches from the very first frame")
        for top in observedTops {
            XCTAssertEqual(top, landedTop, accuracy: 0.5, "top inset never dipped during presentation: \(observedTops)")
        }
        until("the cover bar settled at 44 + top inset") {
            guard let bar = self.node(session, "cover-bar") else { return false }
            return abs(bar.bounds.height - (44 + landedTop)) < 0.5
        }
        XCTAssertFalse(observedBarHeights.isEmpty, "the cover bar rendered during presentation")
        for h in observedBarHeights {
            XCTAssertEqual(h, 44 + landedTop, accuracy: 0.5, "bar height never slid in under the status bar: \(observedBarHeights)")
        }

        // Route-owned additionalSafeAreaInsets must be preserved by landingSafeArea.
        c.additionalSafeAreaInsets.left = 20
        defer { c.additionalSafeAreaInsets.left = 0 }
        until("route-owned additionalSafeAreaInsets reaches local insets") {
            abs((navigation.localInsets.wanted[cover.id]?[3] ?? 0) - 20) < 0.5
        }
        c.additionalSafeAreaInsets.left = 0

        // Retain the enclosing presented controller so we verify UIKit actually completes dismissal.
        var presentedModal: UIViewController = c
        while let parent = presentedModal.parent { presentedModal = parent }
        let closeTap = Agent(session: session).tap(["id": Int(try XCTUnwrap(node(session, "close-cover")).id)])
        XCTAssertNil(closeTap["error"], "close-cover tap succeeded: \(closeTap)")
        var dismissTops: [CGFloat] = []
        let dismissDeadline = Date().addingTimeInterval(5)
        var dismissed = false
        while Date() < dismissDeadline {
            if let insets = navigation.localInsets.wanted[cover.id] {
                dismissTops.append(insets[0])
            }
            if presentedModal.presentingViewController == nil && presentedModal.viewIfLoaded?.window == nil {
                dismissed = true
                break
            }
            RunLoop.main.run(mode: .default, before: Date().addingTimeInterval(0.005))
        }
        XCTAssertTrue(dismissed, "the full-screen cover's presented modal completed dismissal")
        for top in dismissTops {
            XCTAssertEqual(top, landedTop, accuracy: 0.5, "top inset stays steady during dismissal: \(dismissTops)")
        }

        // A full-screen route with a native navigation bar must include both status bar and bar height from frame 0,
        // and preserve both route-owned and ancestor-owned `additionalSafeAreaInsets` (with UIKit's floor) in flight.
        until("home is back on screen") { self.node(session, "open-cover-nav") != nil }
        let navTap = Agent(session: session).tap(["id": Int(try XCTUnwrap(node(session, "open-cover-nav")).id)])
        XCTAssertNil(navTap["error"], "open-cover-nav tap succeeded: \(navTap)")
        var navFirst: [CGFloat]?
        var navObservedTops: [CGFloat] = []
        var navSawInFlight = false
        var navInFlightFloorDelta: CGFloat?
        var navInFlightRouteDelta: CGFloat?
        var navInFlightNegTop: CGFloat?
        var navController: RouteController?
        let navDeadline = Date().addingTimeInterval(5)
        while Date() < navDeadline {
            if let navCover = node(session, "cover-nav"), let nc = navigation.controllers[navCover.id] {
                navController = nc
                let inFlight = nc.inPresentation || nc.transitionCoordinator != nil
                // Record the unaltered first report before probing `additionalSafeAreaInsets`.
                if let insets = navigation.localInsets.wanted[navCover.id] {
                    if navFirst == nil { navFirst = insets }
                    navObservedTops.append(insets[0])
                    if inFlight { navSawInFlight = true }
                }
                if inFlight && navFirst != nil && navInFlightFloorDelta == nil, let nav = nc.navigationController {
                    let baseLeft = navigation.localInsets.wanted[navCover.id]?[3] ?? 0
                    // Ancestor +20 with child -20 must keep the ancestor's 20 (UIKit inherited floor).
                    nav.additionalSafeAreaInsets.left = 20
                    nc.additionalSafeAreaInsets.left = -20
                    _ = navigation.reportLocalInsets()
                    navInFlightFloorDelta = (navigation.localInsets.wanted[navCover.id]?[3] ?? 0) - baseLeft
                    // Ancestor +20 with child +12 must add to +32.
                    nc.additionalSafeAreaInsets.left = 12
                    _ = navigation.reportLocalInsets()
                    navInFlightRouteDelta = (navigation.localInsets.wanted[navCover.id]?[3] ?? 0) - baseLeft
                    // Negative child top inset reduces nav-bar clearance down to the inherited status-bar floor.
                    nc.additionalSafeAreaInsets.top = -200
                    navInFlightNegTop = NavigationHost.landingSafeArea(nc)?.top
                    nav.additionalSafeAreaInsets.left = 0
                    nc.additionalSafeAreaInsets.left = 0
                    nc.additionalSafeAreaInsets.top = 0
                    _ = navigation.reportLocalInsets()
                }
                if nc.presentingViewController != nil && !inFlight { break }
            }
            RunLoop.main.run(mode: .default, before: Date().addingTimeInterval(0.005))
        }
        let nc = try XCTUnwrap(navController, "the bar-bearing full-screen cover appeared")
        XCTAssertTrue(nc.presentingViewController != nil && nc.transitionCoordinator == nil && !nc.inPresentation, "cover-nav landed")
        let navLandedTop = nc.surface.safeAreaInsets.top
        XCTAssertGreaterThan(navLandedTop, view.safeAreaInsets.top + 20, "bar-bearing cover includes status bar + nav bar")
        XCTAssertTrue(navSawInFlight, "observed cover-nav in flight")
        XCTAssertEqual(try XCTUnwrap(navInFlightFloorDelta, "observed ancestor inset with negative child"), 20, accuracy: 0.5)
        XCTAssertEqual(try XCTUnwrap(navInFlightRouteDelta, "observed ancestor + route inset"), 32, accuracy: 0.5)
        XCTAssertEqual(try XCTUnwrap(navInFlightNegTop, "negative child top inset floors at inherited status bar"), view.safeAreaInsets.top, accuracy: 0.5)
        let navReported = try XCTUnwrap(navFirst)
        XCTAssertEqual(navReported[0], navLandedTop, accuracy: 0.5, "bar-bearing cover top inset matches from frame 0")
        for top in navObservedTops {
            XCTAssertEqual(top, navLandedTop, accuracy: 0.5, "bar-bearing cover top inset never jumped: \(navObservedTops)")
        }

        // Dismiss cover-nav via its bar button and verify its presented modal leaves the window.
        var navModal: UIViewController = nc
        while let parent = navModal.parent { navModal = parent }
        let doneTap = Agent(session: session).tap(["id": Int(try XCTUnwrap(node(session, "close-cover-nav")).id)])
        XCTAssertNil(doneTap["error"], "close-cover-nav tap succeeded: \(doneTap)")
        until("the bar-bearing full-screen cover's presented modal completed dismissal") {
            navModal.presentingViewController == nil && navModal.viewIfLoaded?.window == nil
        }
    }

    /// A route coming back in a pop reads the sides it returns to while the pop runs, as UIKit gives them to it, not the
    /// ones it left with, and keeps its top and bottom, which the bars' transition moves, until the pop ends.
    func testARouteComingBackInAPopReadsItsNewSidesDuringThePop() throws {
        guard let scene = UIApplication.shared.connectedScenes.first as? UIWindowScene else {
            throw missing("scene", "the bars and the safe area are a scene window's")
        }
        let (session, _) = try fixture(scene)
        let navigation = session.presenter.navigationHost
        until("the route shows") { self.node(session, "more") != nil && self.node(session, "left-probe") != nil }
        let route = try XCTUnwrap(node(session, "route")), probe = try XCTUnwrap(node(session, "left-probe"))
        let c = try XCTUnwrap(navigation.controllers[route.id]), nav = try XCTUnwrap(c.navigationController)
        until("the first route's insets are reported") { navigation.localInsets.wanted[route.id] != nil }
        let before = try XCTUnwrap(navigation.localInsets.wanted[route.id]), width = probe.bounds.width
        _ = Agent(session: session).tap(["id": Int(try XCTUnwrap(node(session, "more")).id)])
        until("the pushed route lands, the first out of the window") {
            nav.viewControllers.count == 2 && nav.transitionCoordinator == nil && c.viewIfLoaded?.window == nil
        }
        // The pose changes while the first route is away: a side inset it has not seen
        let host = try XCTUnwrap(window?.rootViewController)
        host.additionalSafeAreaInsets.left += 20
        window?.layoutIfNeeded()
        XCTAssertEqual(navigation.localInsets.wanted[route.id], before, "away, it keeps what it had")
        nav.popViewController(animated: true)
        var during: (width: CGFloat, top: CGFloat?)?, timeline: [String] = []
        until("the pop lands") {
            let moving = nav.transitionCoordinator != nil, shown = c.viewIfLoaded?.window != nil
            timeline.append("moving \(moving) shown \(shown) left \(c.surface.safeAreaInsets.left) probe \(probe.bounds.width)")
            if moving, shown, during == nil, abs(probe.bounds.width - width) > 0.5 {
                during = (probe.bounds.width, navigation.localInsets.wanted[route.id]?[0])
            }
            return !moving && nav.viewControllers.count == 1 && timeline.count > 2
        }
        let read = try XCTUnwrap(during, "the content read no new side before the pop landed: \(timeline.suffix(8).joined(separator: " | "))")
        XCTAssertEqual(read.width, width + 20, accuracy: 0.5, "the content reads the left side it returns to while the pop runs")
        XCTAssertEqual(read.top ?? -1, before[0], accuracy: 0.5, "its top holds until the pop ends")
    }

    func testAScrollerAtTheTopOptsOutAndItsFirstBoxReachesUnderTheBar() throws {
        guard let scene = UIApplication.shared.connectedScenes.first as? UIWindowScene else {
            throw missing("scene", "the bars and the safe area are a scene window's")
        }
        try checkTopClearance(in: scene, plan: "BAR_HERO")
        try checkTopClearance(in: scene, plan: "ROUTE_SCROLL_EDGES")
    }

    private func checkTopClearance(in scene: UIWindowScene, plan: String) throws {
        let (session, _) = try fixture(scene, plan)
        until("the route shows under its bar") {
            guard let route = self.node(session, "route"), let c = session.presenter.navigationHost.controllers[route.id] else { return false }
            return c.viewIfLoaded?.window != nil && c.view.safeAreaInsets.top > 0 && c.collapseScroll != nil
        }
        let route = try XCTUnwrap(node(session, "route"))
        let c = try XCTUnwrap(session.presenter.navigationHost.controllers[route.id])
        let list = try XCTUnwrap(node(session, "list")), sv = try XCTUnwrap(list.scrollView)
        XCTAssertEqual(sv.contentInsetAdjustmentBehavior, .never, "UIKit insets nothing of the top")
        let top = c.view.safeAreaInsets.top
        until("the hero's text clears the bar it reads") {
            guard let text = self.node(session, "hero-text") else { return false }
            return abs(self.onScreen(text).minY - top) < 0.5
        }
        let hero = try XCTUnwrap(node(session, "hero"))
        XCTAssertEqual(onScreen(hero).minY, 0, accuracy: 0.5, "the hero's background starts at the screen's top, under the bar")
        XCTAssertEqual(sv.contentOffset.y + sv.adjustedContentInset.top, 0, accuracy: 0.5, "at its start")
        if case .edges(let edges)? = session.presenter.navigationHost.covers[route.id] { XCTAssertEqual(edges.top, 0, "no top cover") }
        guard plan == "ROUTE_SCROLL_EDGES" else { return }
        let tabs = try XCTUnwrap(session.presenter.navigationHost.tabController)
        XCTAssertEqual(onScreen(sv).maxY, onScreen(tabs.view).maxY, accuracy: 0.5)
        XCTAssertEqual(sv.adjustedContentInset.bottom, c.view.safeAreaInsets.bottom, accuracy: 0.5)
        // The tab bar adds its height where UIKit sets it along the bottom; an iPhone Duo sets it in its vertical bar,
        // which adds to the side instead
        if c.traitCollection.placesVerticalBar {
            XCTAssertEqual(sv.adjustedContentInset.bottom, try XCTUnwrap(window).safeAreaInsets.bottom, accuracy: 0.5)
        } else {
            XCTAssertGreaterThan(sv.adjustedContentInset.bottom, try XCTUnwrap(window).safeAreaInsets.bottom)
        }
        let bar = try XCTUnwrap(c.navigationController?.navigationBar)
        sv.contentInsetAdjustmentBehavior = .automatic
        session.presenter.navigationHost.selectScrollEdges(of: c)
        XCTAssertEqual(sv.contentInsetAdjustmentBehavior, .never)
        XCTAssertEqual(sv.adjustedContentInset.top, 0, accuracy: 0.5)
        let expanded = bar.bounds.height
        sv.setContentOffset(CGPoint(x: 0, y: 600), animated: false)
        // An iPhone Duo's bar shows no large title to collapse: UIKit lays it out at its inline height (58 points on
        // the cover display, measured before and after this scroll)
        guard !c.traitCollection.placesVerticalBar else { return }
        waitFor("scrolling collapses the large title") { bar.bounds.height < expanded - 1 }
    }

    func testBottomClearanceLeavesAPulledScrollOffsetAlone() {
        let presenter = Presenter()
        let scroll = UIScrollView(frame: CGRect(x: 0, y: 0, width: 300, height: 600))
        scroll.contentSize = CGSize(width: 300, height: 2400)
        scroll.contentInsetAdjustmentBehavior = .never
        scroll.setContentOffset(CGPoint(x: 0, y: -40), animated: false)
        let inset = ClearanceInset(scroll: scroll, unadjustedBottom: 83)
        presenter.navigationHost.applyClearance([ObjectIdentifier(scroll): inset])
        XCTAssertEqual(scroll.contentInset.bottom, 83)
        XCTAssertEqual(scroll.contentOffset.y, -40)
        presenter.navigationHost.applyClearance([:])
        XCTAssertEqual(scroll.contentOffset.y, -40)
    }

    func testRouteScrollInsetsMatchUIKitOnEachEdge() throws {
        guard let scene = hostedWindowScene() else { throw missing("scene", "the tab and navigation bars need a scene") }
        try checkScrollEdges(in: scene, cover: false)
    }

    func testCoverRouteGivesItsBottomInsetToTheScrollerThatReachesTheTabBar() throws {
        guard let scene = hostedWindowScene() else { throw missing("scene", "the tab bar needs a window") }
        let (session, _) = try fixture(scene, "COVER_SCROLL_EDGES")
        let route = try XCTUnwrap(session.presenter.navigationHost.activeRoute)
        let tabs = try XCTUnwrap(route.tabBarController)
        guard let bar = RouteGeometry.shownBar(tabs.tabBar), onScreen(bar).midY > scene.screen.bounds.midY else {
            throw XCTSkip("the fixture needs a bottom tab bar")
        }
        func press(_ id: String) throws {
            session.presenter.press(try XCTUnwrap(node(session, id)).id)
            session.presenter.settlePump()
        }
        let foot = onScreen(tabs.view).maxY, safe = onScreen(bar).minY
        try press("clipped")
        let clip = try XCTUnwrap(node(session, "clip"))
        let clipped = try XCTUnwrap(node(session, "clipped-list")?.scrollView)
        until("the last scroller and its clipping wrapper reach the tab bar together") {
            abs(self.onScreen(clip).maxY - foot) < 0.5 && abs(self.onScreen(clipped).maxY - foot) < 0.5
                && abs(clipped.adjustedContentInset.bottom - (foot - safe)) < 0.5
        }
        // Hiding the tab bar is iOS 18's (`setTabBarHidden`)
        if #available(iOS 18.0, *) {
            // With the bar hidden, the list holds the route's foot as UIKit's own does: the route pads nothing, and
            // the list, with no band left, is inset by the home indicator. The route opts its top out (`topOff`), so
            // that inset is its clearance, not UIKit's adjustment, which would inset its top too.
            let home = try XCTUnwrap(window).safeAreaInsets.bottom
            try press("tabs")
            waitFor("the separate list takes the foot, inset by the home indicator", facts: {
                "edge \(route.edgeScroll.map { $0.props["testId"] ?? "?" } ?? "none"), clip foot \(route.node.convert(ControlBars.laid(clip), from: clip.superview).maxY) of \(route.node.bounds.maxY), list \(self.onScreen(clipped)) to \(foot), inset \(clipped.adjustedContentInset) of \(clipped.contentInset), home \(home), noTop \(RouteGeometry.insetsNoTop(route.node)), band \(self.node(session, "clipped-list")?.pageBand.bottom ?? -1), tab bar \(self.onScreen(bar))"
            }) {
                route.edgeScroll === self.node(session, "clipped-list")
                    && abs(route.node.convert(ControlBars.laid(clip), from: clip.superview).maxY - route.node.bounds.maxY) < 0.5
                    && abs(self.onScreen(clipped).maxY - foot) < 0.5
                    && abs(clipped.adjustedContentInset.bottom - home) < 0.5
                    && (self.node(session, "clipped-list")?.pageBand.bottom ?? 1) < 0.5
            }
            XCTAssertEqual(clipped.contentInsetAdjustmentBehavior, .never, "UIKit adjusts none of it, the route having no top")
            XCTAssertEqual(clipped.contentInset.bottom, home, accuracy: 0.5, "the clearance is the home indicator's")
            try press("tabs")
        }
        let page = try XCTUnwrap(node(session, "list")?.scrollView)
        try press("footer")
        until("the footer keeps the list above the bar") { self.onScreen(page).maxY < safe && route.edgeScroll == nil }
        try press("padded")
        let padded = foot - (try XCTUnwrap(window)).safeAreaInsets.bottom
        until("only the band above the authored padding insets the list") {
            abs(self.onScreen(page).maxY - padded) < 0.5 && abs(page.adjustedContentInset.bottom - (padded - safe)) < 0.5
        }
        try press("short-route")
        until("a route ending above the bar gives back the bottom edge") { self.onScreen(page).maxY < safe && route.edgeScroll == nil }
        try press("top")
        try press("barless")
        until("a route without a navigation bar still runs under the tabs") {
            route.navigationController?.isNavigationBarHidden == true && abs(self.onScreen(page).maxY - foot) < 0.5
                && abs(page.adjustedContentInset.bottom - (foot - safe)) < 0.5
        }
    }

    func testCoverRouteWithoutTabsGivesItsBottomEdgeToItsScroller() throws {
        guard let scene = hostedWindowScene() else { throw missing("scene", "the home indicator needs a window") }
        let (session, _) = try fixture(scene, "COVER_SCROLL")
        let route = try XCTUnwrap(session.presenter.navigationHost.activeRoute)
        let list = try XCTUnwrap(node(session, "home-scroll"))
        let page = try XCTUnwrap(list.scrollView)
        // UIKit's own: a scroll view filling a large-title screen's view, which UIKit insets past the home indicator
        let reference = UIViewController()
        reference.title = "Home"
        let native = UIScrollView(frame: reference.view.bounds)
        native.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        native.contentSize = CGSize(width: 100, height: 2500)
        reference.view.addSubview(native)
        reference.setContentScrollView(native, for: .all)
        let stack = UINavigationController(rootViewController: reference)
        stack.navigationBar.prefersLargeTitles = true
        let oracle = UIWindow(windowScene: scene)
        oracle.rootViewController = stack
        oracle.isHidden = false
        defer { oracle.isHidden = true }
        waitFor("UIKit insets its own scroll view past the home indicator") {
            oracle.layoutIfNeeded()
            return native.adjustedContentInset.bottom > 0
        }
        waitFor("the route's list reaches the bottom edge with UIKit's inset", facts: {
            "list \(self.onScreen(page)), inset \(page.adjustedContentInset); UIKit's \(self.onScreen(native)), inset \(native.adjustedContentInset)"
        }) {
            abs(self.onScreen(page).maxY - self.onScreen(native).maxY) < 0.5
                && abs(page.adjustedContentInset.bottom - native.adjustedContentInset.bottom) < 0.5
        }
        XCTAssertTrue(route.edgeScroll === list)
        // The window changes size with the app still in the foreground, as an iPhone Duo unfolds after a folded
        // launch: the list follows to the new foot with UIKit's inset, with no trip to the background to refresh it
        let window = try XCTUnwrap(self.window), resized = CGRect(origin: .zero, size: CGSize(width: window.bounds.width, height: window.bounds.height - 120))
        let footBefore = onScreen(native).maxY
        window.frame = resized
        oracle.frame = resized
        waitFor("after the resize the list reaches the new foot with UIKit's inset", facts: {
            "list \(self.onScreen(page)), inset \(page.adjustedContentInset); UIKit's \(self.onScreen(native)), inset \(native.adjustedContentInset)"
        }) {
            oracle.layoutIfNeeded()
            return abs(self.onScreen(native).maxY - (footBefore - 120)) < 0.5
                && abs(self.onScreen(page).maxY - self.onScreen(native).maxY) < 0.5
                && abs(page.adjustedContentInset.bottom - native.adjustedContentInset.bottom) < 0.5
                && route.edgeScroll === list
        }
    }

    func testCoverRouteScrollInsetsMatchUIKitOnEachEdge() throws {
        guard let scene = hostedWindowScene() else { throw missing("scene", "the tab and navigation bars need a scene") }
        try checkScrollEdges(in: scene, cover: true)
    }

    private func checkScrollEdges(in scene: UIWindowScene, cover: Bool) throws {
        let (session, _) = try fixture(scene, cover ? "COVER_SCROLL_EDGES" : "ROUTE_SCROLL_EDGES")
        let navigation = session.presenter.navigationHost
        waitFor("the tab's large-title route appears") {
            navigation.tabController?.selectedViewController != nil && self.node(session, "end") != nil && !navigation.inTransition
        }
        let tabs = try XCTUnwrap(navigation.tabController)
        let nav = try XCTUnwrap(tabs.selectedViewController as? UINavigationController)
        let bottomTabs = RouteGeometry.shownBar(tabs.tabBar).map {
            $0.bounds.width > $0.bounds.height && onScreen($0).midY > onScreen(tabs.view).midY
        } ?? false
        session.presenter.press(try XCTUnwrap(node(session, "open")).id)
        waitFor("UIKit takes the pushed route") { nav.viewControllers.count == 2 }
        let route = try XCTUnwrap(nav.topViewController as? RouteController)
        XCTAssertNotNil(route.edgeScroll, "UIKit takes the pushed route's bottom scroller")
        waitFor("the orientation route is on screen") {
            nav.viewControllers.count == 2 && !navigation.inTransition && self.node(session, "open") == nil
        }
        let window = try XCTUnwrap(window)
        let top = try XCTUnwrap(node(session, "window-top")).bounds.height
        let bottom = try XCTUnwrap(node(session, "window-bottom")).bounds.height
        XCTAssertEqual(top, cover ? window.safeAreaInsets.top : 0, accuracy: 0.5)
        XCTAssertEqual(bottom, cover ? window.safeAreaInsets.bottom : 0, accuracy: 0.5)
        let list = try XCTUnwrap(node(session, "list"))
        let page = try XCTUnwrap(list.scrollView)
        XCTAssertTrue(page.isDescendant(of: route.view))
        if !bottomTabs {
            // Tabs at the top or in a sidebar leave the route the screen's foot, under either fit
            waitFor("the list runs under the home indicator, inset by it", facts: {
                "frame \(self.onScreen(page)), inset \(page.adjustedContentInset), window \(window.safeAreaInsets)"
            }) {
                route.edgeScroll === list && abs(self.onScreen(page).maxY - self.onScreen(tabs.view).maxY) < 0.5
                    && abs(page.adjustedContentInset.bottom - window.safeAreaInsets.bottom) < 0.5
            }
            return
        }
        let last = try XCTUnwrap(node(session, "end"))

        let reference = UIViewController()
        let opaque = UINavigationBarAppearance()
        opaque.configureWithOpaqueBackground()
        reference.navigationItem.standardAppearance = opaque
        reference.navigationItem.scrollEdgeAppearance = opaque
        reference.title = "Window"
        reference.navigationItem.largeTitleDisplayMode = .always
        let native = UIScrollView()
        reference.view.addSubview(native)
        native.frame = reference.view.bounds
        native.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        native.contentSize = CGSize(width: 100, height: 2500)
        reference.setContentScrollView(native, for: .all)
        let stack = UINavigationController(rootViewController: UIViewController())
        stack.pushViewController(reference, animated: false)
        stack.navigationBar.prefersLargeTitles = true
        let container = UITabBarController()
        container.viewControllers = [stack]
        let oracle = UIWindow(windowScene: scene)
        oracle.rootViewController = container
        oracle.isHidden = false
        defer { oracle.isHidden = true }
        waitFor("both controllers have their bars") {
            oracle.layoutIfNeeded()
            return native.safeAreaInsets.bottom > oracle.safeAreaInsets.bottom && route.view.safeAreaInsets.bottom > oracle.safeAreaInsets.bottom
        }

        func press(_ id: String) throws {
            session.presenter.press(try XCTUnwrap(node(session, id)).id)
            session.presenter.settlePump()
        }
        for transparent in [false, true] {
            if transparent {
                try press("bar")
                let appearance = UINavigationBarAppearance()
                appearance.configureWithTransparentBackground()
                reference.navigationItem.scrollEdgeAppearance = appearance
            }
            // Toggling one edge at a time also exercises changes on the same mounted scroller.
            for (topOff, bottomOff, toggle) in [(true, false, ""), (true, true, "bottom"), (false, true, "top"), (false, false, "bottom")] {
                if !toggle.isEmpty { try press(toggle) }
                let underTop = topOff || transparent
                let label = "top off \(topOff), bottom off \(bottomOff), transparent \(transparent)"
                // UIKit selects automatic adjustment for the whole scroll view. With an explicit
                // top, give it the bottom band by hand. With an automatic top, cancel an opted-out bottom.
                native.contentInsetAdjustmentBehavior = underTop ? .never : .automatic
                native.contentInset.bottom = underTop ? (bottomOff ? 0 : native.safeAreaInsets.bottom) : (bottomOff ? -native.safeAreaInsets.bottom : 0)
                native.setContentOffset(CGPoint(x: 0, y: -native.adjustedContentInset.top), animated: false)
                page.setContentOffset(CGPoint(x: 0, y: -page.adjustedContentInset.top), animated: false)
                waitForMainQueue()
                oracle.layoutIfNeeded()
                session.presenter.settlePump()
                waitFor("the scroller reaches the same bottom edge as UIKit: \(label)", facts: {
                    "adjusted \(page.adjustedContentInset), content \(page.contentInset), frame \(self.onScreen(page)), route \(route.node.contentBox()), clearance \(String(describing: navigation.clearance[ObjectIdentifier(page)]))"
                }) {
                    abs(page.adjustedContentInset.bottom - native.adjustedContentInset.bottom) < 0.5 && abs(self.onScreen(page).maxY - self.onScreen(native).maxY) < 0.5
                }
                XCTAssertEqual(RouteGeometry.clearanceOff(route.node), (topOff ? 1 : 0) + (bottomOff ? 4 : 0), label)
                XCTAssertEqual(RouteGeometry.transparentBar(route.node), transparent, label)
                XCTAssertEqual(onScreen(page).minY, onScreen(native).minY, accuracy: 0.5, label)
                XCTAssertEqual(page.adjustedContentInset.top, native.adjustedContentInset.top, accuracy: 0.5, label)
                XCTAssertEqual(session.presenter.scrollOwnedChanges(list), [], "Exact's own insets are not a hatch write: \(label)")
                XCTAssertTrue(route.contentScrollView(for: .top) === page, label)
                if !bottomOff { XCTAssertTrue(route.contentScrollView(for: .bottom) === page, label) }

                waitFor("the last row rests at the end: \(label)", facts: { "\(self.onScreen(last)), offset \(page.contentOffset), size \(page.contentSize)" }) {
                    session.presenter.settlePump()
                    for scroller in [native, page] {
                        let end = scroller.contentSize.height + scroller.adjustedContentInset.bottom - scroller.bounds.height
                        if abs(scroller.contentOffset.y - end) > 0.5 {
                            scroller.setContentOffset(CGPoint(x: 0, y: end), animated: false)
                            return false
                        }
                    }
                    return abs(self.onScreen(last).maxY - (self.onScreen(native).maxY - native.adjustedContentInset.bottom)) < 1
                }
                if !bottomOff { XCTAssertLessThanOrEqual(onScreen(last).maxY, onScreen(tabs.tabBar).minY + 1, label) }
                XCTAssertEqual(nav.navigationBar.bounds.height, stack.navigationBar.bounds.height, accuracy: 0.5, "the scrolled title: \(label)")
                for scroller in [native, page] {
                    scroller.setContentOffset(CGPoint(x: 0, y: -scroller.adjustedContentInset.top), animated: false)
                }
                waitForMainQueue()
            }
            if #available(iOS 18.0, *), cover {
                try press("tabs")
                waitFor("with the tab bar hidden the list runs under the home indicator, inset by it", facts: {
                    "frame \(self.onScreen(page)), inset \(page.adjustedContentInset), edge \(String(describing: route.edgeScroll))"
                }) {
                    route.edgeScroll === list && abs(self.onScreen(page).maxY - self.onScreen(tabs.view).maxY) < 0.5
                        && abs(page.adjustedContentInset.bottom - window.safeAreaInsets.bottom) < 0.5
                }
                try press("tabs")
                waitFor("the scroller takes the tab bar again") {
                    route.edgeScroll === list && abs(page.adjustedContentInset.bottom - native.adjustedContentInset.bottom) < 0.5
                }
            }
            if !transparent { try press("top") }
        }

        // A shorter list can end exactly one tab bar above the foot, even after the
        // route has given up its padding. It takes only the safe area it overlaps.
        try press("height")
        waitFor("the height-limited list ends above the tab bar with no bottom inset", facts: {
            "frame \(self.onScreen(page)), bar \(self.onScreen(tabs.tabBar)), adjusted \(page.adjustedContentInset), safe \(page.safeAreaInsets)"
        }) {
            abs(self.onScreen(page).maxY - self.onScreen(tabs.tabBar).minY) < 0.5 && abs(page.adjustedContentInset.bottom) < 0.5
        }
        try press("height")
        waitFor("the list regains its bottom inset when it grows under the bar") {
            abs(self.onScreen(page).maxY - self.onScreen(native).maxY) < 0.5 && abs(page.adjustedContentInset.bottom - native.adjustedContentInset.bottom) < 0.5
        }
    }

    func testAScrollerBelowAControlBarGetsNoInsetForIt() throws {
        guard let scene = UIApplication.shared.connectedScenes.first as? UIWindowScene else {
            throw missing("scene", "the bars and the safe area are a scene window's")
        }
        let (session, _) = try fixture(scene, "BAR_CONTROL")
        until("the route shows its scroller under its bar") {
            guard let route = self.node(session, "route"), let c = session.presenter.navigationHost.controllers[route.id] else { return false }
            return c.viewIfLoaded?.window != nil && c.view.safeAreaInsets.top > 0 && self.node(session, "first") != nil
        }
        let route = try XCTUnwrap(node(session, "route"))
        let c = try XCTUnwrap(session.presenter.navigationHost.controllers[route.id])
        let control = try XCTUnwrap(node(session, "control")), list = try XCTUnwrap(node(session, "list"))
        let sv = try XCTUnwrap(list.scrollView)
        // The route clears the bar, the control sits under it and the scroller under the control
        XCTAssertEqual(onScreen(control).minY, c.view.safeAreaInsets.top, accuracy: 0.5, "the control under the bar")
        XCTAssertEqual(onScreen(list).minY, onScreen(control).maxY, accuracy: 0.5, "the scroller under the control")
        until("the first row starts at the scroller's top") {
            guard let first = self.node(session, "first") else { return false }
            return abs(self.onScreen(first).minY - self.onScreen(list).minY) < 0.5
        }
        XCTAssertEqual(sv.adjustedContentInset.top, 0, accuracy: 0.5, "no inset: the scroller overlaps no safe area")
    }

    /// A departing route holds its side cover, local insets, and content width steady through
    /// an interactive pop (and resamples cleanly if the pop cancels), even as UIKit translates
    /// its view past a side safe-area strip such as an open iPhone Duo's vertical bar (#1942).
    func testADepartingRouteKeepsItsWidthAndLocalInsetsDuringAnInteractivePop() throws {
        guard let scene = UIApplication.shared.connectedScenes.first as? UIWindowScene else {
            throw missing("scene", "interactive pop transitions need a window scene")
        }
        let (session, _) = try fixture(scene)
        let navigation = session.presenter.navigationHost
        until("the root route shows") { self.node(session, "more") != nil && self.node(session, "route") != nil }
        let rootNode = try XCTUnwrap(node(session, "route"))
        let rootController = try XCTUnwrap(navigation.controllers[rootNode.id])
        let rootRestingRight = rootController.surface.safeAreaInsets.right
        let rootRestingCover = navigation.covers[rootNode.id]
        let host = try XCTUnwrap(window?.rootViewController)
        _ = Agent(session: session).tap(["id": Int(try XCTUnwrap(node(session, "more")).id)])
        var nav: UINavigationController?
        var pushSnapshots: [String] = []
        until("the pushed detail route settles") {
            guard let detail = self.node(session, "detail"),
                  let c = navigation.controllers[detail.id],
                  let stack = c.navigationController else { return false }
            nav = stack
            if stack.transitionCoordinator != nil {
                pushSnapshots.append("rootSafeR=\(rootController.surface.safeAreaInsets.right) rootCover=\(String(describing: navigation.covers[rootNode.id])) detailSafeR=\(c.surface.safeAreaInsets.right) detailCover=\(String(describing: navigation.covers[detail.id]))")
            }
            return stack.viewControllers.count == 2 && stack.transitionCoordinator == nil
                && navigation.localInsets.wanted[detail.id] != nil
        }
        let detail = try XCTUnwrap(node(session, "detail"))
        let fill = try XCTUnwrap(node(session, "detail-fill"))
        let rightProbe = try XCTUnwrap(node(session, "detail-right-probe"))
        let c = try XCTUnwrap(navigation.controllers[detail.id])
        let stack = try XCTUnwrap(nav)
        // On a screen without a native vertical bar (non-Duo), simulate an 84 pt trailing strip.
        // On an open iPhone Duo, UIKit's own vertical bar already supplies the trailing inset.
        let simulatedStrip = c.surface.safeAreaInsets.right < 1
        if simulatedStrip {
            host.additionalSafeAreaInsets.right = 84
            window?.layoutIfNeeded()
            until("simulated trailing strip reaches detail") {
                c.surface.safeAreaInsets.right > 1 && (navigation.localInsets.wanted[detail.id]?[1] ?? 0) > 1
            }
        }
        let expectedRight = c.surface.safeAreaInsets.right
        XCTAssertGreaterThan(expectedRight, 1, "detail has a trailing side strip")
        until("detail content and right-inset probe settle") {
            abs(rightProbe.bounds.width - expectedRight) < 0.5
                && abs(fill.bounds.width - (c.surface.bounds.width - expectedRight - c.surface.safeAreaInsets.left)) < 1
        }
        let restingWidth = fill.bounds.width
        let restingCover = navigation.covers[detail.id]
        let restingLocal = try XCTUnwrap(navigation.localInsets.wanted[detail.id])

        // Drive an interactive pop that slides `detail` rightward past the trailing strip
        // (matching UIKit's interactive pop animation on an open iPhone Duo).
        final class SlidePop: NSObject, UINavigationControllerDelegate, UIViewControllerAnimatedTransitioning {
            let interaction = UIPercentDrivenInteractiveTransition()
            func navigationController(_ nav: UINavigationController, animationControllerFor op: UINavigationController.Operation,
                                      from: UIViewController, to: UIViewController) -> UIViewControllerAnimatedTransitioning? {
                op == .pop ? self : nil
            }
            func navigationController(_ nav: UINavigationController, interactionControllerFor animator: UIViewControllerAnimatedTransitioning) -> UIViewControllerInteractiveTransitioning? {
                interaction
            }
            func transitionDuration(using context: UIViewControllerContextTransitioning?) -> TimeInterval { 0.35 }
            func animateTransition(using context: UIViewControllerContextTransitioning) {
                guard let from = context.viewController(forKey: .from), let to = context.viewController(forKey: .to) else {
                    return context.completeTransition(false)
                }
                let container = context.containerView
                to.view.frame = context.finalFrame(for: to)
                container.insertSubview(to.view, belowSubview: from.view)
                let start = context.initialFrame(for: from)
                UIView.animate(withDuration: transitionDuration(using: context), animations: {
                    from.view.frame = start.offsetBy(dx: container.bounds.width, dy: 0)
                }, completion: { _ in
                    let finished = !context.transitionWasCancelled
                    if !finished { from.view.frame = start }
                    context.completeTransition(finished)
                })
            }
        }

        let pop = SlidePop()
        navigation.setAppDelegate(stack, pop)
        defer { navigation.setAppDelegate(stack, nil) }
        stack.popViewController(animated: true)
        until("interactive pop starts") { stack.transitionCoordinator != nil }
        pop.interaction.update(0.4)
        // Let UIKit lay out the translated view, fire viewSafeAreaInsetsDidChange(), and flush layout.
        for _ in 0..<15 {
            window?.layoutIfNeeded()
            navigation.reportCovers()
            session.presenter.settlePump()
            RunLoop.main.run(mode: .default, before: Date().addingTimeInterval(0.02))
        }
        // Mid-swipe: `detail` must keep its resting cover, local insets, and content width
        // even though translating `from.view` rightward dropped its live safeAreaInsets.right to 0.
        XCTAssertEqual(navigation.covers[detail.id], restingCover, "departing route keeps its settled cover mid-swipe")
        XCTAssertEqual(navigation.localInsets.wanted[detail.id], restingLocal, "departing route keeps its local insets mid-swipe")
        XCTAssertEqual(fill.bounds.width, restingWidth, accuracy: 0.5, "departing route does not widen mid-swipe")
        XCTAssertEqual(rightProbe.bounds.width, expectedRight, accuracy: 0.5, "var(--exact-safe-area-inset-right) does not drop to 0 mid-swipe")
        // Also verify returning root does not transiently adopt detail's trailing strip on real Duo.
        if !simulatedStrip {
            XCTAssertEqual(
                navigation.covers[rootNode.id],
                rootRestingCover,
                "returning root keeps its resting cover mid-swipe (rootSafeR=\(rootController.surface.safeAreaInsets.right) restR=\(rootRestingRight) detailR=\(expectedRight) push=\(pushSnapshots.prefix(3)))"
            )
        }

        // Cancel the swipe and verify covers resample cleanly once settled.
        pop.interaction.cancel()
        until("cancelled pop settles back on detail") { stack.transitionCoordinator == nil && stack.viewControllers.count == 2 }
        XCTAssertEqual(fill.bounds.width, restingWidth, accuracy: 0.5, "cancelled pop preserves resting width")
        XCTAssertEqual(rightProbe.bounds.width, expectedRight, accuracy: 0.5, "cancelled pop preserves resting right inset")

        // Complete a second swipe and verify the stack settles cleanly on root.
        stack.popViewController(animated: true)
        until("second interactive pop starts") { stack.transitionCoordinator != nil }
        pop.interaction.update(0.4)
        window?.layoutIfNeeded()
        navigation.reportCovers()
        session.presenter.settlePump()
        XCTAssertEqual(fill.bounds.width, restingWidth, accuracy: 0.5, "second swipe keeps width")
        pop.interaction.finish()
        until("finished pop lands on root") { stack.transitionCoordinator == nil && stack.viewControllers.count == 1 }
    }
}
#endif
