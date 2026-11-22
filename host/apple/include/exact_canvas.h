/* exact_canvas.h — Canvas 2D's C ABI (LLP 1047.001 D4, D10, §9.21): the
 * display, image handles, held replays, deferred draws, the text measurer,
 * a capture's pixel copy and a canvas's performance record. Its exports are
 * `exact_apple::canvas_exports!`'s, emitted where `surfaces` is linked; the
 * Surfaces product's Swift, and only it, calls them. */
#ifndef EXACT_CANVAS_H
#define EXACT_CANVAS_H

#include "exact.h"

#ifdef __cplusplus
extern "C" {
#endif

/* LLP 1056 D8: one Canvas 2D run measured with Core Text where the draw
 * runs, with the context exact_set_measure was given. The strings live for
 * the call. v: width, left, right, ascent, descent (ink, from the run's left
 * alphabetic origin), font ascent, font descent, em ascent, em descent,
 * hanging, ideographic — CSS px. */
typedef struct ExactCanvasText {
    const uint8_t *text; size_t len;
    const uint8_t *families; size_t families_len; /* names joined by ',' */
    double size, stretch, letter_spacing, word_spacing;
    uint16_t weight;
    uint8_t style;   /* 0 normal, 1 italic, 2 oblique */
    uint8_t caps;    /* fontVariantCaps index */
    uint8_t kerning; /* 0 auto, 1 normal, 2 none */
    uint8_t rtl;
} ExactCanvasText;
typedef struct ExactCanvasMetrics { double v[11]; } ExactCanvasMetrics;
typedef ExactCanvasMetrics (*ExactCanvasTextFn)(void *ctx, const ExactCanvasText *run);
void exact_set_canvas_text(ExactRuntime rt, ExactCanvasTextFn measure);
/* The host owns the pixel copy. Call on the runtime owner before encoding. */
uint8_t exact_canvas_snapshotted(ExactRuntime rt, uint64_t ticket);
uint32_t exact_canvas_performance_requested(ExactRuntime rt, size_t len);
uint32_t exact_canvas_performance_record(ExactRuntime rt, size_t len);
/// The display's scale and physical memory for Canvas 2D (LLP 1056 D4);
/// callable before boot. Returns the batch length.
uint32_t exact_canvas_display(ExactRuntime rt, double scale, double memory);
/* LLP 1056 D9: a Canvas 2D image handle the batch's "canvasImages" named,
 * decoded (ok 1, its size in pixels) or not (ok 0); the handle is the input
 * buffer's first len bytes. Returns the batch length. */
uint32_t exact_canvas_image(ExactRuntime rt, size_t len, uint32_t width, uint32_t height, uint32_t ok);
/* LLP 1056 D9: the presenter let a decoded Canvas 2D image handle go (the
 * input buffer's first len bytes); a canvas that draws it again asks again. */
void exact_canvas_image_released(ExactRuntime rt, size_t len);
/* LLP 1056 D5: a 2D canvas's replay is behind (held 1) or caught up (0);
 * while held, its frame request waits (frames drop rather than queue). */
void exact_canvas_held(ExactRuntime rt, uint32_t view, uint32_t held);
/* LLP 1072 §8.5: canvas draws in a turn of their own (deferred 1): every other
 * turn's batch says "canvasOwed" and exact_canvas_draw runs the draws, off the
 * turns main waits on; a tick draws in its own turn. */
void exact_canvas_defer(ExactRuntime rt, uint32_t deferred);
uint32_t exact_canvas_draw(ExactRuntime rt);

#ifdef __cplusplus
}
#endif

#endif
