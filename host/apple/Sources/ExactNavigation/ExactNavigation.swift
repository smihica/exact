// The navigation family (LLP 1047.001 D10, the roster's `Navigation`):
// route stacks, tabs and their bars as UIKit's navigation and tab bar
// controllers with route modals (iOS and tvOS), and on the Mac the routes
// shown and hidden by the web's rule, route sheets, the list beside its
// detail, Back and Forward, the root tabs as the window's sidebar and the
// back swipe. An app's composition links this module when its plan has a
// navigation root, and installs it before any session is made; the core
// reaches it only through `NavigationLink`.
import ExactKit

public enum ExactNavigation {
    public static func install() {
        #if os(iOS) || os(tvOS)
        NavigationLink.make = { NavigationHost(presenter: $0) }
        NavigationLink.makeModals = { ModalHost(presenter: $0) }
        #elseif os(macOS)
        NavigationLink.make = { NavigationHost(presenter: $0) }
        NavigationLink.makeSidebar = { SidebarHost($0) }
        NavigationLink.makeBackSwipe = { BackSwipe($0) }
        NavigationLink.makeWindowSplit = { view, options in
            ExactSidebarController(view: view, autosaveName: options.autosaveName, minimumPageWidth: options.minimumPageWidth,
                                   sidebar: options.sidebar, minimumDetailWidth: options.minimumDetailWidth)
        }
        #endif
    }
}
