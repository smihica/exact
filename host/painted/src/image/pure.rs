//! Pure-Rust JPEG, WebP, and GIF (frame 0) decoding for targets without a
//! platform image decoder (Linux, plus macOS test verification).
//!
//! No system library is linked or `dlopen`'d; every container header and inner
//! bitstream dimension is validated before constructing a codec, and every
//! scratch buffer drops before returning the premultiplied `tiny_skia::Pixmap`.
use super::super::png_decode::{premultiplied, AreaRows, DecodePlan, PlatformFormat};
use super::{is_jpeg, read_all};
use exact_raster::{Refusal, MAX_HEADER_BYTES};
use std::io::{self, BufRead, Cursor, Read, Seek, SeekFrom};
use tiny_skia::{IntSize, Pixmap};
use zune_core::bytestream::ZCursor;
use zune_core::colorspace::ColorSpace;
use zune_core::options::DecoderOptions;

/// Upper bound on progressive JPEG scans accepted in a single decode.
const MAX_JPEG_SCANS: usize = 32;

/// Decode JPEG, WebP, or GIF frame 0 at the plan's size and crop.
pub(in crate::image) fn decode<R: Read + Seek>(
    input: R,
    plan: &DecodePlan,
    cancelled: impl Fn() -> bool,
) -> Result<Pixmap, Refusal> {
    if cancelled() {
        return Err(Refusal::Stale);
    }
    let bytes = read_all(input, plan, &cancelled)?;
    if bytes.len() as u64 != plan.header.metadata.encoded_bytes {
        return Err(Refusal::DecodeFailed);
    }
    match plan.header.format() {
        PlatformFormat::Jpeg | PlatformFormat::JpegProgressive => {
            decode_jpeg(bytes, plan, &cancelled)
        }
        PlatformFormat::Webp | PlatformFormat::WebpAlpha | PlatformFormat::WebpAnimated => {
            decode_webp(bytes, plan, &cancelled)
        }
        PlatformFormat::Gif => decode_gif(bytes, plan, &cancelled),
        PlatformFormat::None => Err(Refusal::DecodeFailed),
    }
}

struct JpegStreamInfo {
    sof_width: u32,
    sof_height: u32,
    sof_components: u8,
    is_progressive: bool,
    scan_count: usize,
    first_scan_components: u8,
    retained_app_bytes: u64,
    has_eoi: bool,
}

/// Walks top-level JPEG markers and byte-stuffed entropy scans to verify SOF
/// dimensions, an explicit EOI marker (`0xFF 0xD9`), retained APP metadata size,
/// and whether zune-jpeg will take its multi-scan full-`i16`-plane path.
fn inspect_jpeg_stream(bytes: &[u8]) -> Option<JpegStreamInfo> {
    if !is_jpeg(bytes) {
        return None;
    }
    let mut pos = 2usize;
    let mut sof_dims: Option<(u32, u32, u8)> = None;
    let mut is_progressive = false;
    let mut scan_count = 0usize;
    let mut first_scan_components = 0u8;
    let mut retained_app_bytes = 0u64;
    let mut has_eoi = false;

    while pos < bytes.len() {
        if bytes[pos] != 0xFF {
            return None;
        }
        while pos < bytes.len() && bytes[pos] == 0xFF {
            pos += 1;
        }
        if pos >= bytes.len() {
            return None;
        }
        let m = bytes[pos];
        pos += 1;
        if m == 0xD9 {
            has_eoi = true;
            break;
        }
        if m == 0xD8 || (0xD0..=0xD7).contains(&m) || m == 0x01 {
            continue;
        }
        if pos + 2 > bytes.len() {
            return None;
        }
        let seg_len = usize::from(u16::from_be_bytes([bytes[pos], bytes[pos + 1]]));
        if seg_len < 2 || pos + seg_len > bytes.len() {
            return None;
        }
        let payload = &bytes[pos + 2..pos + seg_len];
        if matches!(m, 0xE1 | 0xE2 | 0xED) {
            retained_app_bytes = retained_app_bytes.saturating_add(seg_len as u64);
        }
        let sof = (0xC0..=0xCF).contains(&m) && m != 0xC4 && m != 0xC8 && m != 0xCC;
        if sof && sof_dims.is_none() {
            if payload.len() < 6 {
                return None;
            }
            let h = u32::from(u16::from_be_bytes([payload[1], payload[2]]));
            let w = u32::from(u16::from_be_bytes([payload[3], payload[4]]));
            sof_dims = Some((w, h, payload[5]));
            is_progressive = matches!(m, 0xC2 | 0xCA);
        }
        pos += seg_len;
        if m == 0xDA {
            if payload.is_empty() {
                return None;
            }
            if scan_count == 0 {
                first_scan_components = payload[0];
            }
            scan_count += 1;
            if scan_count > MAX_JPEG_SCANS {
                return None;
            }
            // Scan entropy-coded bytes until a non-stuffed, non-restart marker.
            loop {
                let rel = bytes[pos..].iter().position(|&b| b == 0xFF)?;
                pos += rel;
                let marker_start = pos;
                while pos < bytes.len() && bytes[pos] == 0xFF {
                    pos += 1;
                }
                if pos >= bytes.len() {
                    return None;
                }
                let next = bytes[pos];
                if next == 0x00 || (0xD0..=0xD7).contains(&next) {
                    pos += 1;
                    continue;
                }
                // Rewind to the last 0xFF before the marker so the outer loop reads it.
                pos = (pos - 1).max(marker_start);
                break;
            }
        }
    }

    let (sof_width, sof_height, sof_components) = sof_dims?;
    Some(JpegStreamInfo {
        sof_width,
        sof_height,
        sof_components,
        is_progressive,
        scan_count,
        first_scan_components,
        retained_app_bytes,
        has_eoi,
    })
}

