use super::png_decode::{decode_rows, inspect, DecodePlan};
use std::io::Cursor;

#[cfg(windows)]
#[test]
fn windows_decodes_bitmaps_and_the_largest_icon_representation() {
    fn decode(bytes: &[u8], size: (u32, u32)) -> tiny_skia::Pixmap {
        let header = inspect(&mut Cursor::new(bytes), bytes.len() as u64).unwrap();
        let plan = DecodePlan::new(header, size).unwrap();
        assert_eq!(plan.natural(), size);
        decode_rows(Cursor::new(bytes), &plan, || false).unwrap()
    }

    // One opaque RGB(17,34,51) pixel in a 24-bit BMP and a 32-bit DIB icon.
    let bmp = super::assets::data_url("data:image/bmp;base64,Qk06AAAAAAAAADYAAAAoAAAAAQAAAAEAAAABABgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAMyIRAA==").unwrap();
    let dib = super::assets::data_url("data:image/x-icon;base64,AAABAAEAAQEAAAEAIAAwAAAAFgAAACgAAAABAAAAAgAAAAEAIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAzIhH/AAAAAA==").unwrap();
    for bytes in [&bmp, &dib] {
        assert_eq!(decode(bytes, (1, 1)).data(), &[17, 34, 51, 255]);
    }

    let png = png(4, 4);
    let mut icon = vec![0, 0, 1, 0, 2, 0];
    let mut offset = 38u32;
    for (side, size) in [(1, dib.len() - 22), (4, png.len())] {
        icon.extend([side, side, 0, 0, 1, 0, 32, 0]);
        icon.extend((size as u32).to_le_bytes());
        icon.extend(offset.to_le_bytes());
        offset += size as u32;
    }
    icon.extend_from_slice(&dib[22..]);
    icon.extend_from_slice(&png);
    assert_eq!(decode(&icon, (4, 4)).data(), decode(&png, (4, 4)).data());
    icon.pop();
    assert!(matches!(
        inspect(&mut Cursor::new(&icon), icon.len() as u64),
        Err(exact_raster::Refusal::DecodeFailed)
    ));
}

#[cfg(windows)]
#[test]
fn windows_applies_jpeg_exif_orientation_to_dimensions_pixels_and_crops() {
    let source = include_bytes!("../../../../apps/shared-elements/assets/alpine-water.jpg");
    let header = inspect(&mut Cursor::new(source), source.len() as u64).unwrap();
    let natural = header.metadata.natural;
    let base_plan = DecodePlan::new(header, (natural.width, natural.height)).unwrap();
    let base = decode_rows(Cursor::new(source), &base_plan, || false).unwrap();
    for orientation in [6u8, 1, 2, 3, 4, 5, 7, 8] {
        // One little-endian SHORT in IFD0, inserted before the JPEG's existing segments.
        let mut bytes =
            b"\xff\xd8\xff\xe1\0\x22Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0"
                .to_vec();
        bytes.extend([orientation, 0, 0, 0, 0, 0, 0, 0]);
        bytes.extend_from_slice(&source[2..]);
        let header = inspect(&mut Cursor::new(&bytes), bytes.len() as u64).unwrap();
        let (w, h) = if orientation >= 5 {
            (natural.height, natural.width)
        } else {
            (natural.width, natural.height)
        };
        assert_eq!(
            (
                header.metadata.natural.width,
                header.metadata.natural.height
            ),
            (w, h),
            "orientation {orientation}"
        );
        let plan = DecodePlan::new(header, (w, h)).unwrap();
        let image = decode_rows(Cursor::new(&bytes), &plan, || false).unwrap();
        for (x, y) in [
            (0, 0),
            (w - 1, 0),
            (0, h - 1),
            (w - 1, h - 1),
            (w / 3, h / 4),
        ] {
            let (sx, sy) = match orientation {
                2 => (w - 1 - x, y),
                3 => (w - 1 - x, h - 1 - y),
                4 => (x, h - 1 - y),
                5 => (y, x),
                6 => (y, w - 1 - x),
                7 => (h - 1 - y, w - 1 - x),
                8 => (h - 1 - y, x),
                _ => (x, y),
            };
            assert_eq!(
                image.pixel(x, y),
                base.pixel(sx, sy),
                "orientation {orientation} at {x},{y}"
            );
        }
        let crop = plan.part(w / 3, h / 4, (5, 7)).unwrap();
        let cropped = decode_rows(Cursor::new(&bytes), &crop, || false).unwrap();
        for y in 0..7 {
            for x in 0..5 {
                assert_eq!(
                    cropped.pixel(x, y),
                    image.pixel(w / 3 + x, h / 4 + y),
                    "orientation {orientation}"
                );
            }
        }
    }
}

#[cfg(windows)]
#[test]
fn windows_decodes_photos_and_animation_first_frames_at_the_requested_size() {
    for path in [
        "apps/shared-elements/assets/alpine-water.jpg",
        "apps/motion-gallery/assets/spin.gif",
        "apps/motion-gallery/assets/pulse.webp",
    ] {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let bytes = std::fs::read(root.join(path)).unwrap();
        let header = inspect(&mut Cursor::new(&bytes), bytes.len() as u64).unwrap();
        let size = (
            header.metadata.natural.width.min(64),
            header.metadata.natural.height.min(64),
        );
        let plan = DecodePlan::new(header, size).unwrap();
        let decoded = decode_rows(Cursor::new(&bytes), &plan, || false)
            .unwrap_or_else(|e| panic!("{path}: {e:?}"));
        assert_eq!((decoded.width(), decoded.height()), size, "{path}");
        assert!(
            decoded.data().as_chunks::<4>().0.iter().any(|p| p[3] > 0),
            "{path}"
        );
        let part = plan.part(2, 3, (size.0 - 4, size.1 - 6)).unwrap();
        let cropped = decode_rows(Cursor::new(&bytes), &part, || false).unwrap();
        for row in 0..cropped.height() {
            let from = (((row + 3) * size.0 + 2) * 4) as usize;
            let len = cropped.width() as usize * 4;
            assert_eq!(
                &cropped.data()[row as usize * len..(row as usize + 1) * len],
                &decoded.data()[from..from + len],
                "{path} row {row}"
            );
        }
        assert!(matches!(
            decode_rows(Cursor::new(&bytes), &plan, || true),
            Err(exact_raster::Refusal::Stale)
        ));
    }
}

