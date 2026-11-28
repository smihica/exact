//! JPEG, GIF and WebP: the header read here (the frame's SOF marker, the GIF
//! screen descriptor, the WebP frame header; no decoder), the pixels by the
//! platform's decoder where there is one — Android's `AImageDecoder`
//! (libjpeg-turbo, giflib, libwebp; sampled to the plan's size while
//! decoding, as the platform's own image views decode; an animation's first
//! frame), Windows' WIC, macOS's ImageIO, and on Linux pure-Rust decoders
//! (`pure.rs`: `zune-jpeg`, `image-webp`, `gif` frame 0). Before Android 11
//! (API 30, `AImageDecoder`'s) the same decoder through the reader's
//! `ImageDecoder` (`crate::canvas::legacy`). On Android, an animated GIF or
//! WebP plays in the platform's own animated drawable, which a reader may draw
//! in the still's place ([`crate::image::Bitmap::animation_file`]).
//!
//! @ref LLP 1076 §3.4 (the Rust painter on Android)
use super::png_decode::{DecodePlan, Header, PlatformFormat};
use exact_raster::{Metadata, PixelSize, Refusal, MAX_HEADER_BYTES, MAX_SOURCE_PIXELS};
use std::io::{Read, Seek, SeekFrom};
use tiny_skia::Pixmap;

/// Whether these leading bytes are a JPEG's SOI.
pub(super) fn is_jpeg(head: &[u8]) -> bool {
    head.len() >= 3 && head[0] == 0xFF && head[1] == 0xD8 && head[2] == 0xFF
}

/// A GIF's or WebP's header from its leading bytes (plus any WebP `EXIF`
/// orientation chunk within `MAX_HEADER_BYTES`), or `None` for neither.
pub(super) fn inspect_animated(
    head: &[u8],
    input: &mut (impl Read + Seek),
    encoded_bytes: u64,
) -> Result<Option<Header>, Refusal> {
    let le16 = |i: usize| u32::from(u16::from_le_bytes([head[i], head[i + 1]]));
    let le24 = |i: usize| u32::from_le_bytes([head[i], head[i + 1], head[i + 2], 0]);
    let mut orientation = 1u8;
    let (width, height, format, header_len) =
        if head.len() >= 10 && (&head[..6] == b"GIF87a" || &head[..6] == b"GIF89a") {
            (le16(6), le16(8), PlatformFormat::Gif, 10u64)
        } else if head.len() >= 16 && &head[..4] == b"RIFF" && &head[8..12] == b"WEBP" {
            match &head[12..16] {
                b"VP8 " if head.len() >= 30 => (
                    le16(26) & 0x3FFF,
                    le16(28) & 0x3FFF,
                    PlatformFormat::Webp,
                    30u64,
                ),
                b"VP8L" if head.len() >= 25 && head[20] == 0x2F => {
                    let bits = u32::from_le_bytes([head[21], head[22], head[23], head[24]]);
                    (
                        (bits & 0x3FFF) + 1,
                        ((bits >> 14) & 0x3FFF) + 1,
                        PlatformFormat::Webp,
                        25u64,
                    )
                }
                b"VP8X" if head.len() >= 30 => {
                    let flags = head[20];
                    let format = if flags & 0x02 != 0 {
                        PlatformFormat::WebpAnimated
                    } else if flags & 0x10 != 0 {
                        PlatformFormat::WebpAlpha
                    } else {
                        PlatformFormat::Webp
                    };
                    if flags & 0x08 != 0 {
                        let riff_size =
                            u64::from(u32::from_le_bytes([head[4], head[5], head[6], head[7]]));
                        let riff_end = 8u64.saturating_add(riff_size).min(encoded_bytes);
                        if let Some(value) = webp_exif_orientation(input, riff_end) {
                            orientation = value;
                        }
                    }
                    (le24(24) + 1, le24(27) + 1, format, 30u64)
                }
                _ => return Err(Refusal::DecodeFailed),
            }
        } else {
            return Ok(None);
        };
    if width == 0 || height == 0 {
        return Err(Refusal::InvalidDimensions);
    }
    if u64::from(width) * u64::from(height) > MAX_SOURCE_PIXELS {
        return Err(Refusal::SourcePixels);
    }
    Ok(Some(
        Header::platform_format(
            Metadata {
                natural: PixelSize { width, height },
                encoded_bytes,
                header_bytes: header_len.min(encoded_bytes),
            },
            format,
        )
        .oriented(orientation),
    ))
}

