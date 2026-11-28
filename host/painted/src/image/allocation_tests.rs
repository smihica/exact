//! Measure requested allocation capacities in the decoder's calling thread.
//! The test allocator is absent from production; encoded fixture bytes exist
//! before tracking begins. Realloc counts old+new until the old block retires.
#![allow(unsafe_code)]
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static LIVE: Cell<usize> = const { Cell::new(0) };
    static PEAK: Cell<usize> = const { Cell::new(0) };
}
struct Observed;
#[global_allocator]
static ALLOCATOR: Observed = Observed;
fn add(n: usize) {
    if TRACK.try_with(Cell::get).unwrap_or(false) {
        LIVE.with(|v| {
            v.set(v.get() + n);
            PEAK.with(|p| p.set(p.get().max(v.get())));
        });
    }
}
fn sub(n: usize) {
    if TRACK.try_with(Cell::get).unwrap_or(false) {
        LIVE.with(|v| v.set(v.get().saturating_sub(n)));
    }
}
// SAFETY: every operation delegates the exact pointer/layout contract to
// System; observations never read or modify the allocation itself.
unsafe impl GlobalAlloc for Observed {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            add(layout.size());
        }
        ptr
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            add(layout.size());
        }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        sub(layout.size());
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        add(size);
        let new = unsafe { System.realloc(ptr, layout, size) };
        if new.is_null() {
            sub(size);
        } else {
            sub(layout.size());
        }
        new
    }
}

pub(crate) fn measure_allocations<T>(f: impl FnOnce() -> T) -> (T, usize, usize) {
    LIVE.with(|n| n.set(0));
    PEAK.with(|n| n.set(0));
    TRACK.with(|t| t.set(true));
    let result = f();
    TRACK.with(|t| t.set(false));
    (result, PEAK.with(Cell::get), LIVE.with(Cell::get))
}

#[test]
fn measured_decoder_capacity_stays_under_reserved_peak_for_tall_wide_and_adam7_pngs() {
    use super::{
        decode_tests::{adam7, png},
        png_decode::{decode_rows, inspect, DecodePlan},
    };
    use std::io::Cursor;
    for (name, bytes, target) in [
        ("large", png(4000, 2000), (100, 50)),
        ("wide", png(80_000, 1), (100, 1)),
        ("tall", png(1, 8192), (1, 100)),
        ("adam7", adam7(19, 11), (7, 5)),
    ] {
        let (header, peak, _) =
            measure_allocations(|| inspect(&mut Cursor::new(&bytes), bytes.len() as u64).unwrap());
        assert_eq!(peak, 0, "metadata inspection allocates nothing");
        let plan = DecodePlan::new(header, target).unwrap();
        let (succeeded, peak, remaining) =
            measure_allocations(|| decode_rows(Cursor::new(&bytes), &plan, || false).is_ok());
        eprintln!(
            "{name}: measured capacity peak={peak}; reserved={}; remaining={remaining}",
            plan.peak_bytes()
        );
        assert!(succeeded, "{name}: decoder must actually produce pixels");
        assert_eq!(remaining, 0);
        assert!(
            peak as u64 <= plan.peak_bytes(),
            "{name}: {peak} > {}",
            plan.peak_bytes()
        );
    }
}

