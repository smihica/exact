// CSS gradients (LLP 1047.001 D10, the roster's `gradients`): parsing,
// resolving and painting a `background-image` or `mask-image` gradient, and
// the pattern layer repeating linear gradients tile, installed into
// `GradientLink` by an app whose composition links them.
import ExactKit

public enum ExactGradients {
    /// Set `GradientLink`'s slots, before any session is made.
    public static func install() {
        GradientLink.make = { Gradient($0, tint: $1) }
        GradientLink.makePattern = { GradientPatternLayer() }
    }
}