/// Read the `EXIF` chunk's TIFF orientation from a `VP8X` WebP without reading
/// pixel payloads, bounding total chunk-header + EXIF bytes read by `MAX_HEADER_BYTES`.
fn webp_exif_orientation(input: &mut (impl Read + Seek), encoded_bytes: u64) -> Option<u8> {
    input.seek(SeekFrom::Start(12)).ok()?;
    let mut pos = 12u64;
    let mut read_budget = MAX_HEADER_BYTES;
    while pos + 8 <= encoded_bytes && read_budget >= 8 {
        let mut hdr = [0u8; 8];
        input.read_exact(&mut hdr).ok()?;
        read_budget -= 8;
        let len = u64::from(u32::from_le_bytes(hdr[4..8].try_into().ok()?));
        let padded = len.checked_add(len & 1)?;
        let payload_start = pos.checked_add(8)?;
        let next = payload_start.checked_add(padded)?;
        if payload_start.checked_add(len)? > encoded_bytes {
            return None;
        }
        if &hdr[..4] == b"EXIF" {
            if len == 0 || len > read_budget {
                return None;
            }
            let mut data = vec![0u8; usize::try_from(len).ok()?];
            input.read_exact(&mut data).ok()?;
            return exif_orientation_tiff(data.strip_prefix(b"Exif\0\0").unwrap_or(&data));
        }
        if next > encoded_bytes {
            return None;
        }
        pos = next;
        input.seek(SeekFrom::Start(pos)).ok()?;
    }
    None
}

/// The natural size from the first frame header, reading at most
/// `MAX_HEADER_BYTES`.
pub(super) fn inspect(
    input: &mut (impl Read + Seek),
    encoded_bytes: u64,
) -> Result<Header, Refusal> {
    input
        .seek(SeekFrom::Start(2))
        .map_err(|_| Refusal::DecodeFailed)?;
    let mut at = 2u64;
    let mut orientation = 1;
    loop {
        if at + 4 > MAX_HEADER_BYTES || at + 4 > encoded_bytes {
            return Err(Refusal::HeaderLimit);
        }
        let mut marker = [0u8; 2];
        input
            .read_exact(&mut marker)
            .map_err(|_| Refusal::DecodeFailed)?;
        at += 2;
        if marker[0] != 0xFF {
            return Err(Refusal::DecodeFailed);
        }
        let m = marker[1];
        if m == 0xFF {
            // Fill byte: the marker follows.
            input
                .seek(SeekFrom::Current(-1))
                .map_err(|_| Refusal::DecodeFailed)?;
            at -= 1;
            continue;
        }
        if m == 0xD8 || (0xD0..=0xD7).contains(&m) || m == 0x01 {
            continue;
        }
        if m == 0xD9 || m == 0xDA {
            return Err(Refusal::DecodeFailed);
        }
        let mut len = [0u8; 2];
        input
            .read_exact(&mut len)
            .map_err(|_| Refusal::DecodeFailed)?;
        let len = u64::from(u16::from_be_bytes(len));
        if len < 2 {
            return Err(Refusal::DecodeFailed);
        }
        if at + len > MAX_HEADER_BYTES || at + len > encoded_bytes {
            return Err(Refusal::HeaderLimit);
        }
        if m == 0xE1 {
            let mut data = vec![0; (len - 2) as usize];
            input
                .read_exact(&mut data)
                .map_err(|_| Refusal::DecodeFailed)?;
            if let Some(value) = exif_orientation(&data) {
                orientation = value;
            }
        }
        let sof = (0xC0..=0xCF).contains(&m) && m != 0xC4 && m != 0xC8 && m != 0xCC;
        if sof {
            let mut f = [0u8; 6];
            input
                .read_exact(&mut f)
                .map_err(|_| Refusal::DecodeFailed)?;
            let height = u32::from(u16::from_be_bytes([f[1], f[2]]));
            let width = u32::from(u16::from_be_bytes([f[3], f[4]]));
            if width == 0 || height == 0 {
                return Err(Refusal::InvalidDimensions);
            }
            if u64::from(width) * u64::from(height) > MAX_SOURCE_PIXELS {
                return Err(Refusal::SourcePixels);
            }
            let num_components = f[5];
            let format = if matches!(m, 0xC2 | 0xCA) {
                PlatformFormat::JpegProgressive
            } else if jpeg_first_scan_is_complete(input, at + len, encoded_bytes, num_components) {
                PlatformFormat::Jpeg
            } else {
                // Non-interleaved multi-scan baseline (or SOS past MAX_HEADER_BYTES):
                // zune-jpeg allocates full padded i16 planes just like progressive.
                PlatformFormat::JpegProgressive
            };
            let header = Header::platform_format(
                Metadata {
                    natural: PixelSize { width, height },
                    encoded_bytes,
                    header_bytes: at + len,
                },
                format,
            );
            // The picture's size is the upright one: WIC and the pure-Rust path
            // turn the decode by it, and Android/macOS decoders turn it themselves.
            let header = header.oriented(orientation);
            return Ok(header);
        }
        at += len;
        input
            .seek(SeekFrom::Start(at))
            .map_err(|_| Refusal::DecodeFailed)?;
    }
}

