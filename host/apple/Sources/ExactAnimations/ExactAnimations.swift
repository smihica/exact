// CSS animations (LLP 1047.001 D10, the roster's `animations`): reading a
// spec, lowering its keyframes to Core Animation and playing it on a box,
// installed into `AnimationLink` by an app whose composition links them.
import ExactKit

public enum ExactAnimations {
    /// Set `AnimationLink`'s slots, before any session is made.
    public static func install() {
        AnimationLink.make = { BoxAnimations() }
        AnimationLink.read = { $0.compactMap(CssAnimation.init) }
    }
}