#[cfg(not(any(windows, target_os = "android")))]
#[test]
fn pure_rust_decodes_jpeg_webp_and_gif_first_frames_with_crop_parity_and_cancellation() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for path in [
        "apps/shared-elements/assets/alpine-water.jpg",
        "apps/motion-gallery/assets/spin.gif",
        "apps/motion-gallery/assets/pulse.webp",
    ] {
        let bytes = std::fs::read(root.join(path)).unwrap();
        let header = inspect(&mut Cursor::new(&bytes), bytes.len() as u64).unwrap();
        let size = (
            header.metadata.natural.width.min(64),
            header.metadata.natural.height.min(64),
        );
        let plan = DecodePlan::new(header, size).unwrap();
        let decoded = super::jpeg_decode::pure::decode(Cursor::new(&bytes), &plan, || false)
            .unwrap_or_else(|e| panic!("{path}: {e:?}"));
        assert_eq!((decoded.width(), decoded.height()), size, "{path}");
        assert!(
            decoded.data().as_chunks::<4>().0.iter().any(|p| p[3] > 0),
            "{path}"
        );
        for p in decoded.data().as_chunks::<4>().0 {
            assert!(
                p[0] <= p[3] && p[1] <= p[3] && p[2] <= p[3],
                "{path}: non-premultiplied pixel {p:?}"
            );
        }
        let part = plan.part(2, 3, (size.0 - 4, size.1 - 6)).unwrap();
        let cropped =
            super::jpeg_decode::pure::decode(Cursor::new(&bytes), &part, || false).unwrap();
        for row in 0..cropped.height() {
            let from = (((row + 3) * size.0 + 2) * 4) as usize;
            let len = cropped.width() as usize * 4;
            assert_eq!(
                &cropped.data()[row as usize * len..(row as usize + 1) * len],
                &decoded.data()[from..from + len],
                "{path} row {row}"
            );
        }
        assert!(matches!(
            super::jpeg_decode::pure::decode(Cursor::new(&bytes), &plan, || true),
            Err(exact_raster::Refusal::Stale)
        ));
        // Truncated data must fail instead of returning a gray/partial frame.
        let truncated = &bytes[..bytes.len() / 2];
        assert!(matches!(
            super::jpeg_decode::pure::decode(Cursor::new(truncated), &plan, || false),
            Err(exact_raster::Refusal::DecodeFailed)
        ));
    }

    // Sub-rect GIF frame 0: composite onto the full logical screen canvas.
    let mut sub_gif = Vec::new();
    {
        let mut enc = gif::Encoder::new(&mut sub_gif, 8, 6, &[0, 0, 0, 255, 0, 0]).unwrap();
        let mut frame = gif::Frame::from_indexed_pixels(2, 2, [1u8; 4], None);
        frame.left = 3;
        frame.top = 2;
        enc.write_frame(&frame).unwrap();
    }
    let header = inspect(&mut Cursor::new(&sub_gif), sub_gif.len() as u64).unwrap();
    assert_eq!(
        (
            header.metadata.natural.width,
            header.metadata.natural.height
        ),
        (8, 6)
    );
    let plan = DecodePlan::new(header, (8, 6)).unwrap();
    let img = super::jpeg_decode::pure::decode(Cursor::new(&sub_gif), &plan, || false).unwrap();
    assert_eq!(img.pixel(0, 0).unwrap().alpha(), 0);
    assert_eq!(
        img.pixel(3, 2).unwrap(),
        tiny_skia::PremultipliedColorU8::from_rgba(255, 0, 0, 255).unwrap()
    );
    assert_eq!(img.pixel(5, 4).unwrap().alpha(), 0);

    // Lossless WebP with alpha (semi-transparent + hidden RGB under alpha=0).
    let mut webp = Vec::new();
    let raw = [
        200u8, 100, 50, 128, 0, 255, 0, 0, 255, 0, 0, 255, 0, 0, 255, 255,
    ];
    image_webp::WebPEncoder::new(&mut webp)
        .encode(&raw, 2, 2, image_webp::ColorType::Rgba8)
        .unwrap();
    let header = inspect(&mut Cursor::new(&webp), webp.len() as u64).unwrap();
    let plan = DecodePlan::new(header, (2, 2)).unwrap();
    let img = super::jpeg_decode::pure::decode(Cursor::new(&webp), &plan, || false).unwrap();
    assert_eq!(
        img.pixel(0, 0).unwrap(),
        tiny_skia::PremultipliedColorU8::from_rgba(100, 50, 25, 128).unwrap()
    );
    assert_eq!(
        img.pixel(1, 0).unwrap(),
        tiny_skia::PremultipliedColorU8::from_rgba(0, 0, 0, 0).unwrap()
    );
    let scaled_plan = DecodePlan::new(header, (1, 1)).unwrap();
    let scaled =
        super::jpeg_decode::pure::decode(Cursor::new(&webp), &scaled_plan, || false).unwrap();
    for p in scaled.data().as_chunks::<4>().0 {
        assert!(p[0] <= p[3] && p[1] <= p[3] && p[2] <= p[3]);
    }

    // Grayscale JPEG decodes cleanly to RGB -> opaque premultiplied RGBA.
    let mut gray_jpg = Vec::new();
    jpeg_encoder::Encoder::new(&mut gray_jpg, 90)
        .encode(&[200u8; 8 * 8], 8, 8, jpeg_encoder::ColorType::Luma)
        .unwrap();
    let header = inspect(&mut Cursor::new(&gray_jpg), gray_jpg.len() as u64).unwrap();
    let plan = DecodePlan::new(header, (8, 8)).unwrap();
    let gray = super::jpeg_decode::pure::decode(Cursor::new(&gray_jpg), &plan, || false).unwrap();
    let px = gray.pixel(4, 4).unwrap();
    assert_eq!(px.alpha(), 255);
    assert!(px.red().abs_diff(200) <= 2 && px.red() == px.green() && px.green() == px.blue());

    // Codec-level truncation on single-frame JPEG, WebP, and GIF (where `encoded_bytes`
    // matches the truncated buffer length) must return `DecodeFailed`.
    let alpine = std::fs::read(root.join("apps/shared-elements/assets/alpine-water.jpg")).unwrap();
    for (label, truncated) in [
        ("jpeg", &alpine[..alpine.len() / 2]),
        ("webp", &webp[..webp.len().saturating_sub(4)]),
        ("gif", &sub_gif[..sub_gif.len().saturating_sub(4)]),
    ] {
        if let Ok(header) = inspect(&mut Cursor::new(truncated), truncated.len() as u64) {
            let plan = DecodePlan::new(
                header,
                (
                    header.metadata.natural.width.min(32),
                    header.metadata.natural.height.min(32),
                ),
            )
            .unwrap();
            assert!(
                matches!(
                    super::jpeg_decode::pure::decode(Cursor::new(truncated), &plan, || false),
                    Err(exact_raster::Refusal::DecodeFailed)
                ),
                "truncated {label} must fail decoding"
            );
        }
    }

    // Missing EOI (`0xFF 0xD9`) on a baseline JPEG must fail even though
    // zune-jpeg's baseline decoder ignores EOF while scanning for EOI.
    assert_eq!(&gray_jpg[gray_jpg.len() - 2..], &[0xFF, 0xD9]);
    let no_eoi = &gray_jpg[..gray_jpg.len() - 2];
    let no_eoi_header = inspect(&mut Cursor::new(no_eoi), no_eoi.len() as u64).unwrap();
    let no_eoi_plan = DecodePlan::new(no_eoi_header, (8, 8)).unwrap();
    assert!(matches!(
        super::jpeg_decode::pure::decode(Cursor::new(no_eoi), &no_eoi_plan, || false),
        Err(exact_raster::Refusal::DecodeFailed)
    ));

    // Off-canvas GIF frame must fail cleanly rather than panicking on canvas copy.
    let mut off_canvas_gif = sub_gif.clone();
    let sep = off_canvas_gif
        .iter()
        .position(|&b| b == 0x2C)
        .expect("image descriptor");
    off_canvas_gif[sep + 1..sep + 3].copy_from_slice(&20u16.to_le_bytes());
    let off_header = inspect(
        &mut Cursor::new(&off_canvas_gif),
        off_canvas_gif.len() as u64,
    )
    .unwrap();
    let off_plan = DecodePlan::new(off_header, (8, 6)).unwrap();
    assert!(matches!(
        super::jpeg_decode::pure::decode(Cursor::new(&off_canvas_gif), &off_plan, || false),
        Err(exact_raster::Refusal::DecodeFailed)
    ));

    // Helper to build a RIFF/WEBP container from explicit chunks.
    let build_riff = |chunks: &[(&[u8; 4], &[u8])]| -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(b"WEBP");
        for (cc, data) in chunks {
            body.extend_from_slice(*cc);
            body.extend_from_slice(&(data.len() as u32).to_le_bytes());
            body.extend_from_slice(data);
            if data.len() & 1 == 1 {
                body.push(0);
            }
        }
        let mut out = Vec::with_capacity(8 + body.len());
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(&body);
        out
    };
    let vp8x_payload = |flags: u8, w: u32, h: u32| -> [u8; 10] {
        let wb = (w - 1).to_le_bytes();
        let hb = (h - 1).to_le_bytes();
        [flags, 0, 0, 0, wb[0], wb[1], wb[2], hb[0], hb[1], hb[2]]
    };

    // WebP inner dimension mismatch (`VP8X` 16x16 vs inner `VP8L` 32x32) must
    // refuse before `image-webp` allocates.
    let mut webp_32 = Vec::new();
    image_webp::WebPEncoder::new(&mut webp_32)
        .encode(
            &vec![64u8; 32 * 32 * 3],
            32,
            32,
            image_webp::ColorType::Rgb8,
        )
        .unwrap();
    let vp8l_32_len = u32::from_le_bytes(webp_32[16..20].try_into().unwrap()) as usize;
    let vp8l_32_data = &webp_32[20..20 + vp8l_32_len];
    let bad_vp8x = vp8x_payload(0x00, 16, 16);
    let mismatched_webp = build_riff(&[(b"VP8X", &bad_vp8x), (b"VP8L", vp8l_32_data)]);
    let mis_header = inspect(
        &mut Cursor::new(&mismatched_webp),
        mismatched_webp.len() as u64,
    )
    .unwrap();
    let mis_plan = DecodePlan::new(mis_header, (16, 16)).unwrap();
    assert!(matches!(
        super::jpeg_decode::pure::decode(Cursor::new(&mismatched_webp), &mis_plan, || false),
        Err(exact_raster::Refusal::ConflictingMetadata)
    ));

    // WebP `EXIF` orientation 6 swaps natural dimensions and rotates pixels.
    let mut webp_4x2 = Vec::new();
    let pixels_4x2 = [
        255u8, 0, 0, 255, 10, 20, 30, 255, 40, 50, 60, 255, 0, 255, 0,
        255, // row 0: red .. green
        0, 0, 255, 255, 70, 80, 90, 255, 100, 110, 120, 255, 255, 255, 255,
        255, // row 1: blue .. white
    ];
    image_webp::WebPEncoder::new(&mut webp_4x2)
        .encode(&pixels_4x2, 4, 2, image_webp::ColorType::Rgba8)
        .unwrap();
    let vp8l_4x2_len = u32::from_le_bytes(webp_4x2[16..20].try_into().unwrap()) as usize;
    let vp8l_4x2_data = &webp_4x2[20..20 + vp8l_4x2_len];
    let exif_vp8x = vp8x_payload(0x18, 4, 2); // alpha | exif
    let tiff_rot6 = b"II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0";
    let oriented_webp = build_riff(&[
        (b"VP8X", &exif_vp8x),
        (b"VP8L", vp8l_4x2_data),
        (b"EXIF", tiff_rot6),
    ]);
    let ow_header = inspect(&mut Cursor::new(&oriented_webp), oriented_webp.len() as u64).unwrap();
    assert_eq!(
        (
            ow_header.metadata.natural.width,
            ow_header.metadata.natural.height
        ),
        (2, 4)
    );
    let ow_plan = DecodePlan::new(ow_header, (2, 4)).unwrap();
    let ow_img =
        super::jpeg_decode::pure::decode(Cursor::new(&oriented_webp), &ow_plan, || false).unwrap();
    // Orientation 6 (90° CW): stored (0,0) red -> (1,0), stored (0,1) blue -> (0,0), stored (3,0) green -> (1,3).
    assert_eq!(
        ow_img.pixel(1, 0).unwrap(),
        tiny_skia::PremultipliedColorU8::from_rgba(255, 0, 0, 255).unwrap()
    );
    assert_eq!(
        ow_img.pixel(0, 0).unwrap(),
        tiny_skia::PremultipliedColorU8::from_rgba(0, 0, 255, 255).unwrap()
    );
    assert_eq!(
        ow_img.pixel(1, 3).unwrap(),
        tiny_skia::PremultipliedColorU8::from_rgba(0, 255, 0, 255).unwrap()
    );

    // Animated WebP without `VP8X` alpha flag whose frame 0 is a sub-frame must
    // leave uncovered canvas pixels transparent (not opaque black), at 1:1 and scaled.
    let mut sub_rgb_webp = Vec::new();
    image_webp::WebPEncoder::new(&mut sub_rgb_webp)
        .encode(
            &[255, 0, 0, 255, 0, 0, 255, 0, 0, 255, 0, 0],
            2,
            2,
            image_webp::ColorType::Rgb8,
        )
        .unwrap();
    let sub_vp8l_len = u32::from_le_bytes(sub_rgb_webp[16..20].try_into().unwrap()) as usize;
    let sub_vp8l_chunk = &sub_rgb_webp[12..20 + sub_vp8l_len];
    let mut anmf = Vec::new();
    // x = 2 (stored as x/2 = 1), y = 2 (y/2 = 1), w = 2 (w-1 = 1), h = 2 (h-1 = 1), duration = 100, flags = 2 (no blend)
    anmf.extend_from_slice(&[1, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 100, 0, 0, 2]);
    anmf.extend_from_slice(sub_vp8l_chunk);
    if sub_vp8l_chunk.len() & 1 == 1 {
        anmf.push(0);
    }
    let anim_vp8x = vp8x_payload(0x02, 6, 6); // ANIM only, alpha bit clear
    let anim_subframe = build_riff(&[
        (b"VP8X", &anim_vp8x),
        (b"ANIM", &[0u8; 6]),
        (b"ANMF", &anmf),
    ]);
    let as_header = inspect(&mut Cursor::new(&anim_subframe), anim_subframe.len() as u64).unwrap();
    let as_plan = DecodePlan::new(as_header, (6, 6)).unwrap();
    let as_img =
        super::jpeg_decode::pure::decode(Cursor::new(&anim_subframe), &as_plan, || false).unwrap();
    assert_eq!(as_img.pixel(0, 0).unwrap().alpha(), 0);
    assert_eq!(
        as_img.pixel(2, 2).unwrap(),
        tiny_skia::PremultipliedColorU8::from_rgba(255, 0, 0, 255).unwrap()
    );
    let as_scaled_plan = DecodePlan::new(as_header, (3, 3)).unwrap();
    let as_scaled =
        super::jpeg_decode::pure::decode(Cursor::new(&anim_subframe), &as_scaled_plan, || false)
            .unwrap();
    assert_eq!(as_scaled.pixel(0, 0).unwrap().alpha(), 0);
    assert_eq!(
        as_scaled.pixel(1, 1).unwrap(),
        tiny_skia::PremultipliedColorU8::from_rgba(255, 0, 0, 255).unwrap()
    );

    // Legal 16,384-pixel edge in simple lossless WebP (`VP8L`) decodes cleanly.
    let mut wide_vp8l = Vec::new();
    let wide_rgb: Vec<u8> = [20u8, 180, 90]
        .into_iter()
        .cycle()
        .take(16_384 * 3)
        .collect();
    image_webp::WebPEncoder::new(&mut wide_vp8l)
        .encode(&wide_rgb, 16_384, 1, image_webp::ColorType::Rgb8)
        .unwrap();
    assert_eq!(&wide_vp8l[12..16], b"VP8L");
    let wide_header = inspect(&mut Cursor::new(&wide_vp8l), wide_vp8l.len() as u64).unwrap();
    assert_eq!(
        (
            wide_header.metadata.natural.width,
            wide_header.metadata.natural.height
        ),
        (16_384, 1)
    );
    let wide_plan = DecodePlan::new(wide_header, (64, 1)).unwrap();
    let wide_img =
        super::jpeg_decode::pure::decode(Cursor::new(&wide_vp8l), &wide_plan, || false).unwrap();
    assert_eq!(
        wide_img.pixel(0, 0).unwrap(),
        tiny_skia::PremultipliedColorU8::from_rgba(20, 180, 90, 255).unwrap()
    );

    // Valid short (28-byte, <30) 1x1 simple lossless `VP8L` WebP must inspect and decode.
    let tiny_vp8l: [u8; 28] = [
        b'R', b'I', b'F', b'F', 20, 0, 0, 0, b'W', b'E', b'B', b'P', b'V', b'P', b'8', b'L', 8, 0,
        0, 0, 0x2F, 0x00, 0x00, 0x00, 0x10, 0x88, 0x88, 0x08,
    ];
    let tiny_header = inspect(&mut Cursor::new(&tiny_vp8l), tiny_vp8l.len() as u64).unwrap();
    assert_eq!(
        (
            tiny_header.metadata.natural.width,
            tiny_header.metadata.natural.height
        ),
        (1, 1)
    );
    let tiny_plan = DecodePlan::new(tiny_header, (1, 1)).unwrap();
    let tiny_img =
        super::jpeg_decode::pure::decode(Cursor::new(&tiny_vp8l), &tiny_plan, || false).unwrap();
    assert_eq!(
        tiny_img.pixel(0, 0).unwrap(),
        tiny_skia::PremultipliedColorU8::from_rgba(0, 0, 0, 0).unwrap()
    );

    // Long animated WebP (>1,024 chunks) with trailing `EXIF` chunk keeps EXIF orientation.
    let mut long_chunks: Vec<(&[u8; 4], &[u8])> = Vec::with_capacity(1103);
    let long_vp8x = vp8x_payload(0x0A, 4, 2); // ANIM | EXIF
    let anim_cfg = [0u8; 6];
    let mut frame_4x2_anmf = Vec::new();
    frame_4x2_anmf.extend_from_slice(&[0, 0, 0, 0, 0, 0, 3, 0, 0, 1, 0, 0, 10, 0, 0, 2]);
    frame_4x2_anmf.extend_from_slice(&webp_4x2[12..20 + vp8l_4x2_len]);
    if (8 + vp8l_4x2_len) & 1 == 1 {
        frame_4x2_anmf.push(0);
    }
    long_chunks.push((b"VP8X", &long_vp8x));
    long_chunks.push((b"ANIM", &anim_cfg));
    for _ in 0..1100 {
        long_chunks.push((b"ANMF", &frame_4x2_anmf));
    }
    long_chunks.push((b"EXIF", tiff_rot6));
    let long_anim = build_riff(&long_chunks);
    let long_header = inspect(&mut Cursor::new(&long_anim), long_anim.len() as u64).unwrap();
    assert_eq!(
        (
            long_header.metadata.natural.width,
            long_header.metadata.natural.height
        ),
        (2, 4),
        "trailing EXIF after >1,024 ANMF chunks must still orient natural dimensions"
    );

    // JPEG with >256 KiB of post-SOF APP1 XMP metadata decodes cleanly after in-place stripping.
    let sos_pos = gray_jpg
        .windows(2)
        .position(|w| w == [0xFF, 0xDA])
        .expect("SOS marker");
    let mut bloated_jpg = Vec::with_capacity(gray_jpg.len() + 400_000);
    bloated_jpg.extend_from_slice(&gray_jpg[..sos_pos]);
    let xmp_chunk_payload = vec![b'x'; 50_000];
    for _ in 0..8 {
        bloated_jpg.extend_from_slice(&[0xFF, 0xE1]);
        bloated_jpg.extend_from_slice(&((xmp_chunk_payload.len() + 2) as u16).to_be_bytes());
        bloated_jpg.extend_from_slice(&xmp_chunk_payload);
    }
    bloated_jpg.extend_from_slice(&gray_jpg[sos_pos..]);
    let bloated_header = inspect(&mut Cursor::new(&bloated_jpg), bloated_jpg.len() as u64).unwrap();
    assert_eq!(
        bloated_header.format(),
        super::png_decode::PlatformFormat::Jpeg,
        "post-SOF XMP must not misclassify single-scan baseline JPEG as progressive"
    );
    let bloated_plan = DecodePlan::new(bloated_header, (8, 8)).unwrap();
    let bloated_img =
        super::jpeg_decode::pure::decode(Cursor::new(&bloated_jpg), &bloated_plan, || false)
            .unwrap();
    assert_eq!(bloated_img.data(), gray.data());

    // Crafted VP8 frame declaring 2 token partitions with a bogus 16 MiB size
    // entry must be rejected in preflight before `image-webp` allocates from it.
    let mut bad_vp8_payload = Vec::new();
    // Keyframe tag with first_part_size = 6: (6 << 5) = 0xC0 -> [0xC0, 0x00, 0x00]
    bad_vp8_payload.extend_from_slice(&[0xC0, 0x00, 0x00, 0x9D, 0x01, 0x2A, 16, 0, 16, 0]);
    // 6-byte partition 0 where all 14 header bits before `log2_nbr_of_dct_partitions`
    // are 0, followed by 2-bit `log2_nbr_of_dct_partitions = 1` (num_partitions = 2).
    bad_vp8_payload.extend_from_slice(&[0x00, 0x01, 0x00, 0x00, 0x00, 0x00]);
    // Bogus 24-bit partition 0 size = 0xFFFFFF (~16 MiB) with no payload bytes behind it.
    bad_vp8_payload.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0x00]);
    let bad_vp8 = build_riff(&[(b"VP8 ", &bad_vp8_payload)]);
    let bad_vp8_header = inspect(&mut Cursor::new(&bad_vp8), bad_vp8.len() as u64).unwrap();
    let bad_vp8_plan = DecodePlan::new(bad_vp8_header, (16, 16)).unwrap();
    assert!(matches!(
        super::jpeg_decode::pure::decode(Cursor::new(&bad_vp8), &bad_vp8_plan, || false),
        Err(exact_raster::Refusal::DecodeFailed)
    ));
}