#[cfg(not(any(windows, target_os = "android")))]
#[test]
fn measured_pure_rust_decoder_capacity_stays_under_reserved_peak() {
    use super::{
        jpeg_decode::pure,
        png_decode::{inspect, pure_platform_scratch, DecodePlan},
    };
    use std::io::Cursor;

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let alpine = std::fs::read(root.join("apps/shared-elements/assets/alpine-water.jpg")).unwrap();
    let mut oriented_alpine =
        b"\xff\xd8\xff\xe1\0\x22Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0"
            .to_vec();
    oriented_alpine.extend_from_slice(&alpine[2..]);
    let spin = std::fs::read(root.join("apps/motion-gallery/assets/spin.gif")).unwrap();
    let pulse = std::fs::read(root.join("apps/motion-gallery/assets/pulse.webp")).unwrap();
    let mut lossless_webp = Vec::new();
    let raw = vec![128u8; 640 * 480 * 4];
    image_webp::WebPEncoder::new(&mut lossless_webp)
        .encode(&raw, 640, 480, image_webp::ColorType::Rgba8)
        .unwrap();
    let mut lossless_rgb_webp = Vec::new();
    let raw_rgb = vec![96u8; 640 * 480 * 3];
    image_webp::WebPEncoder::new(&mut lossless_rgb_webp)
        .encode(&raw_rgb, 640, 480, image_webp::ColorType::Rgb8)
        .unwrap();

    // Progressive JPEG (SOF2) encoded via `jpeg-encoder`.
    let mut progressive = Vec::new();
    {
        let mut enc = jpeg_encoder::Encoder::new(&mut progressive, 80);
        enc.set_progressive(true);
        enc.encode(&raw_rgb, 640, 480, jpeg_encoder::ColorType::Rgb)
            .unwrap();
    }

    // Wide, short baseline JPEG (tests padded MCU row workspace).
    let mut wide_short_jpg = Vec::new();
    jpeg_encoder::Encoder::new(&mut wide_short_jpg, 80)
        .encode(
            &vec![120u8; 4096 * 2 * 3],
            4096,
            2,
            jpeg_encoder::ColorType::Rgb,
        )
        .unwrap();

    // Animated lossless RGBA WebP (output + first frame + compositing canvas).
    let mut anim_rgba_frame = Vec::new();
    image_webp::WebPEncoder::new(&mut anim_rgba_frame)
        .encode(
            &vec![180u8; 320 * 240 * 4],
            320,
            240,
            image_webp::ColorType::Rgba8,
        )
        .unwrap();
    let frame_vp8l_len = u32::from_le_bytes(anim_rgba_frame[16..20].try_into().unwrap()) as usize;
    let frame_vp8l_chunk = &anim_rgba_frame[12..20 + frame_vp8l_len];
    let mut anmf = Vec::new();
    let fw = 319u32.to_le_bytes();
    let fh = 239u32.to_le_bytes();
    anmf.extend_from_slice(&[
        0, 0, 0, 0, 0, 0, fw[0], fw[1], fw[2], fh[0], fh[1], fh[2], 100, 0, 0, 0,
    ]);
    anmf.extend_from_slice(frame_vp8l_chunk);
    if frame_vp8l_chunk.len() & 1 == 1 {
        anmf.push(0);
    }
    let mut anim_webp = Vec::new();
    let mut body = Vec::new();
    body.extend_from_slice(b"WEBP");
    let vp8x = [0x12u8, 0, 0, 0, fw[0], fw[1], fw[2], fh[0], fh[1], fh[2]];
    for (cc, data) in [
        (b"VP8X", vp8x.as_slice()),
        (b"ANIM", &[0u8; 6]),
        (b"ANMF", anmf.as_slice()),
    ] {
        body.extend_from_slice(cc);
        body.extend_from_slice(&(data.len() as u32).to_le_bytes());
        body.extend_from_slice(data);
        if data.len() & 1 == 1 {
            body.push(0);
        }
    }
    anim_webp.extend_from_slice(b"RIFF");
    anim_webp.extend_from_slice(&(body.len() as u32).to_le_bytes());
    anim_webp.extend_from_slice(&body);

    // JPEG with >512 KiB of post-SOF extended XMP segments (tests in-place metadata stripping).
    let mut post_sof_xmp_jpg = Vec::new();
    let mut small_jpg = Vec::new();
    jpeg_encoder::Encoder::new(&mut small_jpg, 85)
        .encode(
            &vec![128u8; 64 * 64 * 3],
            64,
            64,
            jpeg_encoder::ColorType::Rgb,
        )
        .unwrap();
    let sos_pos = small_jpg
        .windows(2)
        .position(|w| w == [0xFF, 0xDA])
        .expect("SOS");
    post_sof_xmp_jpg.extend_from_slice(&small_jpg[..sos_pos]);
    let guid = b"0123456789ABCDEF0123456789ABCDEF";
    let total_xmp: u32 = 10 * 50_000;
    for idx in 0..10u32 {
        let mut seg = Vec::with_capacity(35 + 32 + 8 + 50_000);
        seg.extend_from_slice(b"http://ns.adobe.com/xmp/extension/\0");
        seg.extend_from_slice(guid);
        seg.extend_from_slice(&total_xmp.to_be_bytes());
        seg.extend_from_slice(&(idx * 50_000).to_be_bytes());
        seg.resize(seg.len() + 50_000, b'a');
        post_sof_xmp_jpg.extend_from_slice(&[0xFF, 0xE1]);
        post_sof_xmp_jpg.extend_from_slice(&((seg.len() + 2) as u16).to_be_bytes());
        post_sof_xmp_jpg.extend_from_slice(&seg);
    }
    post_sof_xmp_jpg.extend_from_slice(&small_jpg[sos_pos..]);

    // Verify still VP8X-alpha scratch reservation covers VP8 + compressed ALPH peak (>32 MiB at 1792x1792).
    let alpha_1792 = super::png_decode::Header::platform_format(
        exact_raster::Metadata {
            natural: exact_raster::PixelSize {
                width: 1792,
                height: 1792,
            },
            encoded_bytes: 128 * 1024,
            header_bytes: 30,
        },
        super::png_decode::PlatformFormat::WebpAlpha,
    );
    assert!(
        pure_platform_scratch(alpha_1792, (128, 128)).unwrap() > 33 * 1024 * 1024,
        "1792x1792 VP8X-alpha scratch must cover live VP8 + compressed ALPH decode buffers"
    );

    // Verify 16x64,000 progressive JPEG reservation covers 32-pixel MCU sampling factors (4x1).
    let prog_tall = super::png_decode::Header::platform_format(
        exact_raster::Metadata {
            natural: exact_raster::PixelSize {
                width: 16,
                height: 64_000,
            },
            encoded_bytes: 256 * 1024,
            header_bytes: 64,
        },
        super::png_decode::PlatformFormat::JpegProgressive,
    );
    assert!(
        pure_platform_scratch(prog_tall, (16, 1024)).unwrap() >= 32 * 64_000 * 8 + 16 * 64_000 * 3,
        "progressive scratch must align to 32-pixel MCU boundaries for 4x sampling factors"
    );

    for (name, bytes, target) in [
        ("jpeg-baseline", &alpine, (128, 96)),
        ("jpeg-exif-6", &oriented_alpine, (96, 128)),
        ("jpeg-progressive", &progressive, (128, 96)),
        ("jpeg-wide-short", &wide_short_jpg, (512, 1)),
        ("jpeg-post-sof-xmp", &post_sof_xmp_jpg, (64, 64)),
        ("gif", &spin, (48, 48)),
        ("webp-lossy", &pulse, (48, 48)),
        ("webp-lossless-rgba", &lossless_webp, (128, 96)),
        ("webp-lossless-rgb", &lossless_rgb_webp, (128, 96)),
        ("webp-animated-lossless-rgba", &anim_webp, (128, 96)),
    ] {
        let header = inspect(&mut Cursor::new(bytes), bytes.len() as u64).unwrap();
        let target = (
            target.0.min(header.metadata.natural.width),
            target.1.min(header.metadata.natural.height),
        );
        let plan = DecodePlan::new(header, target).unwrap();
        let reserved_scratch = pure_platform_scratch(header, target).unwrap();
        let reserved_peak = reserved_scratch + plan.output_bytes();
        let (succeeded, peak, remaining) =
            measure_allocations(|| pure::decode(Cursor::new(bytes), &plan, || false).is_ok());
        eprintln!(
            "{name}: measured capacity peak={peak}; pure reserved={reserved_peak}; remaining={remaining}"
        );
        assert!(succeeded, "{name}: pure-Rust decoder must produce pixels");
        assert_eq!(remaining, 0, "{name}: scratch must drop before returning");
        assert!(
            peak as u64 <= reserved_peak,
            "{name}: measured peak {peak} > reserved {reserved_peak}"
        );
    }
}