/// Drops `APP1` (EXIF/XMP), `APP2` (ICC), and `APP13` (IPTC) segments in place
/// without allocating so `zune-jpeg` cannot retain or reassemble unbounded
/// metadata after SOF. `APP0` (JFIF) and `APP14` (Adobe color transform) are kept.
fn strip_retained_app_markers_in_place(bytes: &mut Vec<u8>) {
    let len = bytes.len();
    let mut read = 2usize;
    let mut write = 2usize;
    while read < len {
        if bytes[read] != 0xFF {
            return;
        }
        while read < len && bytes[read] == 0xFF {
            read += 1;
        }
        if read >= len {
            return;
        }
        let m = bytes[read];
        read += 1;
        if m == 0xD9 {
            bytes[write] = 0xFF;
            bytes[write + 1] = 0xD9;
            write += 2;
            break;
        }
        if m == 0xD8 || (0xD0..=0xD7).contains(&m) || m == 0x01 {
            bytes[write] = 0xFF;
            bytes[write + 1] = m;
            write += 2;
            continue;
        }
        if read + 2 > len {
            return;
        }
        let seg_len = usize::from(u16::from_be_bytes([bytes[read], bytes[read + 1]]));
        if seg_len < 2 || read + seg_len > len {
            return;
        }
        let seg_start = read;
        let seg_end = read + seg_len;
        read = seg_end;
        if matches!(m, 0xE1 | 0xE2 | 0xED) {
            continue;
        }
        bytes[write] = 0xFF;
        bytes[write + 1] = m;
        write += 2;
        if write != seg_start {
            bytes.copy_within(seg_start..seg_end, write);
        }
        write += seg_len;
        if m == 0xDA {
            let scan_start = read;
            loop {
                let Some(rel) = bytes[read..].iter().position(|&b| b == 0xFF) else {
                    return;
                };
                read += rel;
                let marker_start = read;
                while read < len && bytes[read] == 0xFF {
                    read += 1;
                }
                if read >= len {
                    return;
                }
                let next = bytes[read];
                if next == 0x00 || (0xD0..=0xD7).contains(&next) {
                    read += 1;
                    continue;
                }
                read = (read - 1).max(marker_start);
                break;
            }
            let scan_len = read - scan_start;
            if write != scan_start {
                bytes.copy_within(scan_start..read, write);
            }
            write += scan_len;
        }
    }
    bytes.truncate(write);
}

fn decode_jpeg(
    mut bytes: Vec<u8>,
    plan: &DecodePlan,
    cancelled: &impl Fn() -> bool,
) -> Result<Pixmap, Refusal> {
    if !is_jpeg(&bytes) {
        return Err(Refusal::ConflictingMetadata);
    }
    let stream = inspect_jpeg_stream(&bytes).ok_or(Refusal::DecodeFailed)?;
    if !stream.has_eoi || stream.scan_count == 0 {
        return Err(Refusal::DecodeFailed);
    }
    let needs_progressive_planes = stream.is_progressive
        || stream.scan_count != 1
        || stream.first_scan_components != stream.sof_components;
    if needs_progressive_planes && plan.header.format() == PlatformFormat::Jpeg {
        return Err(Refusal::ConflictingMetadata);
    }
    let orientation = plan.header.orientation;
    let (natural_w, natural_h) = plan.natural();
    let (stored_w, stored_h) = if orientation >= 5 {
        (natural_h, natural_w)
    } else {
        (natural_w, natural_h)
    };
    if stream.sof_width != stored_w || stream.sof_height != stored_h {
        return Err(Refusal::ConflictingMetadata);
    }
    // Strip large APP1/APP2/APP13 metadata in place so zune-jpeg cannot
    // duplicate or reassemble multi-megabyte XMP/ICC segments past the plan.
    if stream.retained_app_bytes > MAX_HEADER_BYTES {
        strip_retained_app_markers_in_place(&mut bytes);
    }
    let options = DecoderOptions::default()
        .set_max_width(stored_w as usize)
        .set_max_height(stored_h as usize)
        .set_use_unsafe(false)
        .set_strict_mode(true)
        .jpeg_set_max_scans(MAX_JPEG_SCANS)
        .jpeg_set_out_colorspace(ColorSpace::RGB);
    let mut decoder = zune_jpeg::JpegDecoder::new_with_options(ZCursor::new(&bytes), options);
    decoder
        .decode_headers()
        .map_err(|_| Refusal::DecodeFailed)?;
    // Verify dimensions without calling `decoder.info()`, which clones all retained metadata.
    let expected = (stored_w as usize)
        .checked_mul(stored_h as usize)
        .and_then(|n| n.checked_mul(3))
        .ok_or(Refusal::Overflow)?;
    if decoder.output_buffer_size() != Some(expected) {
        return Err(Refusal::ConflictingMetadata);
    }
    if cancelled() {
        return Err(Refusal::Stale);
    }
    let mut raw = vec![0u8; expected];
    decoder
        .decode_into(&mut raw)
        .map_err(|_| Refusal::DecodeFailed)?;
    // Release decoder state and encoded bytes before allocating output and resampling scratch.
    drop(decoder);
    drop(bytes);
    sample_oriented(
        &raw,
        stored_w,
        stored_h,
        3,
        orientation,
        None,
        plan,
        cancelled,
    )
}