#[cfg(not(any(windows, target_os = "android")))]
#[test]
fn pure_rust_applies_jpeg_exif_orientation_to_dimensions_pixels_and_crops() {
    let source = include_bytes!("../../../../apps/shared-elements/assets/alpine-water.jpg");
    let header = inspect(&mut Cursor::new(source), source.len() as u64).unwrap();
    let natural = header.metadata.natural;
    let base_plan = DecodePlan::new(header, (natural.width, natural.height)).unwrap();
    let base = super::jpeg_decode::pure::decode(Cursor::new(source), &base_plan, || false).unwrap();
    for orientation in [6u8, 1, 2, 3, 4, 5, 7, 8] {
        let mut bytes =
            b"\xff\xd8\xff\xe1\0\x22Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0"
                .to_vec();
        bytes.extend([orientation, 0, 0, 0, 0, 0, 0, 0]);
        bytes.extend_from_slice(&source[2..]);
        let header = inspect(&mut Cursor::new(&bytes), bytes.len() as u64).unwrap();
        let (w, h) = if orientation >= 5 {
            (natural.height, natural.width)
        } else {
            (natural.width, natural.height)
        };
        assert_eq!(
            (
                header.metadata.natural.width,
                header.metadata.natural.height
            ),
            (w, h),
            "orientation {orientation}"
        );
        let plan = DecodePlan::new(header, (w, h)).unwrap();
        let image = super::jpeg_decode::pure::decode(Cursor::new(&bytes), &plan, || false).unwrap();
        for (x, y) in [
            (0, 0),
            (w - 1, 0),
            (0, h - 1),
            (w - 1, h - 1),
            (w / 3, h / 4),
        ] {
            let (sx, sy) = match orientation {
                2 => (w - 1 - x, y),
                3 => (w - 1 - x, h - 1 - y),
                4 => (x, h - 1 - y),
                5 => (y, x),
                6 => (y, w - 1 - x),
                7 => (h - 1 - y, w - 1 - x),
                8 => (h - 1 - y, x),
                _ => (x, y),
            };
            assert_eq!(
                image.pixel(x, y),
                base.pixel(sx, sy),
                "orientation {orientation} at {x},{y}"
            );
        }
        // Scaled + cropped oriented decode must match the whole scaled oriented decode.
        let small = DecodePlan::new(header, (w / 4, h / 4)).unwrap();
        let whole_small =
            super::jpeg_decode::pure::decode(Cursor::new(&bytes), &small, || false).unwrap();
        let crop = small.part(w / 12, h / 12, (7, 9)).unwrap();
        let cropped =
            super::jpeg_decode::pure::decode(Cursor::new(&bytes), &crop, || false).unwrap();
        for y in 0..9 {
            for x in 0..7 {
                assert_eq!(
                    cropped.pixel(x, y),
                    whole_small.pixel(w / 12 + x, h / 12 + y),
                    "scaled crop orientation {orientation} at {x},{y}"
                );
            }
        }
    }
}

