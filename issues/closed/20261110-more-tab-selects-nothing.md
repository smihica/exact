# iOS: in a compact width a root with six or more tabs gets UIKit's More item, which Exact's tab delegate refuses, so the fifth and later tabs cannot be reached

**Status:** Closed
**Resolution:** Fixed by deb892dc: UIKit selects tabs, More is one of Exact's stacks and showing a tab under it presses the authored tab; More's Edit stays, as UIKit offers it (NavigationMoreIOSTests, run on a simulator there).
**Systems:** host/apple/Sources/ExactKit/IOS/NavigationTabsIOS.swift
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

References are at `dd6f5dcd`.

`installTabs` gives Exact's `UITabBarController` one stack for every tabpanel the root's tablist names, with no limit (`NavigationTabsIOS.swift:65-76`; `NavigationTabs.of`, `NavigationTabs.swift:50-64`). The compiler sets none either (`lower-single-tab` asks only for two), and `docs/` names none. Given six or more view controllers, UIKit "displays only the first four items plus the standard More item", whose controller, `moreNavigationController`, is not in `viewControllers` ([UITabBarController](https://developer.apple.com/documentation/uikit/uitabbarcontroller), "The More navigation controller"; [moreNavigationController](https://developer.apple.com/documentation/uikit/uitabbarcontroller/morenavigationcontroller)).

Exact's delegate answers every tap on the bar itself (`NavigationTabsIOS.swift:25-29`):

```swift
func tabBarController(_ tabs: UITabBarController, shouldSelect controller: UIViewController) -> Bool {
    if app?.tabBarController?(tabs, shouldSelect: controller) == false { return false }
    host?.selectTab(controller)
    return false
}
```

`selectTab` presses the authored tab whose stack is `controller` (`NavigationTabsIOS.swift:344-347`). For the More item `controller` is `moreNavigationController`, which is no panel's stack: nothing is pressed, the delegate returns false, and the More list never opens. The authored tablist takes no room while the bar shows (`NavigationCoversIOS.swift:162-164`), so nothing else on screen selects those tabs.

## The failure

`apps/duo-lab/app.contract:399-415` is a root tablist of six tabs: Fold, Panes, Images, Reflow, Combos, Keyboard. On an iPhone in portrait the bar shows the first four and More. Tapping More does nothing, and Combos and Keyboard cannot be reached. Expected: every authored tab can be selected, as on the web host, whose tablist shows all six.

The agent does not see this: under it the authored tablist shows and the bar is hidden (`NavigationTabsIOS.swift:79`).

## Verification

Exact's part is by reading: this machine cannot run the iOS simulator. UIKit's part was run in a Mac Catalyst process with no window (UIKit 27.0): a `UITabBarController` holding six navigation controllers, a delegate that returns false from `shouldSelect`, compact width and the phone idiom set through `traitOverrides`, and the More item sent to the controller's `_tabBarItemClicked:`, since a process with no window takes no tap. It printed:

```
bar items: ["Tab 1", "Tab 2", "Tab 3", "Tab 4", "More"]
refusing delegate, _tabBarItemClicked:(More): asked ["moreNavigationController"] selectedIndex 0 selected is more: false
```

Five controllers showed five items. In a regular width six showed six, and nine showed seven and More.

A simulator run should build Duo Lab for an iPhone, drive it with `--chrome platform` and tap More.

## Direction

Decide what six tabs are in a compact width and build it: four stacks and a More item that lists the remaining authored tabs, each row pressing its tab, or the authored tablist left showing, as under `appearance="none"`, when the tabs do not fit the bar.

## Related

`issues/closed/20261019-ios-tab-bar-covers-route-content.md` and `issues/closed/20261021-select-tab-and-push-in-one-update.md` concern a bar that shows every tab. Nothing in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` covers the More item.
