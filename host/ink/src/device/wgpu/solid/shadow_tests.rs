use super::super::{programs, Device, Format, TextureDesc, Wgpu};
use super::shadow_interior;
use crate::instance::{
    Bindings, Brush, Clip, CoverageClip, Encoder, Instance, Kind, RoundedClip, Scissor,
};

const SIZE: [u32; 2] = [232, 176];

fn card([x, y]: [f32; 2], scale: f32, radii: [f32; 4]) -> Instance {
    // The 166 x 100 border box stays put while the caster spreads 8 and moves (-1, 20).
    let mut i = Instance::new(Kind::Shadow, Brush::Solid, [-22., -1., 208., 142.]);
    i.uv = [-9., 12., 182., 116.];
    i.radii = radii.map(|r| r + 8.);
    i.param[0] = 4.;
    i.color = [0.4, 0.1, 0.2, 0.65];
    i.xform = [scale, 0., 0., scale, x, y];
    i.set_clip(
        1,
        RoundedClip {
            rect: [x, y, 166. * scale, 100. * scale],
            radii: radii.map(|r| r * scale),
        },
    )
    .unwrap();
    i
}

#[test]
fn shadow_bands_match_unsplit_draw_while_a_card_moves() {
    let mut d = Wgpu::new().unwrap();
    let [w, h] = SIZE;
    let target = d.create_target(w, h).unwrap();
    let mask = d
        .create_texture(TextureDesc {
            width: 2,
            height: 2,
            format: Format::Alpha,
            mipmapped: false,
        })
        .unwrap();
    d.upload_texture(mask, [0, 0], [2, 2], &[200, 40, 120, 255])
        .unwrap();
    let rounded = [3., 13.25, 8., 5.];
    for (origin, scale, radii, skew) in [
        ([24., 20.], 1., [0.; 4], 0.),
        ([24., 20.], 1., rounded, 0.),
        ([24.125, 20.375], 1., rounded, 0.),
        ([24.5, 20.75], 1., rounded, 0.),
        ([-3.25, -9.5], 1.25, rounded, 0.),
        ([24.5, 20.75], 1., rounded, 0.23),
    ] {
        let mut encoder = Encoder::new(w, h);
        encoder
            .push_clip(Clip::Scissor(Scissor::from_rect([3., 7., 221., 156.])))
            .unwrap();
        let mut shadow = card(origin, scale, radii);
        shadow.xform[1] = skew;
        assert_eq!(
            shadow_interior(&shadow, encoder.scissor()).is_some(),
            skew == 0.,
            "{origin:?} scale {scale} skew {skew}"
        );
        let mut small = card([62., 116.], 0.25, rounded);
        small.color = [0.05, 0.4, 0.1, 0.5];
        for i in [small, shadow, small.translated([73., 4.])] {
            encoder.push(i, Bindings::default()).unwrap();
        }
        encoder
            .push_clip(Clip::Coverage(CoverageClip {
                texture: mask,
                rect: [10., 11., 145., 127.],
                uv: [0., 0., 1., 1.],
            }))
            .unwrap();
        encoder
            .push_clip(Clip::Rounded(RoundedClip {
                rect: [20.25, 14.5, 145., 127.],
                radii: [18.; 4],
            }))
            .unwrap();
        encoder
            .push(shadow.translated([-57., -18.]), Bindings::default())
            .unwrap();
        let frame = encoder.finish();
        let clear = Some([0.04, 0.16, 0.09, 0.3]);
        d.render(&frame, target, clear).unwrap();
        let actual = d.readback(target).unwrap();

        // A narrow scissor keeps the same fixed shader and instance order
        // but cannot fit a shadow interior, so every covered pixel is shaded.
        let mut reference = frame;
        for batch in std::mem::take(&mut reference.batches) {
            let end = batch.scissor.x + batch.scissor.width;
            for x in (batch.scissor.x..end).step_by(16) {
                let mut strip = batch.clone();
                strip.scissor.x = x;
                strip.scissor.width = (end - x).min(16);
                reference.batches.push(strip);
            }
        }
        d.render(&reference, target, clear).unwrap();
        let expected = d.readback(target).unwrap();
        assert!(expected.as_chunks::<4>().0.iter().any(|rgba| rgba[3] > 120));
        for (n, (a, b)) in actual
            .as_chunks::<4>()
            .0
            .iter()
            .zip(expected.as_chunks::<4>().0)
            .enumerate()
        {
            assert_eq!(
                a,
                b,
                "{origin:?} scale {scale} skew {skew} at ({}, {})",
                n % w as usize,
                n / w as usize
            );
        }
    }
}