#[cfg(not(any(windows, target_os = "android")))]
#[test]
fn planner_keeps_requested_size_when_scratch_exceeds_available_budget() {
    let source = include_bytes!("../../../../apps/shared-elements/assets/alpine-water.jpg");
    let header = inspect(&mut Cursor::new(source), source.len() as u64).unwrap();
    let plan = super::plan(header, (200., 150.), 1024).unwrap();
    assert!(
        plan.pixels.width > 1 && plan.pixels.height > 1,
        "planner must not crush a large image to 1x1 when even 1x1 exceeds budget: {:?}",
        plan.pixels
    );
    // When scratch shrinks with `pixels` (e.g. macOS thumbnail backing), the
    // planner must still halve to find a candidate that fits inside 32 MiB.
    let big_header = super::png_decode::Header::platform_format(
        exact_raster::Metadata {
            natural: exact_raster::PixelSize {
                width: 2300,
                height: 2300,
            },
            encoded_bytes: 100 * 1024,
            header_bytes: 64,
        },
        super::png_decode::PlatformFormat::Jpeg,
    );
    let fitted = super::plan(big_header, (2300., 2300.), exact_raster::SESSION_BYTES).unwrap();
    assert!(
        fitted.peak_bytes() <= exact_raster::SESSION_BYTES,
        "planner must halve when a smaller decode fits: peak={} > {}",
        fitted.peak_bytes(),
        exact_raster::SESSION_BYTES
    );
}