struct WebpValidation {
    /// Synthetic `VP8X` flags when wrapping a 16,384-pixel simple `VP8L` to
    /// bypass `image-webp` 0.2.4's `(1 + header) & 0x3FFF` truncation bug.
    wrap_vp8x: Option<u8>,
    /// Stored-coordinate rectangle of frame 0 when an animated WebP has no
    /// `VP8X` alpha flag and frame 0 does not cover the full canvas; pixels
    /// outside this rectangle must remain transparent to match Chromium.
    opaque_subframe: Option<(u32, u32, u32, u32)>,
}

/// Minimal zero-allocation RFC 6386 §7.3 equiprobable boolean reader used to
/// parse `log2_nbr_of_dct_partitions` and verify VP8 token partition sizes
/// before `image-webp` allocates from them.
struct Vp8BoolReader<'a> {
    data: &'a [u8],
    pos: usize,
    value: u64,
    range: u32,
    bit_count: i32,
    past_end: u8,
}

impl<'a> Vp8BoolReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            value: 0,
            range: 255,
            bit_count: -8,
            past_end: 0,
        }
    }

    fn read_flag(&mut self) -> Result<bool, Refusal> {
        while self.bit_count < 0 {
            if let Some(&b) = self.data.get(self.pos) {
                self.pos += 1;
                self.value = (self.value << 8) | u64::from(b);
                self.bit_count += 8;
            } else if self.past_end == 0 {
                self.past_end = 1;
                self.value <<= 8;
                self.bit_count += 8;
            } else {
                return Err(Refusal::DecodeFailed);
            }
        }
        let split = 1 + (((self.range - 1) * 128) >> 8);
        let bigsplit = u64::from(split) << self.bit_count;
        let bit = if let Some(v) = self.value.checked_sub(bigsplit) {
            self.range -= split;
            self.value = v;
            true
        } else {
            self.range = split;
            false
        };
        let shift = self.range.leading_zeros().saturating_sub(24);
        self.range <<= shift;
        self.bit_count -= shift as i32;
        Ok(bit)
    }

    fn read_literal(&mut self, n: u8) -> Result<u8, Refusal> {
        let mut v = 0u8;
        for _ in 0..n {
            v = (v << 1) | u8::from(self.read_flag()?);
        }
        Ok(v)
    }

    fn skip_optional_signed(&mut self, n: u8) -> Result<(), Refusal> {
        if self.read_flag()? {
            let _ = self.read_literal(n)?;
            let _ = self.read_flag()?;
        }
        Ok(())
    }
}