#[test]
fn shadow_split_omits_exact_interior_without_clipping_outer_quad() {
    let mut d = Wgpu::new().unwrap();
    let [w, h] = SIZE;
    let target = d.create_target(w, h).unwrap();
    let shadow = card([24.25, 20.5], 1., [7.; 4]);
    let full_scissor = Scissor {
        x: 0,
        y: 0,
        width: w,
        height: h,
    };
    let (_, inner) = shadow_interior(&shadow, full_scissor).expect("shadow splits");
    assert!(!programs::fast_shadow(&shadow));
    // A flat fragment makes every rasterized pixel visible across the full quad.
    d.programs[programs::class(&shadow).unwrap()] = d.solid_pipeline.clone();
    let mut encoder = Encoder::new(w, h);
    encoder.push(shadow, Bindings::default()).unwrap();
    let frame = encoder.finish();
    d.render(&frame, target, Some([0.; 4])).unwrap();
    let split = d.readback(target).unwrap();

    // Render the unsplit quad through narrow strips with the same flat shader.
    let mut unsplit = frame;
    for batch in std::mem::take(&mut unsplit.batches) {
        let end = batch.scissor.x + batch.scissor.width;
        for x in (batch.scissor.x..end).step_by(16) {
            let mut strip = batch.clone();
            strip.scissor.x = x;
            strip.scissor.width = (end - x).min(16);
            unsplit.batches.push(strip);
        }
    }
    d.render(&unsplit, target, Some([0.; 4])).unwrap();
    let full = d.readback(target).unwrap();

    let mut omitted = 0usize;
    let mut shaded = 0usize;
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            let got = &split[i * 4..][..4];
            let exp = &full[i * 4..][..4];
            let in_inner = x >= inner.x
                && x < inner.x + inner.width
                && y >= inner.y
                && y < inner.y + inner.height;
            if in_inner {
                assert_eq!(got, [0; 4], "interior pixel ({x}, {y}) was rasterized");
                omitted += 1;
            } else {
                assert_eq!(got, exp, "outer pixel ({x}, {y}) clipped or altered");
                if got[3] > 0 {
                    shaded += 1;
                }
            }
        }
    }
    assert!(omitted >= 32 * 32);
    assert!(shaded > 0);
}

fn scaled_card(
    rect: [f32; 4],
    radii: [f32; 4],
    clip_radii: [f32; 4],
    offset: [f32; 2],
    sigma: f32,
    scale: f32,
    translate: [f32; 2],
) -> Instance {
    let [x, y, w, h] = rect;
    let reach = 3. * sigma + 1.;
    let sx = x + offset[0];
    let sy = y + offset[1];
    let mut i = Instance::new(
        Kind::Shadow,
        Brush::Solid,
        [sx - reach, sy - reach, w + 2. * reach, h + 2. * reach],
    );
    i.xform = [scale, 0., 0., scale, translate[0], translate[1]];
    i.uv = [sx, sy, w, h];
    i.radii = radii;
    i.param[0] = sigma;
    i.color = [1.; 4];
    i.set_clip(
        1,
        RoundedClip {
            rect: [
                x * scale + translate[0],
                y * scale + translate[1],
                w * scale,
                h * scale,
            ],
            radii: clip_radii.map(|r| r * scale),
        },
    )
    .unwrap();
    i
}

fn rounded_distance(p: [f32; 2], rect: [f32; 4], r: [f32; 4]) -> f32 {
    let lo = [rect[0], rect[1]];
    let hi = [lo[0] + rect[2], lo[1] + rect[3]];
    let corner =
        |c: [f32; 2], rad: f32| ((p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2)).sqrt() - rad;
    if p[0] < lo[0] + r[0] && p[1] < lo[1] + r[0] {
        return corner([lo[0] + r[0], lo[1] + r[0]], r[0]);
    }
    if p[0] > hi[0] - r[1] && p[1] < lo[1] + r[1] {
        return corner([hi[0] - r[1], lo[1] + r[1]], r[1]);
    }
    if p[0] > hi[0] - r[2] && p[1] > hi[1] - r[2] {
        return corner([hi[0] - r[2], hi[1] - r[2]], r[2]);
    }
    if p[0] < lo[0] + r[3] && p[1] > hi[1] - r[3] {
        return corner([lo[0] + r[3], hi[1] - r[3]], r[3]);
    }
    let q = [0, 1].map(|a| (p[a] - (lo[a] + hi[a]) * 0.5).abs() - rect[2 + a] * 0.5);
    (q[0].max(0.).powi(2) + q[1].max(0.).powi(2)).sqrt() + q[0].max(q[1]).min(0.)
}