#[cfg(windows)]
#[test]
fn windows_scaling_ignores_hidden_colour_and_keeps_valid_premultiplied_pixels() {
    let (w, h) = (16, 8);
    let raw: Vec<u8> = (0..h)
        .flat_map(|_| {
            (0..w).flat_map(|x| {
                if x < w / 2 {
                    [255, 0, 0, 255]
                } else {
                    [0, 255, 0, 0]
                }
            })
        })
        .collect();
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, w, h);
    encoder.set_color(png::ColorType::Rgba);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&raw)
        .unwrap();
    let header = inspect(&mut Cursor::new(&bytes), bytes.len() as u64).unwrap();
    // Exercise WIC's converter/scaler chain independently of the PNG row decoder.
    let plan = DecodePlan::new(header, (3, 2)).unwrap();
    let pixels = super::jpeg_decode::decode(Cursor::new(&bytes), &plan, || false).unwrap();
    assert!(pixels
        .data()
        .as_chunks::<4>()
        .0
        .iter()
        .any(|p| p[3] > 0 && p[3] < 255));
    for p in pixels.data().as_chunks::<4>().0 {
        assert_eq!(*p, [p[3], 0, 0, p[3]]);
    }
}

#[cfg(windows)]
#[test]
fn windows_half_scale_averages_opaque_red_and_hidden_green() {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, 2, 1);
    encoder.set_color(png::ColorType::Rgba);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&[255, 0, 0, 255, 0, 255, 0, 0])
        .unwrap();
    let header = inspect(&mut Cursor::new(&bytes), bytes.len() as u64).unwrap();
    let plan = DecodePlan::new(header, (1, 1)).unwrap();
    let pixels = super::jpeg_decode::decode(Cursor::new(&bytes), &plan, || false).unwrap();
    for (actual, expected) in pixels.data().iter().zip([128, 0, 0, 128]) {
        assert!(actual.abs_diff(expected) <= 1, "{:?}", pixels.data());
    }
}

#[cfg(windows)]
#[test]
fn windows_reduces_a_jpeg_in_its_decoder_and_a_worker_opens_one_factory() {
    // Red on the left half and blue on the right: a wrong channel order, row
    // stride or axis shows as the wrong colour.
    let (red, blue) = ([200, 40, 20, 255], [20, 60, 220, 255]);
    let jpeg = |width: u32, height: u32| {
        let mut picture = tiny_skia::Pixmap::new(width, height).unwrap();
        for (i, p) in picture.pixels_mut().iter_mut().enumerate() {
            let [r, g, b, a] = if i as u32 % width < width / 2 {
                red
            } else {
                blue
            };
            *p = tiny_skia::PremultipliedColorU8::from_rgba(r, g, b, a).unwrap();
        }
        let encoding = exact_runner::canvas_capture::Encoding::new("image/jpeg", Some(0.9));
        crate::canvas2d::capture::encode(&picture, &encoding)
            .unwrap()
            .0
    };
    let upright = jpeg(1600, 1200);
    // The same file with EXIF orientation 6: shown turned, red on top.
    let mut turned =
        b"\xff\xd8\xff\xe1\0\x22Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0"
            .to_vec();
    turned.extend_from_slice(&upright[2..]);
    let _worker = super::jpeg_decode::Worker::enter();
    // The box drawn in, the plan's size for it, and the size asked of the
    // decoder in the file's own grid: the smallest of its reductions that is
    // at least twice the plan, or none when a half is not.
    for (bytes, drawn, planned, asked) in [
        (&upright, (40., 40.), (128, 96), Some((400, 300))),
        (&upright, (200., 150.), (256, 192), Some((800, 600))),
        (&upright, (400., 300.), (512, 384), None),
        (&turned, (40., 40.), (96, 128), Some((400, 300))),
    ] {
        let header = inspect(&mut Cursor::new(bytes), bytes.len() as u64).unwrap();
        let plan = super::plan(header, drawn, super::SESSION_BYTES).unwrap();
        assert_eq!((plan.pixels.width, plan.pixels.height), planned);
        let pixels = decode_rows(Cursor::new(bytes), &plan, || false).unwrap();
        assert_eq!(super::jpeg_decode::observed().1, asked, "{planned:?}");
        let (w, h) = (pixels.width(), pixels.height());
        assert_eq!((w, h), planned);
        let (in_red, in_blue) = if w > h {
            ((w / 8, h / 2), (w - w / 8, h / 2))
        } else {
            ((w / 2, h / 8), (w / 2, h - h / 8))
        };
        for ((x, y), colour) in [(in_red, red), (in_blue, blue)] {
            let p = pixels.pixel(x, y).unwrap();
            let near = [p.red(), p.green(), p.blue(), p.alpha()]
                .iter()
                .zip(colour)
                .all(|(got, want)| got.abs_diff(want) <= 6);
            assert!(near, "{planned:?} at {x},{y}: {p:?}");
        }
    }
    // A reduction that does not divide the frame is passed over: 780 rows
    // are 97.5 at an eighth, which would cover this plan twice.
    let uneven = jpeg(1040, 780);
    let header = inspect(&mut Cursor::new(&uneven), uneven.len() as u64).unwrap();
    let plan = DecodePlan::new(header, (60, 45)).unwrap();
    decode_rows(Cursor::new(&uneven), &plan, || false).unwrap();
    let (opened, asked) = super::jpeg_decode::observed();
    assert_eq!(asked, Some((260, 195)));
    assert_eq!(opened, 1);
}

