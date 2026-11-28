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