/// Checks whether the first SOS marker includes all `num_components` of a
/// baseline SOF (single-scan interleaved). Seeks past intervening segment
/// payloads (such as large post-SOF XMP) while bounding bytes read by
/// `MAX_HEADER_BYTES`, so baseline photos with large metadata after SOF are
/// not misclassified as progressive.
fn jpeg_first_scan_is_complete(
    input: &mut (impl Read + Seek),
    mut at: u64,
    encoded_bytes: u64,
    num_components: u8,
) -> bool {
    if num_components == 0 || input.seek(SeekFrom::Start(at)).is_err() {
        return false;
    }
    let mut read_budget = MAX_HEADER_BYTES;
    loop {
        if at + 4 > encoded_bytes || read_budget < 4 {
            return false;
        }
        let mut marker = [0u8; 2];
        if input.read_exact(&mut marker).is_err() || marker[0] != 0xFF {
            return false;
        }
        at += 2;
        read_budget -= 2;
        let m = marker[1];
        if m == 0xFF {
            if input.seek(SeekFrom::Current(-1)).is_err() {
                return false;
            }
            at -= 1;
            continue;
        }
        if m == 0xD8 || (0xD0..=0xD7).contains(&m) || m == 0x01 {
            continue;
        }
        if m == 0xD9 {
            return false;
        }
        let mut len_bytes = [0u8; 2];
        if input.read_exact(&mut len_bytes).is_err() {
            return false;
        }
        read_budget -= 2;
        let len = u64::from(u16::from_be_bytes(len_bytes));
        if len < 2 || at.checked_add(len).is_none_or(|end| end > encoded_bytes) {
            return false;
        }
        if m == 0xDA {
            if len < 3 || read_budget == 0 {
                return false;
            }
            let mut ns = [0u8; 1];
            return input.read_exact(&mut ns).is_ok() && ns[0] == num_components;
        }
        at += len;
        if input.seek(SeekFrom::Start(at)).is_err() {
            return false;
        }
    }
}

fn exif_orientation(bytes: &[u8]) -> Option<u8> {
    exif_orientation_tiff(bytes.strip_prefix(b"Exif\0\0")?)
}