fn pixel(x: u32, y: u32) -> [u8; 4] {
    [(x % 251) as u8, (y % 241) as u8, ((x + y) % 239) as u8, 128]
}

pub(super) fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, width, height);
    encoder.set_color(png::ColorType::Rgba);
    let mut writer = encoder.write_header().unwrap();
    let mut stream = writer.stream_writer().unwrap();
    use std::io::Write;
    for y in 0..height {
        let row: Vec<_> = (0..width).flat_map(|x| pixel(x, y)).collect();
        stream.write_all(&row).unwrap();
    }
    stream.finish().unwrap();
    writer.finish().unwrap();
    bytes
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc = !0u32;
    for b in kind.iter().chain(data) {
        crc ^= u32::from(*b);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb88320 & (0u32.wrapping_sub(crc & 1)));
        }
    }
    out.extend_from_slice(&(!crc).to_be_bytes());
}

fn header(width: u32, height: u32, interlaced: bool) -> Vec<u8> {
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut data = Vec::from(width.to_be_bytes());
    data.extend_from_slice(&height.to_be_bytes());
    data.extend_from_slice(&[8, 6, 0, 0, u8::from(interlaced)]);
    chunk(&mut out, b"IHDR", &data);
    out
}

// A small genuine Adam7 PNG, using a stored deflate block; no image-library
// full-frame deinterlacing is involved in constructing the expected samples.
pub(super) fn adam7(width: u32, height: u32) -> Vec<u8> {
    adam7_pixels(width, height, pixel)
}

fn adam7_pixels(width: u32, height: u32, pixel: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
    let mut raw = Vec::new();
    for (x, y, dx, dy) in [
        (0, 0, 8, 8),
        (4, 0, 8, 8),
        (0, 4, 4, 8),
        (2, 0, 4, 4),
        (0, 2, 2, 4),
        (1, 0, 2, 2),
        (0, 1, 1, 2),
    ] {
        if x >= width {
            continue;
        }
        for y in (y..height).step_by(dy) {
            raw.push(0);
            for x in (x..width).step_by(dx) {
                raw.extend_from_slice(&pixel(x, y));
            }
        }
    }
    let mut zlib = vec![0x78, 0x01];
    let mut blocks = raw.chunks(u16::MAX as usize).peekable();
    while let Some(block) = blocks.next() {
        let n = block.len() as u16;
        zlib.push(u8::from(blocks.peek().is_none()));
        zlib.extend_from_slice(&n.to_le_bytes());
        zlib.extend_from_slice(&(!n).to_le_bytes());
        zlib.extend_from_slice(block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for byte in raw {
        a = (a + u32::from(byte)) % 65521;
        b = (b + a) % 65521;
    }
    zlib.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut out = header(width, height, true);
    chunk(&mut out, b"IDAT", &zlib);
    chunk(&mut out, b"IEND", &[]);
    out
}

fn premultiplied([r, g, b, a]: [u8; 4]) -> [u8; 4] {
    let channel = |c| (u16::from(c) * u16::from(a) / 255) as u8;
    [channel(r), channel(g), channel(b), a]
}

#[test]
fn large_source_uses_bounded_rows_and_preserves_natural_dimensions() {
    let bytes = png(4000, 2000);
    let metadata = inspect(&mut Cursor::new(&bytes), bytes.len() as u64).unwrap();
    let plan = DecodePlan::new(metadata, (100, 50)).unwrap();
    assert_eq!(plan.natural(), (4000, 2000));
    assert_eq!(plan.output_bytes(), 20_000);
    assert!(
        plan.peak_bytes() < 2 * 1024 * 1024,
        "no source-sized frame reservation"
    );
    let bitmap = decode_rows(Cursor::new(&bytes), &plan, || false).unwrap();
    assert_eq!((bitmap.width(), bitmap.height()), (100, 50));
    let oracle = area_reference(4000, 2000, 100, 50, |x, y| premultiplied(pixel(x, y)));
    assert_eq!(bitmap.data(), oracle);
}

#[test]
fn adam7_rows_average_every_pass_without_full_frame_fallback() {
    for (w, h, dw, dh) in [
        (19, 11, 7, 5),
        (8, 8, 8, 8),
        (1, 9, 1, 4),
        (9, 1, 4, 1),
        (1, 1, 1, 1),
    ] {
        let bytes = adam7(w, h);
        let metadata = inspect(&mut Cursor::new(&bytes), bytes.len() as u64).unwrap();
        let plan = DecodePlan::new(metadata, (dw, dh)).unwrap();
        let bitmap = decode_rows(Cursor::new(&bytes), &plan, || false).unwrap();
        assert_eq!(
            bitmap.data(),
            area_reference(w, h, dw, dh, |x, y| premultiplied(pixel(x, y)))
        );
        for (x, y, cw, ch) in [(0, 0, dw, 1), (dw - 1, 0, 1, dh), (dw - 1, dh - 1, 1, 1)] {
            let part = plan.part(x, y, (cw, ch)).unwrap();
            let cut = decode_rows(Cursor::new(&bytes), &part, || false).unwrap();
            for iy in 0..ch {
                for ix in 0..cw {
                    assert_eq!(cut.pixel(ix, iy), bitmap.pixel(x + ix, y + iy));
                }
            }
        }
    }
}

#[test]
fn limits_and_bad_geometry_refuse_before_decoder_allocation() {
    for (w, h) in [(0, 5), (u32::MAX, u32::MAX), (8193, 8192)] {
        let bytes = header(w, h, false);
        assert!(inspect(&mut Cursor::new(&bytes), bytes.len() as u64).is_err());
    }
    let bytes = png(8, 8);
    assert!(inspect(&mut Cursor::new(&bytes), 64 * 1024 * 1024 + 1).is_err());
    let mut over_header = header(8, 8, false);
    chunk(&mut over_header, b"tEXt", &vec![b'a'; 256 * 1024]);
    assert!(inspect(&mut Cursor::new(&over_header), over_header.len() as u64).is_err());
    let metadata = inspect(&mut Cursor::new(&bytes), bytes.len() as u64).unwrap();
    assert!(DecodePlan::new(metadata, (u32::MAX, 2)).is_err());
    let plan = DecodePlan::new(metadata, (4, 4)).unwrap();
    assert!(decode_rows(Cursor::new(&bytes), &plan, || true).is_err());
}

#[test]
fn cancellation_is_observed_between_rows_and_corruption_never_publishes() {
    let bytes = png(80, 40);
    let metadata = inspect(&mut Cursor::new(&bytes), bytes.len() as u64).unwrap();
    let plan = DecodePlan::new(metadata, (10, 5)).unwrap();
    let calls = std::cell::Cell::new(0);
    assert!(decode_rows(Cursor::new(&bytes), &plan, || {
        calls.set(calls.get() + 1);
        calls.get() > 4
    })
    .is_err());
    let mut corrupt = bytes;
    corrupt.truncate(corrupt.len() / 2);
    assert!(decode_rows(Cursor::new(&corrupt), &plan, || false).is_err());
}

#[test]
fn normalized_color_formats_match_the_full_frame_oracle() {
    for (color, depth) in [
        (png::ColorType::Rgba, png::BitDepth::Eight),
        (png::ColorType::Rgb, png::BitDepth::Eight),
        (png::ColorType::GrayscaleAlpha, png::BitDepth::Eight),
        (png::ColorType::Grayscale, png::BitDepth::Eight),
        (png::ColorType::Grayscale, png::BitDepth::One),
        (png::ColorType::Indexed, png::BitDepth::One),
        (png::ColorType::Rgba, png::BitDepth::Sixteen),
    ] {
        let (w, h) = (17u32, 13u32);
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, w, h);
        encoder.set_color(color);
        encoder.set_depth(depth);
        if color == png::ColorType::Indexed {
            encoder.set_palette(vec![12, 180, 70, 211, 30, 230]);
            encoder.set_trns(vec![128, 255]);
        }
        let channels = color.samples();
        let row = (w as usize * channels * depth as usize).div_ceil(8);
        let raw: Vec<u8> = (0..row * h as usize).map(|n| (n % 251) as u8).collect();
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&raw)
            .unwrap();
        let metadata = inspect(&mut Cursor::new(&bytes), bytes.len() as u64).unwrap();
        let (full, _, _) = decode_reference(&bytes).unwrap();
        for (dw, dh) in [(7, 5), (w, 5), (7, h), (w, h)] {
            let plan = DecodePlan::new(metadata, (dw, dh)).unwrap();
            let actual = decode_rows(Cursor::new(&bytes), &plan, || false).unwrap();
            let oracle = area_reference(w, h, dw, dh, |x, y| {
                let at = ((y * w + x) * 4) as usize;
                full[at..at + 4].try_into().unwrap()
            });
            assert_eq!(actual.data(), oracle, "{color:?} {depth:?} {dw}x{dh}");
        }
    }
}

