// UI-resolved immutable font/source capture. CoreText layout stays on the worker.
#if os(macOS)
import Foundation
import CoreText
#if canImport(UIKit)
import UIKit
#else
import AppKit
#endif

/// CoreText permits concurrent use of immutable font objects. Layout objects
/// (typesetter/line/run) stay entirely inside RegionTextLayout on its worker.
/// https://developer.apple.com/documentation/coretext
package final class RegionFontOwner: @unchecked Sendable {
    // The SDK omits Sendable on CTFont. This exception covers only the retained
    // immutable font object Apple explicitly permits concurrent use of, never a
    // layout object, attributed string, engine, cache, or resolver closure.
    package let value: CTFont
    package init(_ value: CTFont) { self.value = value }
}

package struct RegionFont: Sendable {
    package let owner: RegionFontOwner
    package var value: CTFont { owner.value }
    package let box: TextLineBox
}

package struct RegionTextRun: Sendable {
    package let text: String
    package let font: RegionFont
    package let lineHeight: CGFloat?
    package let letterSpacing: CGFloat
    /// Its inline boxes' room beside its text (`Run.inlineStart`, `inlineEnd`).
    package let inlineStart: CGFloat
    package let inlineEnd: CGFloat
    package let color: CGColor
    /// `Run.fixedInk`: its colour is one its author fixed.
    package let fixedInk: Bool
    package let underline: Bool
    package let strike: Bool
    /// A decorating box's colour for a line its own colour does not draw (`Run.propagate`).
    package let underlineColor: CGColor?
    package let strikeColor: CGColor?
    package let href: String
    package let range: NSRange
    package let box: InlineBox?
    let icon: (any TextAtomIconPainting)?
    let background: InlineBackground?
}