fn exif_orientation_tiff(tiff: &[u8]) -> Option<u8> {
    let little = match tiff.get(..4)? {
        b"II\x2a\0" => true,
        b"MM\0\x2a" => false,
        _ => return None,
    };
    let u16_at = |bytes: &[u8]| {
        let pair = [bytes[0], bytes[1]];
        if little {
            u16::from_le_bytes(pair)
        } else {
            u16::from_be_bytes(pair)
        }
    };
    let offset: [u8; 4] = tiff.get(4..8)?.try_into().ok()?;
    let offset = if little {
        u32::from_le_bytes(offset)
    } else {
        u32::from_be_bytes(offset)
    };
    let ifd = tiff.get(usize::try_from(offset).ok()?..)?;
    let count = usize::from(u16_at(ifd.get(..2)?));
    let entries = ifd.get(2..)?.get(..count * 12)?;
    for entry in entries.as_chunks::<12>().0 {
        if u16_at(entry) != 0x112 || u16_at(&entry[2..]) != 3 {
            continue;
        }
        let count: [u8; 4] = entry[4..8].try_into().ok()?;
        let count = if little {
            u32::from_le_bytes(count)
        } else {
            u32::from_be_bytes(count)
        };
        if count != 1 {
            return None;
        }
        let value = u16_at(&entry[8..]);
        return (1..=8).contains(&value).then_some(value as u8);
    }
    None
}

#[cfg(test)]
#[test]
fn exif_orientation_checks_endianness_types_offsets_and_truncation() {
    let little = b"Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0";
    let big = b"Exif\0\0MM\0\x2a\0\0\0\x08\0\x01\x01\x12\0\x03\0\0\0\x01\0\x06\0\0\0\0\0\0";
    for bytes in [little, big] {
        assert_eq!(exif_orientation(bytes), Some(6));
        for len in 0..28 {
            assert_eq!(exif_orientation(&bytes[..len]), None, "prefix {len}");
        }
    }
    for (at, value) in [(10, 255), (16, 255), (18, 4), (20, 2), (24, 9)] {
        let mut invalid = *little;
        invalid[at] = value;
        assert_eq!(exif_orientation(&invalid), None, "byte {at}");
    }
    for orientation in 1..=8 {
        let mut bytes = *little;
        bytes[24] = orientation;
        assert_eq!(exif_orientation(&bytes), Some(orientation));
    }
}

#[cfg(target_os = "android")]
#[allow(unsafe_code)]
mod platform {
    use std::ffi::{c_void, CStr};
    use std::sync::OnceLock;
    /// `ARect` (`<android/rect.h>`): left, top, right, bottom.
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct ARect(pub i32, pub i32, pub i32, pub i32);

    type Create = unsafe extern "C" fn(*const c_void, usize, *mut *mut c_void) -> i32;
    type SetInt = unsafe extern "C" fn(*mut c_void, i32) -> i32;
    type SetSize = unsafe extern "C" fn(*mut c_void, i32, i32) -> i32;
    type SetCrop = unsafe extern "C" fn(*mut c_void, ARect) -> i32;
    type Decode = unsafe extern "C" fn(*mut c_void, *mut c_void, usize, usize) -> i32;
    type Delete = unsafe extern "C" fn(*mut c_void);

    /// `AImageDecoder`'s functions: `createFromBuffer`, `setAndroidBitmapFormat`,
    /// `setTargetSize`, `setCrop`, `decodeImage`, `delete`.
    pub struct Decoder {
        pub create: Create,
        pub format: SetInt,
        pub target: SetSize,
        pub crop: SetCrop,
        pub decode: Decode,
        pub delete: Delete,
    }