fn area_reference(
    width: u32,
    height: u32,
    out_w: u32,
    out_h: u32,
    pixel: impl Fn(u32, u32) -> [u8; 4],
) -> Vec<u8> {
    let (w, h, dw, dh) = (
        u64::from(width),
        u64::from(height),
        u64::from(out_w),
        u64::from(out_h),
    );
    let mut out = Vec::new();
    for y in 0..dh {
        let (top, bottom) = (y * h, (y + 1) * h);
        for x in 0..dw {
            let (left, right) = (x * w, (x + 1) * w);
            let mut sum = [0; 4];
            for iy in top / dh..bottom.div_ceil(dh) {
                for ix in left / dw..right.div_ceil(dw) {
                    let weight = (right.min((ix + 1) * dw) - left.max(ix * dw))
                        * (bottom.min((iy + 1) * dh) - top.max(iy * dh));
                    let p = pixel(ix as u32, iy as u32);
                    for c in 0..4 {
                        sum[c] += u64::from(p[c]) * weight;
                    }
                }
            }
            out.extend(sum.map(|v| ((v + w * h / 2) / (w * h)) as u8));
        }
    }
    out
}

#[test]
fn shrinking_averages_premultiplied_area_and_crops_the_same_pixels() {
    let (w, h) = (30, 30);
    let pixel = |x: u32, y: u32| match (x + y * 3) % 5 {
        0 => [255, 20, 10, 255],
        1 => [10, 255, 250, 0],
        2 => [230, 40, 170, 83],
        3 => [20, 100, 255, 171],
        _ => [0, 0, 0, 255],
    };
    let raw: Vec<u8> = (0..h)
        .flat_map(|y| (0..w).flat_map(move |x| pixel(x, y)))
        .collect();
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, w, h);
    encoder.set_color(png::ColorType::Rgba);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&raw)
        .unwrap();
    for (interlaced, bytes) in [(false, bytes), (true, adam7_pixels(w, h, pixel))] {
        let header = inspect(&mut Cursor::new(&bytes), bytes.len() as u64).unwrap();
        let natural = DecodePlan::new(header, (w, h)).unwrap();
        for (dw, dh) in [
            (15, 15),
            (12, 12),
            (10, 10),
            (30, 12),
            (12, 30),
            (30, 30),
            (7, 11),
            (29, 29),
            (1, 1),
        ] {
            let plan = DecodePlan::new(header, (dw, dh)).unwrap();
            let actual = decode_rows(Cursor::new(&bytes), &plan, || false).unwrap();
            let oracle = area_reference(w, h, dw, dh, |x, y| premultiplied(pixel(x, y)));
            assert_eq!(actual.data(), oracle, "{dw}x{dh} interlaced={interlaced}");
            assert!(actual
                .data()
                .as_chunks::<4>()
                .0
                .iter()
                .all(|p| p[..3].iter().all(|c| *c <= p[3])));
            if (dw, dh) == (w, h) {
                let exact: Vec<u8> = raw
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .flat_map(|p| {
                        [
                            (u16::from(p[0]) * u16::from(p[3]) / 255) as u8,
                            (u16::from(p[1]) * u16::from(p[3]) / 255) as u8,
                            (u16::from(p[2]) * u16::from(p[3]) / 255) as u8,
                            p[3],
                        ]
                    })
                    .collect();
                assert_eq!(actual.data(), exact);
            } else {
                assert_eq!(
                    plan.cost.scratch_bytes - natural.cost.scratch_bytes,
                    u64::from(dw) * if interlaced { u64::from(dh) * 32 } else { 64 }
                );
            }
            for (x, y, cw, ch) in [
                (dw / 3, dh / 3, dw - dw / 3, dh - dh / 3),
                (dw - 1, dh - 1, 1, 1),
            ] {
                let part = plan.part(x, y, (cw, ch)).unwrap();
                let cut = decode_rows(Cursor::new(&bytes), &part, || false).unwrap();
                for iy in 0..ch {
                    for ix in 0..cw {
                        assert_eq!(cut.pixel(ix, iy), actual.pixel(x + ix, y + iy));
                    }
                }
            }
        }
    }
}

