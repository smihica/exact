// Queue-neutral owned result. No CTLine/CTRun/CTTypesetter or TextEngine alias.
#if os(macOS)
import Foundation
import CoreText
import CExact
#if canImport(UIKit)
import UIKit
#else
import AppKit
#endif

package struct RegionCaret: Sendable {
    package let primary: CGFloat
    package let secondary: CGFloat
}

package struct RegionLine: Sendable {
    package let range: NSRange
    package let carets: [RegionCaret]
    private let selectionRuns: [TextSelectionRun]
    package let ink: CGRect
    package let ascent: CGFloat
    package let descent: CGFloat
    package let leading: CGFloat
    package let typographicWidth: CGFloat
    package let flushOffset: CGFloat
    /// Point answers and open-interval answers are kept separately. The worker
    /// asks CoreText at caret boundaries; the UI never invokes CTLine APIs.
    private let hitEdges: [CGFloat]
    private let edgeIndices: [CFIndex]
    private let intervalIndices: [CFIndex]

    /// `whiteSpace` and `text` (the line's source) place the trailing spaces its width counts (`TextEngine.penOffset`).
    package init(_ line: CTLine, flush: CGFloat, width: CGFloat, rtl: Bool = false, whiteSpace: Int = 0, text: NSAttributedString? = nil,
         inset: (left: CGFloat, width: CGFloat) = (0, 0), captureHits: Bool = true, conservativeInk: CGRect? = nil,
         origin: CGFloat? = nil, emptyIndex: Int? = nil) {
        let r = CTLineGetStringRange(line)
        range = NSRange(location: emptyIndex ?? r.location, length: r.length)
        var above: CGFloat = 0, below: CGFloat = 0, extra: CGFloat = 0
        typographicWidth = CGFloat(CTLineGetTypographicBounds(line, &above, &below, &extra))
        ascent = above; descent = below; leading = extra
        if let conservativeInk {
            ink = CGRect(x: conservativeInk.minX, y: conservativeInk.minY,
                         width: typographicWidth + conservativeInk.width, height: conservativeInk.height)
        } else { ink = CTLineGetBoundsWithOptions(line, .useGlyphPathBounds) }
        flushOffset = origin ?? (inset.left + TextEngine.penOffset(line, flush: flush, room: Double(width - inset.width), rtl: rtl, whiteSpace: whiteSpace, source: text))
        if !captureHits {
            self.carets = []; selectionRuns = []; hitEdges = []; edgeIndices = []; intervalIndices = []
            return
        }
        selectionRuns = TextSelectionRun.capture(line)
        var carets: [RegionCaret] = []
        var offsets: [CGFloat] = []
        for index in r.location...(r.location + r.length) {
            var secondary: CGFloat = 0
            let primary = CTLineGetOffsetForStringIndex(line, index, &secondary)
            carets.append(RegionCaret(primary: primary, secondary: secondary))
            offsets += [primary, secondary]
        }
        self.carets = carets
        CTLineEnumerateCaretOffsets(line) { offset, _, _, _ in offsets.append(offset) }
        offsets += [0, typographicWidth]
        offsets = Array(Set(offsets.filter(\.isFinite))).sorted()
        var edges = offsets
        for i in 1..<offsets.count { edges.append((offsets[i - 1] + offsets[i]) / 2) }
        edges = Array(Set(edges)).sorted()
        func hit(_ x: CGFloat) -> CFIndex {
            CTLineGetStringIndexForPosition(line, CGPoint(x: x, y: 0))
        }
        hitEdges = edges
        edgeIndices = edges.map(hit)
        var intervals = [hit((edges.first ?? 0) - 1)]
        for i in 1..<edges.count { intervals.append(hit((edges[i - 1] + edges[i]) / 2)) }
        intervals.append(hit((edges.last ?? 0) + 1))
        intervalIndices = intervals
    }

    package func offsets(at index: Int) -> RegionCaret {
        let local = index - range.location
        guard carets.indices.contains(local) else { return RegionCaret(primary: 0, secondary: 0) }
        return carets[local]
    }

    package func selectionSpans(_ selection: NSRange) -> [ClosedRange<CGFloat>] {
        TextSelectionRun.spans(selection, runs: selectionRuns) { offsets(at: $0).primary }
    }

    package func index(at x: CGFloat) -> CFIndex {
        if range.length == 0 { return range.location }
        var low = 0, high = hitEdges.count
        while low < high {
            let mid = low + (high - low) / 2
            if hitEdges[mid] < x { low = mid + 1 } else { high = mid }
        }
        if low < hitEdges.count, hitEdges[low] == x { return edgeIndices[low] }
        return intervalIndices[low]
    }

    package var arrayBytes: Int {
        carets.count * MemoryLayout<RegionCaret>.stride
            + selectionRuns.count * MemoryLayout<TextSelectionRun>.stride
            + hitEdges.count * MemoryLayout<CGFloat>.stride
            + (edgeIndices.count + intervalIndices.count) * MemoryLayout<CFIndex>.stride
    }
}

