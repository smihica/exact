// Canvas 2D's requests to the runtime (`exact_canvas.h`), which only this
// product makes: an archive whose plan has no canvas emits none of them
// (LLP 1047.001 §9.21).
#if canImport(UIKit)
import UIKit
#else
import AppKit
#endif
import CExact.Canvas
import ExactCore
import ExactKit

extension Runtime {
    /// The display's scale and memory for Canvas 2D (LLP 1056 D4).
    func canvasDisplay(scale: CGFloat, memory: UInt64) -> Batch { on(layout: true) { read(exact_canvas_display(rt, Double(scale), Double(memory))) } }
    /// A Canvas 2D image handle decoded (its size in pixels) or not (LLP 1056 D9).
    func canvasImage(_ src: String, width: Int, height: Int, ok: Bool) -> Batch {
        return on {
            let n = write(src)
            return read(exact_canvas_image(rt, n, UInt32(max(0, width)), UInt32(max(0, height)), ok ? 1 : 0))
        }
    }
    /// The presenter let a decoded Canvas 2D image handle go (LLP 1056 D9).
    func canvasImageReleased(_ src: String) { on { exact_canvas_image_released(rt, write(src)) } }
    /// A 2D canvas's replay is behind or caught up (LLP 1056 D5).
    func canvasHeld(_ view: UInt32, _ held: Bool) { on { exact_canvas_held(rt, view, held ? 1 : 0) } }
    /// The Canvas 2D text measurer (LLP 1056 D8), with the measurer's context.
    func setCanvasText(_ measure: ExactCanvasTextFn?) { on { exact_set_canvas_text(rt, measure) } }
    /// The replay queue waits here: copying and marking share the runtime owner's
    /// turn, so unmount cannot retire the job between them. Encoding runs afterward.
    func snapshotCanvas(_ ticket: UInt64, copy: () -> Bool) -> Bool {
        on(busy: false) {
            guard exact_request_active(rt, ticket) != 0, copy() else { return false }
            return exact_canvas_snapshotted(rt, ticket) != 0
        }
    }
    /// Canvas draws in a turn of their own (LLP 1072 §8.5).
    func canvasDefer(_ deferred: Bool) { on { exact_canvas_defer(rt, deferred ? 1 : 0) } }
    /// The owed canvas draws on the owner, not waited for (LLP 1072 §8.5).
    func canvasDrawAsync(done: @escaping (RuntimeCompletion) -> Void) {
        post(done: done) { [self] _ in read(exact_canvas_draw(rt)) }
    }
    func canvasPerformanceRequested(_ name: String) -> Bool {
        on(busy: false) { exact_canvas_performance_requested(rt, write(name)) != 0 }
    }
    func canvasPerformanceRecord(_ name: String, _ json: String?) -> Batch {
        on { read(exact_canvas_performance_record(rt, write(name + (json.map { "\0" + $0 } ?? "")))) }
    }
}

extension SurfacesHost {
    func wire(_ session: ExactSession, scale: CGFloat) {
        let runtime = session.runtime, canvas2d = session.presenter.canvas2d
        // LLP 1056 D8, D9: Canvas 2D measures with this engine and draws the
        // handles this session decodes.
        runtime.setCanvasText(CanvasText.measureRun)
        canvas2d.onImage = { [weak session] src, image in
            guard let session, session.state != .destroyed else { return }
            session.apply(session.runtime.canvasImage(src, width: image?.width ?? 0, height: image?.height ?? 0, ok: image != nil))
        }
        canvas2d.onRelease = { [weak session] src in
            guard let session, session.state != .destroyed else { return }
            session.runtime.canvasImageReleased(src)
        }
        canvas2d.onHeld = { [weak session] view, held in
            guard let session, session.state != .destroyed else { return }
            session.runtime.canvasHeld(view, held)
        }
        // LLP 1056 D4: Canvas 2D backs its bitmaps at the display's scale; a
        // window on another display corrects it, and the canvases redraw.
        _ = runtime.canvasDisplay(scale: scale, memory: ProcessInfo.processInfo.physicalMemory)
        canvas2d.onScale = { [weak session] scale in
            DispatchQueue.main.async {
                guard let session, session.state != .destroyed else { return }
                session.apply(session.runtime.canvasDisplay(scale: scale, memory: ProcessInfo.processInfo.physicalMemory))
            }
        }
    }
    func display(_ runtime: Runtime, scale: CGFloat) -> Batch {
        runtime.canvasDisplay(scale: scale, memory: ProcessInfo.processInfo.physicalMemory)
    }
    func deferDraws(_ runtime: Runtime) { runtime.canvasDefer(true) }
    func draw(_ runtime: Runtime, done: @escaping (RuntimeCompletion) -> Void) { runtime.canvasDrawAsync(done: done) }
    func capture(_ session: ExactSession, _ op: [String: Any]) { session.captureCanvas(op) }
}

extension ExactSession {
    func canvasPerformanceRequested(_ name: String) -> Bool {
        state != .destroyed && runtime.canvasPerformanceRequested(name)
    }

    func canvasPerformanceRecord(_ name: String, _ json: String?) {
        publishSurfaceRecord { $0.canvasPerformanceRecord(name, json) }
    }

    /// One capture op (LLP 1056 D9, as amended for expo/exact#188): the
    /// presenter encodes the bitmap it keeps as the type asked for, and the
    /// runtime writes it to the app file and answers the source
    /// (`fulfillSurface` kind 8). A refusal the runtime already made goes back
    /// on the next turn, outside the batch being applied, with nothing to write.
    func captureCanvas(_ op: [String: Any]) {
        let owner = generation, num = { (key: String) in (op[key] as? NSNumber)?.uint64Value ?? 0 }
        let ticket = num("ticket")
        guard op["refused"] == nil else {
            DispatchQueue.main.async { [weak self] in self?.completeSurface(ticket, generation: owner, kind: 8) }
            return
        }
        presenter.canvas2d.capture(UInt32(num("id")), lifetime: num("lifetime"), generation: UInt32(num("generation")), sequence: num("sequence"),
                                   width: Int(num("w")), height: Int(num("h")),
                                   type: op["type"] as? String ?? "image/png", quality: (op["quality"] as? NSNumber)?.doubleValue,
                                   snapshot: { [weak self] copy in
            DispatchQueue.main.sync {
                guard let self, self.state != .destroyed, self.generation == owner else { return false }
                return self.runtime.snapshotCanvas(ticket, copy: copy)
            }
        }) { [weak self] encoded in
            self?.completeSurface(ticket, generation: owner, kind: 8, body: encoded ?? Data())
        }
    }
}