/// Validates VP8 keyframe dimensions, partition 0 bounds, and every multi-partition
/// size entry against `payload.len()` before `image-webp` 0.2.4 allocates partition
/// buffers (`vp8.rs:init_partitions` allocates from 24-bit sizes before `read_exact`).
fn parse_vp8_dims(payload: &[u8]) -> Result<(u32, u32), Refusal> {
    if payload.len() < 10 {
        return Err(Refusal::DecodeFailed);
    }
    let frame_tag = u32::from_le_bytes([payload[0], payload[1], payload[2], 0]);
    // Frame tag bit 0 == 0 marks a keyframe; bytes 3..6 are the VP8 start code.
    if frame_tag & 1 != 0 || payload[3..6] != [0x9D, 0x01, 0x2A] {
        return Err(Refusal::DecodeFailed);
    }
    let w = u32::from(u16::from_le_bytes([payload[6], payload[7]]) & 0x3FFF);
    let h = u32::from(u16::from_le_bytes([payload[8], payload[9]]) & 0x3FFF);
    if w == 0 || h == 0 {
        return Err(Refusal::InvalidDimensions);
    }
    let first_part_size = (frame_tag >> 5) as usize;
    let part0_end = 10usize
        .checked_add(first_part_size)
        .ok_or(Refusal::Overflow)?;
    if part0_end > payload.len() {
        return Err(Refusal::DecodeFailed);
    }
    let mut br = Vp8BoolReader::new(&payload[10..part0_end]);
    let color_space = br.read_literal(1)?;
    let _pixel_type = br.read_literal(1)?;
    if color_space != 0 {
        return Err(Refusal::DecodeFailed);
    }
    if br.read_flag()? {
        let update_map = br.read_flag()?;
        let update_data = br.read_flag()?;
        if update_data {
            let _abs = br.read_flag()?;
            for _ in 0..4 {
                br.skip_optional_signed(7)?;
            }
            for _ in 0..4 {
                br.skip_optional_signed(6)?;
            }
        }
        if update_map {
            for _ in 0..3 {
                if br.read_flag()? {
                    let _ = br.read_literal(8)?;
                }
            }
        }
    }
    let _filter_type = br.read_flag()?;
    let _filter_level = br.read_literal(6)?;
    let _sharpness_level = br.read_literal(3)?;
    if br.read_flag()? && br.read_flag()? {
        for _ in 0..8 {
            br.skip_optional_signed(6)?;
        }
    }
    let num_partitions = 1usize << usize::from(br.read_literal(2)?);
    if num_partitions > 1 {
        let rest = &payload[part0_end..];
        let table_len = 3 * (num_partitions - 1);
        if rest.len() < table_len {
            return Err(Refusal::DecodeFailed);
        }
        let mut total = table_len;
        for chunk in rest[..table_len].chunks_exact(3) {
            let size = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], 0]) as usize;
            total = total.checked_add(size).ok_or(Refusal::Overflow)?;
            if total > rest.len() {
                return Err(Refusal::DecodeFailed);
            }
        }
    }
    Ok((w, h))
}

fn parse_vp8l_dims(payload: &[u8]) -> Result<(u32, u32, bool), Refusal> {
    if payload.len() < 5 || payload[0] != 0x2F {
        return Err(Refusal::DecodeFailed);
    }
    let bits = u32::from_le_bytes([payload[1], payload[2], payload[3], payload[4]]);
    if bits >> 29 != 0 {
        return Err(Refusal::DecodeFailed);
    }
    let w = (bits & 0x3FFF) + 1;
    let h = ((bits >> 14) & 0x3FFF) + 1;
    let has_alpha = (bits >> 28) & 1 != 0;
    Ok((w, h, has_alpha))
}

fn read_riff_chunk(buf: &[u8], pos: usize) -> Result<([u8; 4], &[u8], usize), Refusal> {
    if pos.checked_add(8).is_none_or(|end| end > buf.len()) {
        return Err(Refusal::DecodeFailed);
    }
    let fourcc: [u8; 4] = buf[pos..pos + 4].try_into().unwrap();
    let len = u32::from_le_bytes(buf[pos + 4..pos + 8].try_into().unwrap()) as usize;
    let payload_end = pos
        .checked_add(8)
        .and_then(|n| n.checked_add(len))
        .ok_or(Refusal::Overflow)?;
    if payload_end > buf.len() {
        return Err(Refusal::DecodeFailed);
    }
    let padded = len.checked_add(len & 1).ok_or(Refusal::Overflow)?;
    let next = pos
        .checked_add(8)
        .and_then(|n| n.checked_add(padded))
        .ok_or(Refusal::Overflow)?;
    Ok((fourcc, &buf[pos + 8..payload_end], next.min(buf.len())))
}