#[test]
fn fast_shadow_matches_2d_gaussian_oracle_and_clip1_rim() {
    // Abramowitz & Stegun 7.1.26 erf, accurate to 1.5e-7, for f64 quadrature oracle.
    fn exact_cdf(x: f64) -> f64 {
        let a = x.abs() * std::f64::consts::FRAC_1_SQRT_2;
        let t = 1. / (1. + 0.3275911 * a);
        let poly = t
            * (0.254829592
                + t * (-0.284496736 + t * (1.421413741 + t * (-1.453152027 + t * 1.061405429))));
        let tail = 0.5 * poly * (-a * a).exp();
        if x >= 0. {
            1. - tail
        } else {
            tail
        }
    }

    fn gaussian_2d(p: [f64; 2], rect: [f64; 4], radii: [f64; 4], sigma: f64) -> f64 {
        if p[0] < rect[0] - 5.5 * sigma || p[0] > rect[0] + rect[2] + 5.5 * sigma {
            return 0.;
        }
        let low = rect[1].max(p[1] - 5.5 * sigma);
        let high = (rect[1] + rect[3]).min(p[1] + 5.5 * sigma);
        if high <= low {
            return 0.;
        }
        const N: usize = 200;
        let step = (high - low) / N as f64;
        let mut sum = 0.;
        for k in 0..N {
            let line = low + (k as f64 + 0.5) * step;
            let top = line - rect[1];
            let bottom = rect[1] + rect[3] - line;
            let lr = if top < radii[0] { radii[0] } else { radii[3] };
            let rr = if top < radii[1] { radii[1] } else { radii[2] };
            let dl = (lr - if top < radii[0] { top } else { bottom }).max(0.);
            let dr = (rr - if top < radii[1] { top } else { bottom }).max(0.);
            let left = rect[0] + lr - (lr * lr - dl * dl).max(0.).sqrt();
            let right = rect[0] + rect[2] - rr + (rr * rr - dr * dr).max(0.).sqrt();
            let ix = exact_cdf((right - p[0]) / sigma) - exact_cdf((left - p[0]) / sigma);
            let y = (p[1] - line) / sigma;
            sum += ix * (-0.5 * y * y).exp() * (0.3989422804014327 / sigma) * step;
        }
        sum.clamp(0., 1.)
    }

    let mut d = Wgpu::new().unwrap();
    let [w, h] = [240u32, 190u32];
    let target = d.create_target(w, h).unwrap();
    let cases = [
        // Fractional card (Payspot-like r/sigma = 1.78).
        (
            "fractional-card",
            scaled_card(
                [32.333, 28.667, 176., 116.],
                [16.; 4],
                [16.; 4],
                [0., 4.],
                9.,
                1.,
                [0.; 2],
            ),
        ),
        // Guard boundary: r_max = 2*sigma, min_dim = 2*r_max + 7.5*sigma, asymmetric radii.
        (
            "boundary-asym",
            scaled_card(
                [32., 28., 104., 104.],
                [18., 1.5, 12., 18.],
                [18., 1.5, 12., 18.],
                [0., 4.],
                9.,
                1.,
                [0.; 2],
            ),
        ),
        // HiDPI 2x scale with asymmetric radii and fractional translation.
        (
            "hidpi-2x",
            scaled_card(
                [16., 14., 88., 60.],
                [6., 1.5, 4., 6.],
                [6., 1.5, 4., 6.],
                [0., 2.],
                4.5,
                2.,
                [0.25, 0.5],
            ),
        ),
        // Half scale hitting sigma*sx == 1.0 exactly.
        (
            "scale-0.5x",
            scaled_card(
                [64., 56., 200., 120.],
                [4.; 4],
                [4.; 4],
                [0., 4.],
                2.,
                0.5,
                [0.; 2],
            ),
        ),
        // Large local coordinates exercising edge-relative subtraction without half-size cancellation.
        (
            "large-local",
            scaled_card(
                [16000.25, 16000.75, 176., 116.],
                [16.; 4],
                [16.; 4],
                [0., 4.],
                9.,
                1.,
                [-15968., -15972.],
            ),
        ),
        // Partially offscreen clip1 (negative origin) and visible quadrant-boundary clip1 radius (r = h/2).
        (
            "offscreen-quadrant-clip1",
            scaled_card(
                [-24.25, -18.5, 160., 110.],
                [16.; 4],
                [1.5, 1.5, 55., 1.5],
                [0., 4.],
                9.,
                1.,
                [0.; 2],
            ),
        ),
    ];

    for (name, shadow) in cases {
        assert!(programs::fast_shadow(&shadow), "{name} must be eligible");
        let mut encoder = Encoder::new(w, h);
        encoder.push(shadow, Bindings::default()).unwrap();
        let frame = encoder.finish();
        d.render(&frame, target, Some([0.; 4])).unwrap();
        let pixels = d.readback(target).unwrap();

        let [scale, _, _, _, tx, ty] = shadow.xform;
        let rect = shadow.uv.map(f64::from);
        let rad = shadow.radii.map(f64::from);
        let s = f64::from(shadow.param[0]);
        let clip_rad = programs::clip1_radii(&shadow);
        let mut worst = (0u8, 0u32, 0u32);
        let mut checked_corner_pocket = false;
        for y in 0..h {
            for x in 0..w {
                let p_dev = [x as f32 + 0.5, y as f32 + 0.5];
                let dist = rounded_distance(p_dev, shadow.clip1, clip_rad);
                // Exclude only pixels inside clip1 or within 1.5 px of its rounded boundary,
                // keeping the 4 exterior rounded-corner pockets under the 2D Gaussian oracle.
                if dist <= 1.5 {
                    continue;
                }
                if p_dev[0] >= shadow.clip1[0]
                    && p_dev[0] <= shadow.clip1[0] + shadow.clip1[2]
                    && p_dev[1] >= shadow.clip1[1]
                    && p_dev[1] <= shadow.clip1[1] + shadow.clip1[3]
                {
                    checked_corner_pocket = true;
                }
                let p_local = [
                    f64::from(p_dev[0] - tx) / f64::from(scale),
                    f64::from(p_dev[1] - ty) / f64::from(scale),
                ];
                let actual = pixels[((y * w + x) * 4 + 3) as usize];
                let expected = (gaussian_2d(p_local, rect, rad, s) * 255.).round() as u8;
                let off = actual.abs_diff(expected);
                if off > worst.0 {
                    worst = (off, x, y);
                }
            }
        }
        if clip_rad.into_iter().fold(0f32, f32::max) >= 8. {
            assert!(
                checked_corner_pocket,
                "{name}: expected at least one exterior rounded-corner pocket pixel"
            );
        }
        assert!(
            worst.0 <= 1,
            "{name}: max error {}/255 vs 2D Gaussian oracle at ({}, {})",
            worst.0,
            worst.1,
            worst.2
        );

        // Full frame (including the 1 px clip1 antialiasing rim) must stay within 2/255
        // of fs_main's 8-tap + shape_coverage path.
        d.general_only = true;
        d.render(&frame, target, Some([0.; 4])).unwrap();
        let fallback = d.readback(target).unwrap();
        d.general_only = false;
        let worst_rim = pixels
            .as_chunks::<4>()
            .0
            .iter()
            .zip(fallback.as_chunks::<4>().0)
            .map(|(a, b)| a[3].abs_diff(b[3]))
            .max()
            .unwrap_or(0);
        assert!(
            worst_rim <= 2,
            "{name}: full frame differs by {worst_rim}/255 from fs_main"
        );
    }
}

