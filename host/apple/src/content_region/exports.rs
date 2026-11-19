//! The content-region family's C exports, which `host!` and `apple_linked`
//! emit for an entry whose set names `content_regions`.

/// The content-region family's exports (`include/exact_content_regions.h`;
/// LLP 1047.001 §9.11): a registered region's text requests, completions and
/// fallback, which only the family's Swift calls.
#[macro_export]
macro_rules! content_region_exports {
    () => {
        /// Copy current region source/paint metadata. No returned bytes outlive exact_out.
        #[no_mangle]
        pub extern "C" fn exact_region_request(rt: u32, id: u64, known_source: u64) -> u32 {
            $crate::abi::with_runtime(
                &EXACT_RUNTIMES,
                rt,
                false,
                |b, _| b.region_request(id, known_source),
                |n| n,
            )
        }
        /// Takes one native retain on every path, including destroyed/busy runtimes.
        #[no_mangle]
        pub extern "C" fn exact_region_complete(
            rt: u32,
            id: u64,
            metrics: $crate::measure::CMetrics,
            owner: *mut ::std::ffi::c_void,
            release: $crate::content_region::RegionRelease,
        ) -> u32 {
            let retained = ::std::rc::Rc::new($crate::content_region::NativeRegionOwner::new(
                owner, release,
            ));
            $crate::abi::with_runtime(
                &EXACT_RUNTIMES,
                rt,
                false,
                |b, _| b.region_complete(id, metrics, retained),
                |n| n,
            )
        }
        /// Return only a current worker request to native layout.
        #[no_mangle]
        pub extern "C" fn exact_region_fallback(rt: u32, id: u64) -> u32 {
            $crate::abi::with_runtime(
                &EXACT_RUNTIMES,
                rt,
                false,
                |b, _| b.region_fallback(id),
                |n| n,
            )
        }
    };
}