/// Validates RIFF/WEBP container structure and every inner `VP8 `/`VP8L`/`ANMF`
/// bitstream dimension BEFORE constructing `image_webp::WebPDecoder` so
/// malformed inner dimensions can never allocate oversized planes or panic.
fn validate_webp(
    bytes: &[u8],
    stored_w: u32,
    stored_h: u32,
    format: PlatformFormat,
) -> Result<WebpValidation, Refusal> {
    if bytes.len() < 20 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WEBP" {
        return Err(Refusal::ConflictingMetadata);
    }
    let riff_size = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    let riff_end = 8usize
        .checked_add(riff_size)
        .ok_or(Refusal::Overflow)?
        .min(bytes.len());
    if riff_end < 20 {
        return Err(Refusal::DecodeFailed);
    }
    let container = &bytes[..riff_end];
    let (first_cc, first_payload, mut pos) = read_riff_chunk(container, 12)?;
    match &first_cc {
        b"VP8 " => {
            if format != PlatformFormat::Webp {
                return Err(Refusal::ConflictingMetadata);
            }
            let dims = parse_vp8_dims(first_payload)?;
            if dims != (stored_w, stored_h) {
                return Err(Refusal::ConflictingMetadata);
            }
            Ok(WebpValidation {
                wrap_vp8x: None,
                opaque_subframe: None,
            })
        }
        b"VP8L" => {
            if format != PlatformFormat::Webp {
                return Err(Refusal::ConflictingMetadata);
            }
            let (w, h, has_alpha) = parse_vp8l_dims(first_payload)?;
            if (w, h) != (stored_w, stored_h) {
                return Err(Refusal::ConflictingMetadata);
            }
            let wrap_vp8x =
                (w == 16_384 || h == 16_384).then_some(if has_alpha { 0x10 } else { 0 });
            Ok(WebpValidation {
                wrap_vp8x,
                opaque_subframe: None,
            })
        }
        b"VP8X" => {
            if first_payload.len() < 10 {
                return Err(Refusal::DecodeFailed);
            }
            let flags = first_payload[0];
            let animated = flags & 0x02 != 0;
            let has_alpha = flags & 0x10 != 0;
            let expected_fmt = if animated {
                PlatformFormat::WebpAnimated
            } else if has_alpha {
                PlatformFormat::WebpAlpha
            } else {
                PlatformFormat::Webp
            };
            if format != expected_fmt {
                return Err(Refusal::ConflictingMetadata);
            }
            let cw =
                u32::from_le_bytes([first_payload[4], first_payload[5], first_payload[6], 0]) + 1;
            let ch =
                u32::from_le_bytes([first_payload[7], first_payload[8], first_payload[9], 0]) + 1;
            if (cw, ch) != (stored_w, stored_h) {
                return Err(Refusal::ConflictingMetadata);
            }
            let mut saw_image = false;
            let mut opaque_subframe = None;
            while pos + 8 <= container.len() {
                let (cc, payload, next) = read_riff_chunk(container, pos)?;
                pos = next;
                match &cc {
                    b"VP8 " if !animated && !saw_image => {
                        if parse_vp8_dims(payload)? != (stored_w, stored_h) {
                            return Err(Refusal::ConflictingMetadata);
                        }
                        saw_image = true;
                    }
                    b"VP8L" if !animated && !saw_image => {
                        let (w, h, _) = parse_vp8l_dims(payload)?;
                        if (w, h) != (stored_w, stored_h) {
                            return Err(Refusal::ConflictingMetadata);
                        }
                        saw_image = true;
                    }
                    b"ANMF" if animated && !saw_image => {
                        if payload.len() < 16 {
                            return Err(Refusal::DecodeFailed);
                        }
                        let fx = u32::from_le_bytes([payload[0], payload[1], payload[2], 0]) * 2;
                        let fy = u32::from_le_bytes([payload[3], payload[4], payload[5], 0]) * 2;
                        let fw = u32::from_le_bytes([payload[6], payload[7], payload[8], 0]) + 1;
                        let fh = u32::from_le_bytes([payload[9], payload[10], payload[11], 0]) + 1;
                        if fx.checked_add(fw).is_none_or(|r| r > stored_w)
                            || fy.checked_add(fh).is_none_or(|b| b > stored_h)
                        {
                            return Err(Refusal::DecodeFailed);
                        }
                        let sub = &payload[16..];
                        let (sub_cc, sub_payload, sub_next) = read_riff_chunk(sub, 0)?;
                        match &sub_cc {
                            b"VP8 " => {
                                if parse_vp8_dims(sub_payload)? != (fw, fh) {
                                    return Err(Refusal::ConflictingMetadata);
                                }
                            }
                            b"VP8L" => {
                                let (w, h, _) = parse_vp8l_dims(sub_payload)?;
                                if (w, h) != (fw, fh) {
                                    return Err(Refusal::ConflictingMetadata);
                                }
                            }
                            b"ALPH" => {
                                let (img_cc, img_payload, _) = read_riff_chunk(sub, sub_next)?;
                                if &img_cc != b"VP8 " || parse_vp8_dims(img_payload)? != (fw, fh) {
                                    return Err(Refusal::ConflictingMetadata);
                                }
                            }
                            _ => return Err(Refusal::DecodeFailed),
                        }
                        if !has_alpha && (fx != 0 || fy != 0 || fw != stored_w || fh != stored_h) {
                            opaque_subframe = Some((fx, fy, fw, fh));
                        }
                        saw_image = true;
                        break;
                    }
                    _ => {}
                }
            }
            if !saw_image {
                return Err(Refusal::DecodeFailed);
            }
            Ok(WebpValidation {
                wrap_vp8x: None,
                opaque_subframe,
            })
        }
        _ => Err(Refusal::DecodeFailed),
    }
}

/// Zero-copy `Read + Seek` view prepending a 30-byte `RIFF/WEBP/VP8X` header
/// before a simple `VP8L` chunk.
struct PrefixedSlice<'a> {
    prefix: [u8; 30],
    tail: &'a [u8],
    pos: usize,
}