    /// The platform decoder, looked up at run time: `AImageDecoder` came with
    /// API 30, and a library that imported it would not load on Android 10.
    pub fn decoder() -> Option<&'static Decoder> {
        static DECODER: OnceLock<Option<Decoder>> = OnceLock::new();
        // SAFETY: libjnigraphics is a public NDK library; dlopen loads it or
        // takes a reference. Each symbol is cast to its NDK signature.
        DECODER
            .get_or_init(|| unsafe {
                let lib = libc::dlopen(c"libjnigraphics.so".as_ptr(), libc::RTLD_NOW);
                if lib.is_null() {
                    return None;
                }
                let symbol = |name: &CStr| {
                    let at = libc::dlsym(lib, name.as_ptr());
                    (!at.is_null()).then_some(at)
                };
                Some(Decoder {
                    create: std::mem::transmute::<*mut c_void, Create>(symbol(
                        c"AImageDecoder_createFromBuffer",
                    )?),
                    format: std::mem::transmute::<*mut c_void, SetInt>(symbol(
                        c"AImageDecoder_setAndroidBitmapFormat",
                    )?),
                    target: std::mem::transmute::<*mut c_void, SetSize>(symbol(
                        c"AImageDecoder_setTargetSize",
                    )?),
                    crop: std::mem::transmute::<*mut c_void, SetCrop>(symbol(
                        c"AImageDecoder_setCrop",
                    )?),
                    decode: std::mem::transmute::<*mut c_void, Decode>(symbol(
                        c"AImageDecoder_decodeImage",
                    )?),
                    delete: std::mem::transmute::<*mut c_void, Delete>(symbol(
                        c"AImageDecoder_delete",
                    )?),
                })
            })
            .as_ref()
    }
}

/// Whether `AImageDecoder` is the platform's (Android 11, API 30, and later).
#[cfg(target_os = "android")]
pub(crate) fn platform_decoder() -> bool {
    platform::decoder().is_some()
}

/// The whole file (it is bounded by `MAX_ENCODED_BYTES`): the platform or
/// pure-Rust decoder takes a buffer.
fn read_all<R: Read + Seek>(
    mut input: R,
    plan: &DecodePlan,
    cancelled: impl Fn() -> bool,
) -> Result<Vec<u8>, Refusal> {
    input
        .seek(SeekFrom::Start(0))
        .map_err(|_| Refusal::DecodeFailed)?;
    let mut bytes = Vec::with_capacity(plan.header.metadata.encoded_bytes as usize);
    input
        .take(plan.header.metadata.encoded_bytes)
        .read_to_end(&mut bytes)
        .map_err(|_| Refusal::DecodeFailed)?;
    if cancelled() {
        return Err(Refusal::Stale);
    }
    Ok(bytes)
}

/// Decode `bytes` at the plan's size into `pixels` (`len` bytes, rows
/// `stride` apart): the whole picture, or the part of it the plan is
/// (`DecodePlan::part`; the decoder scales to the whole's size, then crops,
/// so the part's pixels are the whole's). Whether every row was written.
///
/// The platform decoder samples by a power of two in the DCT and scales the
/// rest of the way itself (bilinear, Skia's vector code), as an image loader
/// asks it to: cheaper than decoding at the power of two and resampling here
/// (heavy's probe: -7% of every thread's instructions).
///
/// # Safety
/// `pixels` is writable for `len` bytes, `len` ≥ `stride × h`, `stride` ≥ `4w`.
#[cfg(target_os = "android")]
#[allow(unsafe_code)]
unsafe fn decode_into(
    bytes: &[u8],
    plan: &DecodePlan,
    pixels: *mut std::ffi::c_void,
    stride: usize,
    len: usize,
) -> bool {
    use platform::ARect;
    let (full, part, out) = (plan.full(), plan.crop, plan.pixels);
    let Some(d) = platform::decoder() else {
        let crop = [part.x, part.y, out.width, out.height];
        return crate::canvas::legacy::decode(
            bytes,
            (full.width, full.height),
            crop,
            pixels,
            stride,
            len,
        );
    };
    crate::android::section_begin(c"exact platform decode");
    struct End;
    impl Drop for End {
        fn drop(&mut self) {
            crate::android::section_end();
        }
    }
    let _end = End;
    let mut decoder = std::ptr::null_mut();
    // The buffer outlives the decoder, which is deleted on every path.
    if (d.create)(bytes.as_ptr().cast(), bytes.len(), &mut decoder) != 0 {
        return false;
    }
    let cut = ARect(
        part.x as i32,
        part.y as i32,
        (part.x + out.width) as i32,
        (part.y + out.height) as i32,
    );
    let ok = (d.format)(decoder, 1) == 0
        && (d.target)(decoder, full.width as i32, full.height as i32) == 0
        && (part.whole() || (d.crop)(decoder, cut) == 0)
        && (d.decode)(decoder, pixels, stride, len) == 0;
    (d.delete)(decoder);
    ok
}

