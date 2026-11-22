//! Canvas 2D's C exports, which `host!` and `apple_linked` emit for an
//! entry whose set names `surfaces`.

/// Canvas 2D's exports (`include/exact_canvas.h`; LLP 1047.001 §9.21): the
/// display, decoded and released image handles, held replays, deferred
/// draws, the text measurer, captures' pixel copies and performance records,
/// which only the Surfaces product's Swift calls.
#[macro_export]
macro_rules! canvas_exports {
    () => {
        /// The Canvas 2D text measurer (LLP 1056 D8): called on the runtime's
        /// thread with the context `exact_set_measure` was given.
        #[no_mangle]
        pub extern "C" fn exact_set_canvas_text(
            rt: u32,
            measure: ::std::option::Option<$crate::canvas_text::CanvasTextFn>,
        ) {
            $crate::abi::with_entry(&EXACT_RUNTIMES, rt, |e| {
                e.hooks.canvas_text = measure;
            });
        }

        /// A Canvas 2D image handle decoded (LLP 1056 D9): the handle is
        /// the input buffer's first `len` bytes; the batch's length.
        #[no_mangle]
        pub extern "C" fn exact_canvas_image(
            rt: u32,
            len: usize,
            width: u32,
            height: u32,
            ok: u32,
        ) -> u32 {
            $crate::abi::with_runtime(
                &EXACT_RUNTIMES,
                rt,
                false,
                |b, _| b.canvas_image(len, width, height, ok != 0),
                |n| n,
            )
        }

        /// Mark an owned pixel copy before the presenter starts encoding it.
        #[no_mangle]
        pub extern "C" fn exact_canvas_snapshotted(rt: u32, ticket: u64) -> u8 {
            $crate::abi::with_runtime(
                &EXACT_RUNTIMES,
                rt,
                false,
                |b, _| u8::from(b.canvas_snapshotted(ticket)),
                |_| 0,
            )
        }

        /// Whether this canvas name is read by a performance resource.
        #[no_mangle]
        pub extern "C" fn exact_canvas_performance_requested(rt: u32, len: usize) -> u32 {
            $crate::abi::with_runtime(
                &EXACT_RUNTIMES,
                rt,
                false,
                |b, _| b.canvas_performance_requested(len),
                |n| n,
            )
        }
        /// Publish or clear a canvas's performance record.
        #[no_mangle]
        pub extern "C" fn exact_canvas_performance_record(rt: u32, len: usize) -> u32 {
            $crate::abi::with_runtime(
                &EXACT_RUNTIMES,
                rt,
                false,
                |b, _| b.canvas_performance_record(len),
                |n| n,
            )
        }

        /// The display's scale and physical memory for Canvas 2D (LLP 1056
        /// D4); callable before boot. Returns the batch length.
        #[no_mangle]
        pub extern "C" fn exact_canvas_display(rt: u32, scale: f64, memory: f64) -> u32 {
            $crate::abi::with_runtime(
                &EXACT_RUNTIMES,
                rt,
                false,
                |b, _| b.canvas_display(scale, memory),
                |n| n,
            )
        }

        /// A Canvas 2D image handle the presenter let go (LLP 1056 D9): the
        /// handle is the input buffer's first `len` bytes.
        #[no_mangle]
        pub extern "C" fn exact_canvas_image_released(rt: u32, len: usize) {
            $crate::abi::with_runtime(
                &EXACT_RUNTIMES,
                rt,
                false,
                |b, _| b.canvas_image_released(len),
                |_| (),
            )
        }

        /// A 2D canvas's replay is behind (`held` 1) or caught up (0): a
        /// frame request for it waits while held (LLP 1056 D5).
        #[no_mangle]
        pub extern "C" fn exact_canvas_held(rt: u32, view: u32, held: u32) {
            $crate::abi::with_runtime(
                &EXACT_RUNTIMES,
                rt,
                false,
                |b, _| b.canvas_held(view, held != 0),
                |_| (),
            )
        }

        /// Canvas draws in a turn of their own (`deferred` nonzero), or in every
        /// turn (LLP 1072 §8.5): the batch says `canvasOwed` and
        /// `exact_canvas_draw` runs them, off the turns main waits on.
        #[no_mangle]
        pub extern "C" fn exact_canvas_defer(rt: u32, deferred: u32) {
            $crate::abi::with_runtime(
                &EXACT_RUNTIMES,
                rt,
                false,
                |b, _| b.canvas_defer(deferred != 0),
                |_| (),
            )
        }

        /// The canvas draws owed (LLP 1072 §8.5); returns the batch's length.
        #[no_mangle]
        pub extern "C" fn exact_canvas_draw(rt: u32) -> u32 {
            $crate::abi::with_runtime(&EXACT_RUNTIMES, rt, false, |b, _| b.canvas_draw(), |n| n)
        }
    };
}