impl<'a> PrefixedSlice<'a> {
    fn vp8x_wrap(bytes: &'a [u8], flags: u8, width: u32, height: u32) -> Result<Self, Refusal> {
        let tail = bytes.get(12..).ok_or(Refusal::DecodeFailed)?;
        let riff_size = u32::try_from(
            4usize
                .checked_add(18)
                .and_then(|n| n.checked_add(tail.len()))
                .ok_or(Refusal::Overflow)?,
        )
        .map_err(|_| Refusal::Overflow)?;
        let w = (width - 1).to_le_bytes();
        let h = (height - 1).to_le_bytes();
        let mut prefix = [0u8; 30];
        prefix[0..4].copy_from_slice(b"RIFF");
        prefix[4..8].copy_from_slice(&riff_size.to_le_bytes());
        prefix[8..12].copy_from_slice(b"WEBP");
        prefix[12..16].copy_from_slice(b"VP8X");
        prefix[16..20].copy_from_slice(&10u32.to_le_bytes());
        prefix[20] = flags;
        prefix[24..27].copy_from_slice(&w[..3]);
        prefix[27..30].copy_from_slice(&h[..3]);
        Ok(Self {
            prefix,
            tail,
            pos: 0,
        })
    }

    fn total_len(&self) -> usize {
        self.prefix.len() + self.tail.len()
    }
}

impl Read for PrefixedSlice<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.pos >= self.total_len() || buf.is_empty() {
            return Ok(0);
        }
        let mut written = 0;
        if self.pos < self.prefix.len() {
            let take = (self.prefix.len() - self.pos).min(buf.len());
            buf[..take].copy_from_slice(&self.prefix[self.pos..self.pos + take]);
            self.pos += take;
            written += take;
        }
        if written < buf.len() && self.pos >= self.prefix.len() {
            let tail_pos = self.pos - self.prefix.len();
            let avail = self.tail.len().saturating_sub(tail_pos);
            let take = avail.min(buf.len() - written);
            buf[written..written + take].copy_from_slice(&self.tail[tail_pos..tail_pos + take]);
            self.pos += take;
            written += take;
        }
        Ok(written)
    }
}

impl Seek for PrefixedSlice<'_> {
    fn seek(&mut self, style: SeekFrom) -> io::Result<u64> {
        let base: i128 = match style {
            SeekFrom::Start(n) => i128::from(n),
            SeekFrom::End(n) => self.total_len() as i128 + i128::from(n),
            SeekFrom::Current(n) => self.pos as i128 + i128::from(n),
        };
        if base < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid seek to negative position",
            ));
        }
        self.pos = usize::try_from(base).unwrap_or(usize::MAX);
        Ok(self.pos as u64)
    }
}

impl BufRead for PrefixedSlice<'_> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.pos < self.prefix.len() {
            Ok(&self.prefix[self.pos..])
        } else {
            let tail_pos = self.pos - self.prefix.len();
            Ok(self.tail.get(tail_pos..).unwrap_or(&[]))
        }
    }

    fn consume(&mut self, amt: usize) {
        self.pos = self.pos.saturating_add(amt).min(self.total_len());
    }
}

fn decode_webp_reader<R: BufRead + Seek>(
    reader: R,
    bytes_len: usize,
    stored_w: u32,
    stored_h: u32,
    plan: &DecodePlan,
    cancelled: &impl Fn() -> bool,
) -> Result<(Vec<u8>, usize), Refusal> {
    let mut decoder = image_webp::WebPDecoder::new(reader).map_err(|_| Refusal::DecodeFailed)?;
    if decoder.dimensions() != (stored_w, stored_h) {
        return Err(Refusal::ConflictingMetadata);
    }
    let channels = if decoder.has_alpha() { 4usize } else { 3usize };
    let expected = (stored_w as usize)
        .checked_mul(stored_h as usize)
        .and_then(|n| n.checked_mul(channels))
        .ok_or(Refusal::Overflow)?;
    if decoder.output_buffer_size() != Some(expected) {
        return Err(Refusal::DecodeFailed);
    }
    let already = (bytes_len as u64).saturating_add(expected as u64);
    let limit =
        usize::try_from(plan.cost.scratch_bytes.saturating_sub(already)).unwrap_or(usize::MAX);
    decoder.set_memory_limit(limit);
    if cancelled() {
        return Err(Refusal::Stale);
    }
    let mut raw = vec![0u8; expected];
    decoder
        .read_image(&mut raw)
        .map_err(|_| Refusal::DecodeFailed)?;
    Ok((raw, channels))
}