#[test]
fn fractional_area_decodes_and_crops_have_known_bytes() {
    let raw = [
        240, 0, 0, 255, 0, 255, 0, 0, 0, 0, 240, 255, 120, 60, 30, 255, 240, 120, 60, 85, 60, 120,
        240, 170,
    ];
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, 3, 2);
    encoder.set_color(png::ColorType::Rgba);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&raw)
        .unwrap();
    let interlaced = adam7_pixels(3, 2, |x, y| {
        let at = (y * 3 + x) as usize * 4;
        raw[at..at + 4].try_into().unwrap()
    });
    for bytes in [bytes, interlaced] {
        let header = inspect(&mut Cursor::new(&bytes), bytes.len() as u64).unwrap();
        let plan = DecodePlan::new(header, (2, 1)).unwrap();
        let pixels = decode_rows(Cursor::new(&bytes), &plan, || false).unwrap();
        assert_eq!(pixels.data(), [133, 27, 13, 184, 27, 33, 137, 156]);
        let part = plan.part(1, 0, (1, 1)).unwrap();
        let pixels = decode_rows(Cursor::new(&bytes), &part, || false).unwrap();
        assert_eq!(pixels.data(), [27, 33, 137, 156]);
        let plan = DecodePlan::new(header, (1, 1)).unwrap();
        let pixels = decode_rows(Cursor::new(&bytes), &plan, || false).unwrap();
        assert_eq!(pixels.data(), [80, 30, 75, 170]);
    }
}

#[test]
fn scaled_adam7_storage_is_bounded_without_reducing_decode_size() {
    let bytes = adam7(514, 513);
    let metadata = inspect(&mut Cursor::new(&bytes), bytes.len() as u64).unwrap();
    let natural = DecodePlan::new(metadata, (514, 513)).unwrap();
    for (width, sums) in [(512, exact_raster::SESSION_BYTES / 4), (513, 0)] {
        let plan = DecodePlan::new(metadata, (width, 512)).unwrap();
        assert_eq!(plan.cost.scratch_bytes - natural.cost.scratch_bytes, sums);
        let pixels = decode_rows(Cursor::new(&bytes), &plan, || false).unwrap();
        if width == 512 {
            assert_eq!(
                pixels.data(),
                area_reference(514, 513, 512, 512, |x, y| { premultiplied(pixel(x, y)) })
            );
        } else {
            for (x, y, expected) in [
                (0, 0, [0, 0, 0, 128]),
                (256, 256, [3, 8, 18, 128]),
                (512, 511, [5, 15, 34, 128]),
            ] {
                let at = (y * width + x) as usize * 4;
                assert_eq!(&pixels.data()[at..at + 4], expected);
            }
        }
        for part in [plan, plan.part(256, 256, (1, 1)).unwrap()] {
            let key = exact_raster::RasterKey {
                source: 1,
                generation: 1,
                pixels: part.pixels,
                variant: 1,
                crop: part.crop,
            };
            let again = DecodePlan::of_key(metadata, &key).unwrap();
            assert_eq!(
                (again.pixels, again.crop, again.cost),
                (part.pixels, part.crop, part.cost)
            );
            let decoded = decode_rows(Cursor::new(&bytes), &again, || false).unwrap();
            if part.crop.whole() {
                assert_eq!(decoded.data(), pixels.data());
            } else {
                assert_eq!(decoded.pixel(0, 0), pixels.pixel(256, 256));
            }
        }
    }
    let mut bytes = header(4096, 4096, true);
    chunk(&mut bytes, b"IDAT", &[]);
    let header = inspect(&mut Cursor::new(&bytes), bytes.len() as u64).unwrap();
    for (desired, available, side) in [
        (2048., exact_raster::SESSION_BYTES, 2048),
        (2048., 16 * 1024 * 1024, 1024),
        (1000., exact_raster::SESSION_BYTES, 1024),
    ] {
        let planned = super::plan(header, (desired, desired), available).unwrap();
        assert_eq!((planned.pixels.width, planned.pixels.height), (side, side));
        assert!(planned.peak_bytes() <= available);
    }
}

fn decode_reference(bytes: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width, info.height);
    if w == 0 || h == 0 {
        return None;
    }
    let src = &buf[..info.buffer_size()];
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    let push = |out: &mut Vec<u8>, r: u8, g: u8, b: u8, a: u8| {
        let p = |c: u8| (c as u32 * a as u32 / 255) as u8;
        out.extend_from_slice(&[p(r), p(g), p(b), a]);
    };
    match info.color_type {
        png::ColorType::Rgba => {
            for px in src.as_chunks::<4>().0 {
                push(&mut out, px[0], px[1], px[2], px[3]);
            }
        }
        png::ColorType::Rgb => {
            for px in src.as_chunks::<3>().0 {
                push(&mut out, px[0], px[1], px[2], 255);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for px in src.as_chunks::<2>().0 {
                push(&mut out, px[0], px[0], px[0], px[1]);
            }
        }
        png::ColorType::Grayscale => {
            for px in src {
                push(&mut out, *px, *px, *px, 255);
            }
        }
        png::ColorType::Indexed => return None,
    }
    Some((out, w, h))
}

/// A part of a picture is the whole decode's pixels of that part, bit for
/// bit, charged as the part; a key names it, and a part outside the decode
/// (or of a size the planner would not decode at) is refused.
#[test]
fn a_part_is_the_whole_decodes_pixels() {
    let bytes = png(200, 120);
    let header = inspect(&mut Cursor::new(&bytes), bytes.len() as u64).unwrap();
    let whole = DecodePlan::new(header, (100, 60)).unwrap();
    let all = decode_rows(Cursor::new(&bytes), &whole, || false).unwrap();
    for (x, y, w, h) in [
        (17u32, 5u32, 40u32, 30u32),
        (0, 0, 100, 13),
        (63, 59, 37, 1),
        (99, 0, 1, 60),
    ] {
        let part = whole.part(x, y, (w, h)).unwrap();
        assert_eq!((part.pixels.width, part.pixels.height), (w, h));
        assert_eq!(part.output_bytes(), u64::from(w) * u64::from(h) * 4);
        assert_eq!(part.cost.scratch_bytes, whole.cost.scratch_bytes);
        assert_eq!((part.full(), part.natural()), (whole.pixels, (200, 120)));
        let cut = decode_rows(Cursor::new(&bytes), &part, || false).unwrap();
        for row in 0..h {
            let from = (((y + row) * 100 + x) * 4) as usize;
            assert_eq!(
                &cut.data()[(row * w * 4) as usize..((row + 1) * w * 4) as usize],
                &all.data()[from..from + (w * 4) as usize],
                "row {row} of {w}x{h} at {x},{y}"
            );
        }
        // The key that names the part gives the same plan back.
        let key = exact_raster::RasterKey {
            source: 1,
            generation: 1,
            pixels: part.pixels,
            variant: 1,
            crop: part.crop,
        };
        let again = DecodePlan::of_key(header, &key).unwrap();
        assert_eq!(
            (again.pixels, again.crop, again.cost),
            (part.pixels, part.crop, part.cost)
        );
    }
    // The whole is itself, not a part.
    assert!(whole.part(0, 0, (100, 60)).unwrap().crop.whole());
    for (x, y, w, h) in [
        (0u32, 0u32, 101u32, 60u32),
        (61, 0, 40, 60),
        (0, 31, 10, 30),
        (0, 0, 0, 5),
        (u32::MAX, 0, 2, 2),
    ] {
        assert!(whole.part(x, y, (w, h)).is_err(), "{w}x{h} at {x},{y}");
    }
    assert!(
        whole
            .part(1, 1, (5, 5))
            .unwrap()
            .part(0, 0, (2, 2))
            .is_err(),
        "a part of a part"
    );
}