package final class RegionParagraph: Sendable {
    package let source: RegionTextSource
    package let sourceSHA256: String
    package let lines: [RegionLine]
    package let baselines: [CGFloat]
    package let lineBottoms: [CGFloat]
    package let width: CGFloat
    package let height: CGFloat
    package let offeredWidth: CGFloat
    package let exclusions: [TextFlowShape]
    package let fragments: [ExactFlowFragment]
    package let flowLineHeight: CGFloat
    /// Synthetic soft hyphens belong to layout, never copied text or event indices.
    package let inserted: [Int]
    private var sourceMap: SourceMap { SourceMap(inserted: inserted) }
    package let shapedOnMainThread: Bool
    /// The C seam uses -1 when no line supplies a baseline.
    package var firstBaseline: CGFloat { baselines.first ?? -1 }
    /// Logical array payload only; font/platform internals and array slack are
    /// opaque, source storage is reported separately, not a peak RSS guarantee.
    package var arrayBytes: Int {
        inserted.count * MemoryLayout<Int>.stride + fragments.count * MemoryLayout<ExactFlowFragment>.stride
            + exclusions.reduce(0) { $0 + MemoryLayout<TextFlowShape>.stride + $1.pairs.count * MemoryLayout<SIMD2<Float>>.stride }
            + lines.reduce(0) { $0 + $1.arrayBytes } + lines.count * MemoryLayout<RegionLine>.stride
            + (baselines.count + lineBottoms.count) * MemoryLayout<CGFloat>.stride
    }

    package convenience init(source: RegionTextSource, sourceSHA256: String, lines: [CTLine], baselines: [CGFloat], width: CGFloat,
                     height: CGFloat, lineBottoms: [CGFloat], offeredWidth: CGFloat, retainHits: Bool = true, captureHits: Bool = true,
                     attributed: NSAttributedString? = nil, inserted: [Int] = []) {
        self.init(source: source, sourceSHA256: sourceSHA256, lines: lines, baselines: baselines, width: width,
                  height: height, lineBottoms: lineBottoms, offeredWidth: offeredWidth, retainHits: retainHits,
                  captureHits: captureHits, attributed: attributed, inserted: inserted, metadataCheckpoint: {})
    }
    package init(source: RegionTextSource, sourceSHA256: String, lines: [CTLine], baselines: [CGFloat], width: CGFloat,
         height: CGFloat, lineBottoms: [CGFloat], offeredWidth: CGFloat, retainHits: Bool = true, captureHits: Bool = true,
         attributed: NSAttributedString? = nil, inserted: [Int] = [], origins: [CGFloat] = [],
         exclusions: [TextFlowShape] = [], fragments: [ExactFlowFragment] = [], flowLineHeight: CGFloat = 0,
         metadataCheckpoint: () throws -> Void) rethrows {
        precondition(!Thread.isMainThread)
        self.source = source; self.baselines = baselines; self.lineBottoms = lineBottoms
        self.sourceSHA256 = sourceSHA256
        self.width = width; self.height = height; self.offeredWidth = offeredWidth
        self.inserted = inserted
        self.exclusions = exclusions; self.fragments = fragments; self.flowLineHeight = flowLineHeight
        shapedOnMainThread = Thread.isMainThread
        let flush: CGFloat = source.align == 1 ? 0.5 : source.align == 2 ? 1 : 0
        var captured: [RegionLine] = []
        if retainHits {
            captured.reserveCapacity(lines.count)
            for (index, line) in lines.enumerated() {
                if index > 0 && index % 128 == 0 { try metadataCheckpoint() }
                let location = CTLineGetStringRange(line).location
                captured.append(RegionLine(line, flush: flush, width: offeredWidth, rtl: source.base(at: location), whiteSpace: source.whiteSpace,
                                           text: attributed, inset: source.insets.at(location), captureHits: captureHits,
                                           origin: origins.indices.contains(index) ? origins[index] : nil,
                                           emptyIndex: fragments.indices.contains(index) && CTLineGetGlyphCount(line) == 0 ? fragments[index].utf16_start : nil))
            }
            // Include the final short tail. No partially initialized metadata
            // or layout binding escapes if the owning request was superseded.
            if !lines.isEmpty { try metadataCheckpoint() }
        }
        self.lines = captured
    }

    package init(source: RegionTextSource, sourceSHA256: String, lines: [RegionLine], inserted: [Int] = [], baselines: [CGFloat],
         lineBottoms: [CGFloat], width: CGFloat, height: CGFloat, offeredWidth: CGFloat) {
        precondition(!Thread.isMainThread)
        self.source = source; self.sourceSHA256 = sourceSHA256; self.lines = lines
        self.baselines = baselines; self.lineBottoms = lineBottoms
        self.width = width; self.height = height; self.offeredWidth = offeredWidth
        self.inserted = inserted
        exclusions = []; fragments = []; flowLineHeight = 0
        shapedOnMainThread = false
    }

    package func sourceIndex(_ shaped: Int) -> Int {
        shaped == kCFNotFound ? source.utf16Count : min(max(0, sourceMap.source(shaped)), source.utf16Count)
    }
    package func shapedRange(_ range: NSRange) -> NSRange {
        let start = sourceMap.collapsed(range.location)
        return NSRange(location: start, length: sourceMap.collapsed(NSMaxRange(range)) - start)
    }

    package func copy(_ range: NSRange) -> String {
        guard range.location >= 0, range.length >= 0, range.location <= source.utf16Count,
              range.length <= source.utf16Count - range.location else { return "" }
        return (source.text as NSString).substring(with: range)
    }

    package func lineIndex(at y: CGFloat, lastInBand: Bool = false) -> Int? {
        guard !lines.isEmpty else { return nil }
        var lo = 0, hi = lineBottoms.count
        while lo < hi {
            let mid = lo + (hi - lo) / 2
            if lineBottoms[mid] <= y { lo = mid + 1 } else { hi = mid }
        }
        var index = min(lo, lines.count - 1)
        if lastInBand {
            while index + 1 < lines.count && baselines[index + 1] == baselines[index] { index += 1 }
        }
        return index
    }
    package func lineIndex(at point: CGPoint) -> Int? {
        guard let first = lineIndex(at: point.y), !fragments.isEmpty else { return lineIndex(at: point.y) }
        let baseline = baselines[first]
        var start = first
        while start > 0 && baselines[start - 1] == baseline { start -= 1 }
        var best = start, distance = CGFloat.infinity
        for i in start..<lines.count {
            if baselines[i] != baseline { break }
            let x = lines[i].flushOffset
            let d = max(0, max(x - point.x, point.x - x - lines[i].typographicWidth))
            if d < distance { best = i; distance = d }
        }
        return best
    }
    private func line(at point: CGPoint, in bounds: CGRect) -> RegionLine? {
        lineIndex(at: CGPoint(x: point.x - bounds.minX, y: point.y - bounds.minY)).map { lines[$0] }
    }
    package func cachedIndex(at point: CGPoint, in bounds: CGRect, artifact: UInt64, hits: RegionViewportHits) -> Int? {
        guard point.x.isFinite, point.y.isFinite else { return nil }
        if point.y < bounds.minY { return 0 }
        if point.y > bounds.maxY { return source.utf16Count }
        guard let i = lineIndex(at: CGPoint(x: point.x - bounds.minX, y: point.y - bounds.minY)) else { return 0 }
        if lines[i].range.length == 0 { return sourceIndex(lines[i].range.location) }
        guard let index = hits.index(artifact: artifact,line: i,x: point.x - bounds.minX - lines[i].flushOffset) else { return nil }
        return sourceIndex(index)
    }
    package func link(at point: CGPoint, in bounds: CGRect, exactIndex: Int) -> String? {
        guard bounds.contains(point), let line = line(at: point,in: bounds) else { return nil }
        let x = bounds.minX + line.flushOffset
        guard point.x >= x, point.x <= x + line.typographicWidth else { return nil }
        return source.link(at: exactIndex)
    }

    package func index(at point: CGPoint, in bounds: CGRect) -> Int {
        if point.y < bounds.minY { return 0 }
        if point.y > bounds.maxY { return source.utf16Count }
        guard let line = line(at: point, in: bounds) else { return 0 }
        let index = line.index(at: point.x - bounds.minX - line.flushOffset)
        return sourceIndex(index)
    }

    package func link(at point: CGPoint, in bounds: CGRect) -> String? {
        guard bounds.contains(point), let line = line(at: point, in: bounds) else { return nil }
        let x = bounds.minX + line.flushOffset
        guard point.x >= x, point.x <= x + line.typographicWidth else { return nil }
        return source.link(at: index(at: point, in: bounds))
    }

    package func selectionRects(_ selection: NSRange, in bounds: CGRect, dirty: CGRect) -> [CGRect] {
        guard selection.location >= 0, selection.length > 0,
              selection.location <= source.utf16Count,
              selection.length <= source.utf16Count - selection.location else { return [] }
        let selection = shapedRange(selection)
        var result: [CGRect] = []
        for (line, baseline) in zip(lines, baselines) {
            let lo = max(selection.location, line.range.location)
            let hi = min(NSMaxRange(selection), NSMaxRange(line.range))
            guard hi > lo else { continue }
            let y = bounds.minY + source.painted(baseline)
            guard y + line.descent >= dirty.minY, y - line.ascent <= dirty.maxY else { continue }
            for span in line.selectionSpans(NSRange(location: lo, length: hi - lo)) {
                result.append(CGRect(x: bounds.minX + line.flushOffset + span.lowerBound, y: y - line.ascent,
                                     width: max(1, span.upperBound - span.lowerBound), height: line.ascent + line.descent))
            }
        }
        return result
    }

}
#endif