fn decode_webp(
    bytes: Vec<u8>,
    plan: &DecodePlan,
    cancelled: &impl Fn() -> bool,
) -> Result<Pixmap, Refusal> {
    let orientation = plan.header.orientation;
    let (natural_w, natural_h) = plan.natural();
    let (stored_w, stored_h) = if orientation >= 5 {
        (natural_h, natural_w)
    } else {
        (natural_w, natural_h)
    };
    let validation = validate_webp(&bytes, stored_w, stored_h, plan.header.format())?;
    let (raw, channels) = if let Some(flags) = validation.wrap_vp8x {
        let reader = PrefixedSlice::vp8x_wrap(&bytes, flags, stored_w, stored_h)?;
        decode_webp_reader(reader, bytes.len(), stored_w, stored_h, plan, cancelled)?
    } else {
        decode_webp_reader(
            Cursor::new(bytes.as_slice()),
            bytes.len(),
            stored_w,
            stored_h,
            plan,
            cancelled,
        )?
    };
    drop(bytes);
    sample_oriented(
        &raw,
        stored_w,
        stored_h,
        channels,
        orientation,
        validation.opaque_subframe,
        plan,
        cancelled,
    )
}

fn decode_gif(
    bytes: Vec<u8>,
    plan: &DecodePlan,
    cancelled: &impl Fn() -> bool,
) -> Result<Pixmap, Refusal> {
    if bytes.len() < 6 || (&bytes[..6] != b"GIF87a" && &bytes[..6] != b"GIF89a") {
        return Err(Refusal::ConflictingMetadata);
    }
    let (w, h) = plan.natural();
    let expected = (w as usize)
        .checked_mul(h as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or(Refusal::Overflow)?;
    let mut opts = gif::DecodeOptions::new();
    opts.set_color_output(gif::ColorOutput::RGBA);
    opts.check_frame_consistency(true);
    // Bound GIF's own frame allocation to the logical screen's RGBA frame plus
    // LZW/palette tables so an oversized frame cannot eat into encoded or canvas budget.
    let frame_limit = (expected as u64).saturating_add(256 * 1024).max(1);
    opts.set_memory_limit(gif::MemoryLimit::Bytes(
        std::num::NonZeroU64::new(frame_limit).expect("non-zero"),
    ));
    let mut decoder = opts
        .read_info(Cursor::new(bytes.as_slice()))
        .map_err(|_| Refusal::DecodeFailed)?;
    if (u32::from(decoder.width()), u32::from(decoder.height())) != (w, h) {
        return Err(Refusal::ConflictingMetadata);
    }
    if cancelled() {
        return Err(Refusal::Stale);
    }
    let frame = decoder
        .read_next_frame()
        .map_err(|_| Refusal::DecodeFailed)?
        .ok_or(Refusal::DecodeFailed)?;
    let left = u32::from(frame.left);
    let top = u32::from(frame.top);
    let fw_u32 = u32::from(frame.width);
    let fh_u32 = u32::from(frame.height);
    if left.checked_add(fw_u32).is_none_or(|r| r > w)
        || top.checked_add(fh_u32).is_none_or(|b| b > h)
    {
        return Err(Refusal::DecodeFailed);
    }
    let full_canvas =
        left == 0 && top == 0 && fw_u32 == w && fh_u32 == h && frame.buffer.len() == expected;
    if full_canvas {
        // Borrow the decoded frame directly without allocating a second canvas.
        let out = sample_oriented(&frame.buffer, w, h, 4, 1, None, plan, cancelled);
        drop(decoder);
        drop(bytes);
        return out;
    }
    let fw = fw_u32 as usize;
    let fh = fh_u32 as usize;
    let frame_expected = fw
        .checked_mul(fh)
        .and_then(|n| n.checked_mul(4))
        .ok_or(Refusal::Overflow)?;
    if frame.buffer.len() != frame_expected {
        return Err(Refusal::DecodeFailed);
    }
    let mut canvas = vec![0u8; expected];
    let left = left as usize;
    let top = top as usize;
    let canvas_w = w as usize;
    if fw > 0 && fh > 0 {
        for y in 0..fh {
            if cancelled() {
                return Err(Refusal::Stale);
            }
            let src_start = y * fw * 4;
            let dst_start = ((top + y) * canvas_w + left) * 4;
            canvas[dst_start..dst_start + fw * 4]
                .copy_from_slice(&frame.buffer[src_start..src_start + fw * 4]);
        }
    }
    drop(decoder);
    drop(bytes);
    sample_oriented(&canvas, w, h, 4, 1, None, plan, cancelled)
}

/// Apply EXIF orientation (1..=8), optional sub-frame transparency mask,
/// premultiplication into `tiny_skia::Pixmap`, and area-downsampling + crop
/// using `png::AreaRows` so cropped decodes match whole decodes bit for bit.
#[allow(clippy::too_many_arguments)]
fn sample_oriented(
    raw: &[u8],
    stored_w: u32,
    stored_h: u32,
    channels: usize,
    orientation: u8,
    opaque_rect: Option<(u32, u32, u32, u32)>,
    plan: &DecodePlan,
    cancelled: &impl Fn() -> bool,
) -> Result<Pixmap, Refusal> {
    let (w, h) = plan.natural();
    let expected = (stored_w as usize)
        .checked_mul(stored_h as usize)
        .and_then(|n| n.checked_mul(channels))
        .ok_or(Refusal::Overflow)?;
    if raw.len() != expected {
        return Err(Refusal::DecodeFailed);
    }
    let size = IntSize::from_wh(plan.pixels.width, plan.pixels.height)
        .ok_or(Refusal::InvalidDimensions)?;
    let mut out = Pixmap::new(size.width(), size.height()).ok_or(Refusal::DecodeFailed)?;
    let full = plan.full();
    let scaled = full != plan.header.metadata.natural;
    let mut area = scaled.then(|| AreaRows::new(plan));
    let needs_row_buf = scaled && (orientation != 1 || opaque_rect.is_some());
    let row_channels = if opaque_rect.is_some() { 4 } else { channels };
    let mut row_buf = needs_row_buf.then(|| vec![0u8; w as usize * row_channels]);
    let stored_stride = stored_w as usize * channels;

    if let Some(area) = &mut area {
        for y in 0..h {
            if cancelled() {
                return Err(Refusal::Stale);
            }
            if orientation == 1 && opaque_rect.is_none() {
                let start = y as usize * stored_stride;
                area.push(
                    &raw[start..start + stored_stride],
                    channels,
                    y,
                    plan,
                    &mut out,
                );
            } else {
                let row = row_buf.as_mut().expect("allocated for orientation/mask");
                fill_oriented_row(
                    raw,
                    stored_w,
                    w,
                    h,
                    channels,
                    orientation,
                    opaque_rect,
                    y,
                    row,
                );
                area.push(row, row_channels, y, plan, &mut out);
            }
        }
    } else {
        // 1:1 (possibly cropped): read only the cropped destination rows directly.
        let out_w = plan.pixels.width as usize;
        let crop_x = plan.crop.x;
        let crop_y = plan.crop.y;
        let dst = out.data_mut();
        for oy in 0..plan.pixels.height {
            if cancelled() {
                return Err(Refusal::Stale);
            }
            let y = crop_y + oy;
            let row_dst = oy as usize * out_w * 4;
            for ox in 0..plan.pixels.width {
                let x = crop_x + ox;
                let (sx, sy) = map_oriented(orientation, x, y, w, h);
                let at = row_dst + ox as usize * 4;
                if let Some((rx, ry, rw, rh)) = opaque_rect {
                    if sx < rx || sx >= rx + rw || sy < ry || sy >= ry + rh {
                        dst[at..at + 4].copy_from_slice(&[0, 0, 0, 0]);
                        continue;
                    }
                }
                let src = (sy as usize * stored_w as usize + sx as usize) * channels;
                dst[at..at + 4].copy_from_slice(&premultiplied(&raw[src..src + channels]));
            }
        }
    }
    if cancelled() {
        return Err(Refusal::Stale);
    }
    Ok(out)
}

#[inline]
fn map_oriented(orientation: u8, x: u32, y: u32, w: u32, h: u32) -> (u32, u32) {
    match orientation {
        2 => (w - 1 - x, y),
        3 => (w - 1 - x, h - 1 - y),
        4 => (x, h - 1 - y),
        5 => (y, x),
        6 => (y, w - 1 - x),
        7 => (h - 1 - y, w - 1 - x),
        8 => (h - 1 - y, x),
        _ => (x, y),
    }
}

#[allow(clippy::too_many_arguments)]
fn fill_oriented_row(
    raw: &[u8],
    stored_w: u32,
    w: u32,
    h: u32,
    channels: usize,
    orientation: u8,
    opaque_rect: Option<(u32, u32, u32, u32)>,
    y: u32,
    row: &mut [u8],
) {
    let sw = stored_w as usize;
    if let Some((rx, ry, rw, rh)) = opaque_rect {
        for x in 0..w {
            let (sx, sy) = map_oriented(orientation, x, y, w, h);
            let dst = x as usize * 4;
            if sx < rx || sx >= rx + rw || sy < ry || sy >= ry + rh {
                row[dst..dst + 4].copy_from_slice(&[0, 0, 0, 0]);
            } else {
                let src = (sy as usize * sw + sx as usize) * channels;
                row[dst] = raw[src];
                row[dst + 1] = raw[src + 1];
                row[dst + 2] = raw[src + 2];
                row[dst + 3] = if channels == 4 { raw[src + 3] } else { 255 };
            }
        }
        return;
    }
    for x in 0..w {
        let (sx, sy) = map_oriented(orientation, x, y, w, h);
        let src = (sy as usize * sw + sx as usize) * channels;
        let dst = x as usize * channels;
        row[dst..dst + channels].copy_from_slice(&raw[src..src + channels]);
    }
}