/// Decode at the plan's size, onto the heap.
#[cfg(target_os = "android")]
#[allow(unsafe_code)]
pub(super) fn decode<R: Read + Seek>(
    input: R,
    plan: &DecodePlan,
    cancelled: impl Fn() -> bool,
) -> Result<Pixmap, Refusal> {
    let bytes = read_all(input, plan, cancelled)?;
    let (w, h) = (plan.pixels.width, plan.pixels.height);
    let stride = w as usize * 4;
    let len = stride * h as usize;
    // Not zeroed first: the decoder writes every byte of it, and zeroing a
    // picture's megabytes cost a fifth of the decode threads' time (heavy).
    let mut out: Vec<u8> = Vec::with_capacity(len);
    // SAFETY: the output's capacity is `stride × h` bytes, as the decoder is
    // told, and its length is set only after it has written all of them.
    unsafe {
        if !decode_into(&bytes, plan, out.as_mut_ptr().cast(), stride, len) {
            return Err(Refusal::DecodeFailed);
        }
        out.set_len(len);
    }
    tiny_skia::IntSize::from_wh(w, h)
        .and_then(|size| Pixmap::from_vec(out, size))
        .ok_or(Refusal::DecodeFailed)
}

/// A picture's pixels for [`super::Bitmap`]: a platform-decoded one straight
/// into a GPU buffer where a host's reader draws from those
/// ([`super::hardware_pictures`]), anything else onto the heap.
pub(super) fn decode_pixels<R: Read + Seek>(
    input: R,
    plan: &DecodePlan,
    _variant: u32,
    cancelled: impl Fn() -> bool,
) -> Result<super::bitmap::Pixels, Refusal> {
    #[cfg(target_os = "android")]
    #[allow(unsafe_code)]
    if _variant == exact_raster::variant::HARDWARE_SRGB8 {
        let bytes = read_all(input, plan, cancelled)?;
        let size = (plan.pixels.width, plan.pixels.height);
        // SAFETY: `filled` hands a buffer of `len` bytes, rows `stride` apart.
        return super::hardware::Hardware::filled(size.0, size.1, |pixels, stride, len| unsafe {
            decode_into(&bytes, plan, pixels, stride, len)
        })
        .map(super::bitmap::Pixels::Hardware)
        .ok_or(Refusal::DecodeFailed);
    }
    super::png_decode::decode_rows(input, plan, cancelled).map(Into::into)
}

#[cfg(windows)]
#[path = "windows.rs"]
mod windows;
#[cfg(all(windows, test))]
pub(super) use windows::observed;
#[cfg(windows)]
pub(super) use windows::{decode, Worker};

/// ImageIO decodes and orients the thumbnail on the raster worker.
#[cfg(target_os = "macos")]
pub(super) fn decode<R: Read + Seek>(
    input: R,
    plan: &DecodePlan,
    cancelled: impl Fn() -> bool,
) -> Result<Pixmap, Refusal> {
    let bytes = read_all(input, plan, &cancelled)?;
    let full = plan.full();
    let pixels = exact_ink::image::apple::decode(
        &bytes,
        [full.width, full.height],
        [
            plan.crop.x,
            plan.crop.y,
            plan.pixels.width,
            plan.pixels.height,
        ],
    )
    .ok_or(Refusal::DecodeFailed)?;
    if cancelled() {
        Err(Refusal::Stale)
    } else {
        Ok(pixels)
    }
}

#[cfg(any(
    not(any(target_os = "android", windows, target_os = "macos")),
    all(target_os = "macos", test)
))]
#[path = "pure.rs"]
pub(super) mod pure;

/// Pure-Rust decoder on Linux and other non-Apple/Windows/Android targets.
#[cfg(not(any(target_os = "android", windows, target_os = "macos")))]
pub(super) use pure::decode;