package final class RegionTextSource: Sendable {
    package let runs: [RegionTextRun]
    package let rasterFormat: (space: CGColorSpace, headroom: Float, deep: Bool)
    package let strut: RegionFont?
    package let strutExplicit: Bool
    package let flowFontSize: CGFloat
    package let align: Int
    package let direction: Int
    /// Natural text's paragraph bases (LLP 1060.000 D3); empty, `direction`.
    package let bases: [(start: Int, rtl: Bool)]
    package let lineClamp: Int
    package let overflowWrap: Int
    package let textIndent: CGFloat
    package let hyphens: Int
    package let language: String
    package let hyphenated: Bool
    package let emergencyBreaks: [Int]
    package let emergencyHyphens: Bool
    /// CSS `text-wrap-style: balance` (`Spec.balance`).
    package let balance: Bool
    /// CSS `white-space` (`Spec.whiteSpace`): what of a line's trailing spaces
    /// its width counts (`TextEngine.uncountedTrailingWidth`).
    package let whiteSpace: Int
    /// CSS `text-box`: `Spec`'s trim, over and under edges.
    package let textBox: (trim: Int, over: Int, under: Int)
    /// A preserved tab's stop interval (CSS `tab-size: 8`), as the engine's
    /// paragraphs have it; nil when no tab is preserved.
    package let tabInterval: CGFloat?
    /// The device pixels a point line boxes round to (`TextEngine.lineScale`).
    package let lineScale: CGFloat
    package let text: String
    package let utf16Count: Int
    package let sourceUTF8Bytes: Int
    /// Logical joined UTF8 length, not distinct allocated storage: joining one
    /// run may share its String. CF attributed storage is worker-local extra.
    package let joinedSourceUTF8Bytes: Int
    package let captureSeconds: Double
    package let capturedOnMainThread: Bool

    private init(_ spec: Spec, engine: TextEngine) {
        precondition(Thread.isMainThread, "font and appearance resolution belongs to UI")
        let start = ProcessInfo.processInfo.systemUptime
        func font(_ run: Run) -> RegionFont {
            let f = engine.font(run)
            return RegionFont(owner: RegionFontOwner(f as CTFont), box: TextLineBox(f as CTFont, height: run.lineHeight, scale: engine.lineScale))
        }
        var offset = 0
        var paints: [NSDictionary] = []
        runs = spec.runs.map { run in
            let count = run.text.utf16.count
            defer { offset += count }
            var paint: [NSAttributedString.Key: Any] = [.font: engine.font(run)]
            engine.presentation?.attributes(run, color: spec.color, scale: engine.lineScale, attributes: &paint)
            paints.append(paint as NSDictionary)
            return RegionTextRun(text: run.text, font: font(run), lineHeight: run.lineHeight,
                                 letterSpacing: run.letterSpacing, inlineStart: run.inlineStart, inlineEnd: run.inlineEnd,
                                 color: TextEngine.color(run.color ?? spec.color).cgColor,
                                 fixedInk: run.fixedInk && run.color != nil,
                                 underline: run.underlined || run.underlineColor != nil,
                                 strike: run.decoration.contains("line-through") || run.strikeColor != nil,
                                 underlineColor: run.underlined ? nil : run.underlineColor.map { TextEngine.color($0).cgColor },
                                 strikeColor: run.decoration.contains("line-through") ? nil : run.strikeColor.map { TextEngine.color($0).cgColor },
                                 href: run.href,
                                 range: NSRange(location: offset, length: count), box: run.box,
                                 icon: paint[.exactAtomIcon] as? any TextAtomIconPainting, background: paint[.exactBackground] as? InlineBackground)
        }
        rasterFormat = TextRasterJob.format(paints)
        let base = spec.strut ?? spec.runs.first
        strut = base.map(font); strutExplicit = base?.lineHeight != nil
        flowFontSize = base?.size ?? 0
        align = spec.align; direction = spec.direction; lineClamp = spec.lineClamp; overflowWrap = spec.overflowWrap
        textIndent = spec.firstLineIndent; hyphens = spec.hyphens; language = spec.language
        hyphenated = !spec.source.inserted.isEmpty; emergencyBreaks = spec.emergencyBreaks
        emergencyHyphens = spec.emergencyHyphens
        textBox = (spec.textBoxTrim, spec.textBoxOver, spec.textBoxUnder)
        balance = spec.balance
        whiteSpace = spec.whiteSpace
        lineScale = engine.lineScale
        text = spec.runs.map(\.text).joined()
        bases = spec.natural ? TextEngine.naturalBases(text, fallback: spec.direction) : []
        tabInterval = spec.preserves && text.contains("\t") ? base.map { TextEngine.tabInterval(font($0).value, letterSpacing: $0.letterSpacing) } : nil
        utf16Count = offset
        sourceUTF8Bytes = spec.runs.reduce(0) { $0 + $1.text.utf8.count }
        joinedSourceUTF8Bytes = text.utf8.count
        capturedOnMainThread = Thread.isMainThread
        captureSeconds = ProcessInfo.processInfo.systemUptime - start
    }

    /// A baseline on the device pixels its line box was rounded to.
    package func painted(_ baseline: CGFloat) -> CGFloat { (baseline * lineScale).rounded() / lineScale }

    /// Whether the line that starts at UTF-16 `location` has a right-to-left base.
    package func base(at location: Int) -> Bool {
        var paragraphs = LineInsets()
        paragraphs.rtl = direction == 1; paragraphs.bases = bases
        return paragraphs.rtl(at: location)
    }

    package var insets: LineInsets {
        var value = LineInsets()
        value.textIndent = textIndent; value.rtl = direction == 1; value.bases = bases
        value.edgeStarts = LineInsets.edgeStarts(runs.map { (text: $0.text, start: $0.inlineStart) })
        return value
    }

    package static func capture(_ spec: Spec, engine: TextEngine) -> RegionTextSource {
        RegionTextSource(spec, engine: engine)
    }

    /// Authored ranges are ordered and disjoint, including empty runs. CoreText
    /// may coalesce several of them into one glyph run; visit only its overlaps.
    package func runs(overlapping range: NSRange) -> ArraySlice<RegionTextRun> {
        var lo = 0, hi = runs.count
        while lo < hi {
            let mid = lo + (hi - lo) / 2
            if NSMaxRange(runs[mid].range) <= range.location { lo = mid + 1 }
            else { hi = mid }
        }
        let start = lo
        while lo < runs.count && runs[lo].range.location < NSMaxRange(range) { lo += 1 }
        return runs[start..<lo]
    }

    package func link(at index: Int) -> String? {
        guard let run = runs(overlapping: NSRange(location: index, length: 1)).first,
              !run.href.isEmpty else { return nil }
        return run.href
    }

    /// Locally mutable Foundation builder; only immutable font/color/scalars
    /// from capture are used. No AppKit font manager, resolver or TextEngine.
    package func attributed() -> NSAttributedString {
        let value = NSMutableAttributedString()
        for (index, run) in runs.enumerated() {
            var attributes: [NSAttributedString.Key: Any] = [
                NSAttributedString.Key(kCTFontAttributeName as String): run.font.value,
                NSAttributedString.Key(kCTForegroundColorAttributeName as String): run.color,
            ]
            if let box = run.box { TextEngine.boxAttributes(box, font: run.font.value, index: index, scale: lineScale, into: &attributes) }
            if let icon = run.icon { attributes[.exactAtomIcon] = icon }
            if let background = run.background { attributes[.exactBackground] = background }
            if run.letterSpacing != 0 { attributes[.kern] = run.letterSpacing }
            if run.fixedInk { attributes[.exactFixedInk] = true }
            if run.icon == nil, run.underline { attributes[.underlineStyle] = NSUnderlineStyle.single.rawValue }
            if run.icon == nil, run.strike { attributes[.strikethroughStyle] = NSUnderlineStyle.single.rawValue }
            if let c = run.underlineColor { attributes[NSAttributedString.Key(kCTUnderlineColorAttributeName as String)] = c }
            if let c = run.strikeColor { attributes[.strikethroughColor] = c }
            value.append(NSAttributedString(string: run.text, attributes: attributes))
        }
        let rooms = runs.map { (text: $0.text, start: $0.inlineStart, end: $0.inlineEnd) }
        TextEngine.kernInlineEdges(value, runs: rooms, rtl: direction == 1)
        TextEngine.setBaseDirection(value, bases: bases.isEmpty ? [(start: 0, rtl: direction == 1)] : bases)
        let empties = runs.indices.compactMap { i -> EmptyRun? in
            guard runs[i].text.isEmpty, let background = runs[i].background else { return nil }
            return EmptyRun(offset: runs[i].range.location, attributes: [.exactBackground: background], past: Spec.roomPast(rooms, i))
        }
        PresentedText().finish(value, empties: empties)
        if let tabInterval { TextEngine.setTabStops(value, interval: tabInterval) }
        return value
    }
}
#endif