#[test]
fn fast_shadow_split_matches_unsplit_draw_and_omits_interior() {
    let mut d = Wgpu::new().unwrap();
    let [w, h] = [240u32, 190u32];
    let target = d.create_target(w, h).unwrap();
    let shadow = scaled_card(
        [32.333, 28.667, 176., 116.],
        [16.; 4],
        [16.; 4],
        [0., 4.],
        9.,
        1.,
        [0.; 2],
    );
    assert!(programs::fast_shadow(&shadow));
    let full_scissor = Scissor {
        x: 0,
        y: 0,
        width: w,
        height: h,
    };
    let (_, inner) = shadow_interior(&shadow, full_scissor).expect("fast shadow splits");

    let mut encoder = Encoder::new(w, h);
    encoder.push(shadow, Bindings::default()).unwrap();
    let frame = encoder.finish();
    d.render(&frame, target, Some([0.04, 0.16, 0.09, 0.3]))
        .unwrap();
    let split = d.readback(target).unwrap();

    let mut unsplit = frame.clone();
    for batch in std::mem::take(&mut unsplit.batches) {
        let end = batch.scissor.x + batch.scissor.width;
        for x in (batch.scissor.x..end).step_by(16) {
            let mut strip = batch.clone();
            strip.scissor.x = x;
            strip.scissor.width = (end - x).min(16);
            unsplit.batches.push(strip);
        }
    }
    d.render(&unsplit, target, Some([0.04, 0.16, 0.09, 0.3]))
        .unwrap();
    let expected = d.readback(target).unwrap();
    assert_eq!(split, expected, "split fast_shadow must match unsplit draw");

    // Verify exact rasterization mask with the flat shader.
    d.fast_shadow_pipeline = d.solid_pipeline.clone();
    d.render(&frame, target, Some([0.; 4])).unwrap();
    let mask = d.readback(target).unwrap();
    let px = |x: u32, y: u32| &mask[((y * w + x) * 4) as usize..][..4];
    assert_eq!(
        px(inner.x + inner.width / 2, inner.y + inner.height / 2),
        [0; 4]
    );
    assert_eq!(px(120, 155)[3], 255);
}
