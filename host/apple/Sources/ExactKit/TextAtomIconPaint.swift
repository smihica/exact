// An inline atom's icon in a line of text (LLP 1045.000 D4). Only Markdown
// atoms carry one, so the Markdown family makes the icon's paint record
// (`MarkdownCapability.atomIcon`); the core draws whatever record a run's
// `.exactAtomIcon` attribute holds.
import CExact
import CoreText
#if os(macOS)
import AppKit
#else
import UIKit
#endif

/// The pixels of a run's atom icon, captured before the line reaches any
/// background painter.
package protocol TextAtomIconPainting: AnyObject, Sendable {
    /// Nil while its picture loads.
    var image: CGImage? { get }
    /// A symbol, a mask filled with the run's ink.
    var tinted: Bool { get }
    /// Draw it over `run`'s placeholder, a tinted one in `ink` or the run's colour.
    func draw(_ run: CTRun, in context: CGContext, ink: CGColor?)
}

package enum TextAtomIcons {
    /// The atom icon `run` shows, unless it is hidden.
    package static func icon(_ run: CTRun) -> (any TextAtomIconPainting)? {
        let attrs = CTRunGetAttributes(run) as NSDictionary
        guard attrs[NSAttributedString.Key.exactHidden] as? Bool != true, CTRunGetGlyphCount(run) > 0 else { return nil }
        return attrs[NSAttributedString.Key.exactAtomIcon] as? any TextAtomIconPainting
    }
    static func draw(_ line: CTLine, in context: CGContext, ink: CGColor? = nil) {
        for run in CTLineGetGlyphRuns(line) as! [CTRun] { draw(run, in: context, ink: ink) }
    }
    static func draw(_ run: CTRun, in context: CGContext, ink: CGColor? = nil) {
        icon(run)?.draw(run, in: context, ink: ink)
    }
}

extension NodeView {
    /// An atom icon's picture arrived: a success changes paint; a failure
    /// also removes the reserved advance.
    package func atomIconReady(geometryChanged: Bool) {
        invalidateText()
        #if os(macOS)
        needsDisplay = true
        #else
        setNeedsDisplay()
        #endif
        guard geometryChanged, let session = presenter?.session, session.booted,
              session.state != .destroyed, !session.atomIconRemeasurePending else { return }
        session.atomIconRemeasurePending = true
        // All completions already queued on main share one geometry update.
        DispatchQueue.main.async { [weak session] in
            guard let session else { return }
            session.atomIconRemeasurePending = false
            guard session.booted, session.state != .destroyed else { return }
            session.apply(session.runtime.on { session.runtime.read(exact_text_scale_changed(session.runtime.rt)) })
        }
    }
}

extension NSAttributedString.Key {
    static let exactAtomIcon = NSAttributedString.Key("ExactAtomIcon")
}
