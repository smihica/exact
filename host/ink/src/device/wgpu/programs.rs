//! Fixed fragment programs, compiled before the first frame (expo/exact#145).
//! Selectors follow smihica's V3D investigation; clips remain instance data.
use super::*;

pub(super) struct Class {
    kind_brush: u32,
    flags: u32,
    required: u32,
}

const CLIPS: u32 = flags::CLIP0 | flags::CLIP1 | flags::MASK;
const IMAGE: u32 = CLIPS | flags::TINT | flags::NEAREST | flags::PIXELATED;

impl Class {
    const fn new(kind: Kind, brush: Brush, flags: u32) -> Self {
        Self {
            kind_brush: kind as u32 | ((brush as u32) << 16),
            flags,
            required: 0,
        }
    }

    const fn exact(kind: Kind, brush: Brush, flags: u32) -> Self {
        Self {
            required: flags,
            ..Self::new(kind, brush, flags)
        }
    }

    pub(super) fn constants(&self) -> [(&'static str, f64); 4] {
        [
            ("SPECIALIZED", 1.),
            ("KIND_BRUSH", self.kind_brush as f64),
            ("FLAGS_MASK", self.flags as f64),
            ("FLAGS_SET", self.required as f64),
        ]
    }
}

// Unlisted kinds, brushes and flags use the general program.
pub(super) const CLASSES: [Class; 14] = [
    Class::new(Kind::RoundedRect, Brush::Solid, 0),
    Class::new(Kind::RoundedRect, Brush::Image, flags::CLIP0),
    Class::new(Kind::Glyph, Brush::Solid, 0),
    Class::new(Kind::Layer, Brush::Image, 0),
    Class::new(Kind::RoundedRect, Brush::Solid, CLIPS),
    Class::new(Kind::Ring, Brush::Solid, CLIPS),
    Class::new(Kind::Shadow, Brush::Solid, flags::CLIP1),
    Class::new(Kind::Shadow, Brush::Solid, CLIPS),
    Class::new(Kind::Glyph, Brush::Solid, CLIPS | flags::COLOR_GLYPH),
    Class::new(Kind::Coverage, Brush::Solid, CLIPS),
    Class::new(Kind::Layer, Brush::Image, CLIPS),
    Class::new(Kind::RoundedRect, Brush::Image, IMAGE),
    Class::new(Kind::Coverage, Brush::Image, IMAGE),
    Class::exact(
        Kind::Shadow,
        Brush::Image,
        flags::CLIP1 | flags::TINT | flags::RASTERED_SHADOW,
    ),
];

pub(super) fn class(instance: &Instance) -> Option<usize> {
    CLASSES.iter().position(|c| {
        instance.kind_brush == c.kind_brush
            && instance.flags & !c.flags == 0
            && instance.flags & c.required == c.required
    })
}

pub(super) fn clip1_radii(i: &Instance) -> [f32; 4] {
    [
        i.clip_radii[2] & 65535,
        i.clip_radii[2] >> 16,
        i.clip_radii[3] & 65535,
        i.clip_radii[3] >> 16,
    ]
    .map(|r| r as f32 / 16.)
}

/// Eligible `CLIP1` outer shadows evaluated by `fs_fast_shadow` (closed-form
/// straight bands + 3-point Gauss-Legendre corner arcs + analytical `clip1` SDF).
///
/// The geometric and numerical bounds keep worst-case error within 0.61/255
/// against the exact 2D Gaussian convolution (even at full opacity and across
/// asymmetric corner radii) and keep `rounded_box_coverage` within 0.66/255 of
/// `shape_coverage` across the 1 px `clip1` antialiasing rim:
/// - `r_max <= 2.0 * sigma` bounds the 3-point GL quadrature error on corner arcs;
/// - `min(w, h) >= 2 * r_max + 7.5 * sigma` keeps opposite-edge and opposite-corner
///   crosstalk beyond 3.75 sigma (`Phi(-3.75) < 1e-4`);
/// - `0.5 <= sigma < 32768`, `0 < sx <= 64`, `sigma * sx >= 1`, and `|coords| < 32768`
///   bound f32 cancellation and prevent FTZ underflow/overflow;
/// - `clip1` dimensions `>= 4` and radii in `[1.5, min(cw, ch) / 2]` keep each
///   corner inside its quadrant and clear of `shape_coverage`'s sharp-corner ridge.
pub(super) fn fast_shadow(i: &Instance) -> bool {
    if i.kind_brush != Kind::Shadow as u32 || i.flags != flags::CLIP1 {
        return false;
    }
    let [sx, skew_y, skew_x, sy, tx, ty] = i.xform;
    if skew_x != 0. || skew_y != 0. || !(sx > 0. && sx <= 64. && sx == sy) {
        return false;
    }
    let sigma = i.param[0];
    if !(0.5..32768.).contains(&sigma) || sigma * sx < 1.0 {
        return false;
    }
    let [ux, uy, uw, uh] = i.uv;
    if !(uw > 0. && uh > 0.)
        || ![ux, uy, ux + uw, uy + uh, tx, ty]
            .iter()
            .all(|v| v.abs() < 32768.)
        || !i.rect.iter().all(|v| v.is_finite())
        || !i.radii.iter().all(|r| *r >= 0. && *r < 32768.)
        || !i.color.iter().all(|c| c.is_finite())
    {
        return false;
    }
    let r_max = i.radii.into_iter().fold(0f32, f32::max);
    if r_max > 2.0 * sigma || uw.min(uh) < 2.0 * r_max + 7.5 * sigma {
        return false;
    }
    let [cx, cy, cw, ch] = i.clip1;
    if !(cw >= 4. && ch >= 4.) || ![cx, cy, cx + cw, cy + ch].iter().all(|v| v.abs() < 32768.) {
        return false;
    }
    let cr = clip1_radii(i);
    let clip1_r_min = cr.into_iter().fold(f32::INFINITY, f32::min);
    let clip1_r_max = cr.into_iter().fold(0f32, f32::max);
    clip1_r_min >= 1.5 && clip1_r_max <= 0.5 * cw.min(ch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance::RoundedClip;

    fn eligible_shadow() -> Instance {
        let mut i = Instance::new(Kind::Shadow, Brush::Solid, [4., 8., 216., 166.]);
        i.uv = [32., 36., 160., 110.];
        i.radii = [16.; 4];
        i.param[0] = 9.;
        i.color = [0., 0., 0., 0.2];
        i.set_clip(
            1,
            RoundedClip {
                rect: [32., 32., 160., 110.],
                radii: [16.; 4],
            },
        )
        .unwrap();
        i
    }

    #[test]
    fn uncommon_flags_and_brushes_use_the_general_program() {
        let mut i = Instance::rounded([0.; 4], [0.; 4], [1.; 4]);
        assert_eq!(CLASSES[class(&i).unwrap()].flags, 0);
        for bit in 0..32 {
            i.flags = 1 << bit;
            assert_eq!(
                class(&i).map(|p| CLASSES[p].flags),
                (i.flags & !CLIPS == 0).then_some(CLIPS)
            );
        }
        for brush in [Brush::Gradient, Brush::Pattern] {
            i = Instance::new(Kind::RoundedRect, brush, [0.; 4]);
            assert_eq!(class(&i), None);
        }
    }

    #[test]
    fn fast_shadow_guard_checks_every_domain_boundary() {
        let base = eligible_shadow();
        assert!(fast_shadow(&base));

        let mut clipped = base;
        clipped
            .set_clip(
                0,
                RoundedClip {
                    rect: [0., 0., 200., 200.],
                    radii: [0.; 4],
                },
            )
            .unwrap();
        assert!(!fast_shadow(&clipped), "CLIP0 must fall back");

        let mut skewed = base;
        skewed.xform[1] = 0.05;
        assert!(!fast_shadow(&skewed), "skew must fall back");

        let mut aniso = base;
        aniso.xform[3] = 1.5;
        assert!(!fast_shadow(&aniso), "anisotropic scale must fall back");

        let mut huge_scale = base;
        huge_scale.xform[0] = 65.;
        huge_scale.xform[3] = 65.;
        assert!(!fast_shadow(&huge_scale), "sx > 64 must fall back");

        // Test sigma*sx < 1 and sigma < 0.5 with zero caster radii so r_max does not mask them.
        let mut small_sigma = base;
        small_sigma.radii = [0.; 4];
        small_sigma.param[0] = 0.75;
        assert!(!fast_shadow(&small_sigma), "sigma*sx < 1 must fall back");
        small_sigma.xform[0] = 2.;
        small_sigma.xform[3] = 2.;
        assert!(fast_shadow(&small_sigma), "sigma=0.75 at 2x passes");
        small_sigma.param[0] = 0.4;
        small_sigma.xform[0] = 4.;
        small_sigma.xform[3] = 4.;
        assert!(
            !fast_shadow(&small_sigma),
            "local sigma < 0.5 must fall back"
        );

        let mut large_radius = base;
        large_radius.uv = [32., 36., 200., 150.];
        large_radius.radii = [18.1, 16., 16., 16.];
        assert!(
            !fast_shadow(&large_radius),
            "r_max > 2.0*sigma must fall back"
        );
        large_radius.radii[0] = 18.0;
        assert!(fast_shadow(&large_radius));

        let mut narrow = base;
        narrow.uv = [32., 36., 160., 99.];
        assert!(
            !fast_shadow(&narrow),
            "min(w,h) < 2*r_max + 7.5*sigma must fall back"
        );

        let mut far = base;
        far.uv[0] = 32768.;
        assert!(!fast_shadow(&far), "coords >= 32768 must fall back");

        let mut nan_sigma = base;
        nan_sigma.param[0] = f32::NAN;
        assert!(!fast_shadow(&nan_sigma), "NaN sigma must fall back");

        // Sharp and sub-1.5px clip1 corners diverge from shape_coverage's forward difference.
        let mut sharp_clip1 = base;
        sharp_clip1
            .set_clip(
                1,
                RoundedClip {
                    rect: [32., 32., 160., 110.],
                    radii: [0., 16., 16., 16.],
                },
            )
            .unwrap();
        assert!(
            !fast_shadow(&sharp_clip1),
            "sharp clip1 corner must fall back"
        );
        sharp_clip1
            .set_clip(
                1,
                RoundedClip {
                    rect: [32., 32., 160., 110.],
                    radii: [1.4375, 16., 16., 16.],
                },
            )
            .unwrap();
        assert!(!fast_shadow(&sharp_clip1), "clip1 r < 1.5 must fall back");
        sharp_clip1
            .set_clip(
                1,
                RoundedClip {
                    rect: [32., 32., 160., 110.],
                    radii: [1.5, 16., 16., 16.],
                },
            )
            .unwrap();
        assert!(fast_shadow(&sharp_clip1));

        // Asymmetric clip1 r > min(cw, ch) / 2 crosses quadrants.
        let mut asym_clip1 = base;
        asym_clip1
            .set_clip(
                1,
                RoundedClip {
                    rect: [32., 32., 160., 110.],
                    radii: [56., 1.5, 1.5, 1.5],
                },
            )
            .unwrap();
        assert!(
            !fast_shadow(&asym_clip1),
            "asymmetric clip1 r > min(w,h)/2 must fall back"
        );
        asym_clip1
            .set_clip(
                1,
                RoundedClip {
                    rect: [32., 32., 160., 110.],
                    radii: [55., 1.5, 1.5, 1.5],
                },
            )
            .unwrap();
        assert!(fast_shadow(&asym_clip1));

        // Thin clip1 (< 4 px) with valid proportional radius must still be rejected.
        let mut thin_clip1 = base;
        thin_clip1
            .set_clip(
                1,
                RoundedClip {
                    rect: [32., 32., 3.5, 110.],
                    radii: [1.5; 4],
                },
            )
            .unwrap();
        assert!(!fast_shadow(&thin_clip1), "clip1 dim < 4 must fall back");
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[test]
fn every_fixed_program_matches_the_general_program_pixel_for_pixel() {
    use crate::instance::{CoverageClip, Encoder, RoundedClip};
    let mut d = Wgpu::new().expect("wgpu device for program parity");
    // This test verifies fs_main override specializations in CLASSES, not fs_fast_shadow.
    d.fast_shadow = false;
    assert_eq!(d.programs.len(), CLASSES.len());
    eprintln!(
        "Ink: {} fixed material programs, device {:.3} ms, shaders {:.3} ms, cached {}",
        d.programs.len(),
        d.info.device_ms,
        d.info.shaders_ms,
        d.info.cached
    );
    let target = d.create_target(64, 48).unwrap();
    let image = d.create_texture(TextureDesc::rgba(4, 4)).unwrap();
    let alpha = d
        .create_texture(TextureDesc {
            width: 4,
            height: 4,
            format: Format::Alpha,
            mipmapped: false,
        })
        .unwrap();
    d.upload_texture(image, [0, 0], [4, 4], &[32, 16, 8, 128].repeat(16))
        .unwrap();
    d.upload_texture(alpha, [0, 0], [4, 4], &[127; 16]).unwrap();
    let bindings = Bindings {
        image,
        color: image,
        glyph: alpha,
        path: alpha,
        mask: alpha,
        ..Default::default()
    };
    let mut visited = [false; CLASSES.len()];
    for (index, class) in CLASSES.iter().enumerate() {
        let bits: Vec<_> = (0..32)
            .filter(|b| (class.flags & !class.required) & (1 << b) != 0)
            .collect();
        for combination in 0..(1 << bits.len()) {
            let clips = bits
                .iter()
                .enumerate()
                .fold(class.required, |value, (n, bit)| {
                    value
                        | if combination & (1 << n) != 0 {
                            1 << bit
                        } else {
                            0
                        }
                });
            let mut e = Encoder::new(64, 48);
            let mut i = Instance::rounded(
                [5.25, 4.5, 51., 37.],
                [9., 2., 6., 13.],
                [0.25, 0.125, 0.0625, 0.5],
            );
            i.kind_brush = class.kind_brush;
            i.flags = clips;
            i.uv = [0., 0., 1., 1.];
            i.param = [0.7, 0., 0., 0.];
            let kind = i.kind_brush & 65535;
            let brush = i.kind_brush >> 16;
            if kind == Kind::Ring as u32 {
                i.uv = [8., 8., 43., 27.];
                i.param = [4.; 4];
            }
            if kind == Kind::Shadow as u32 && brush == Brush::Solid as u32 {
                i.uv = [13., 12., 32., 23.];
                i.param = [3., 0., 0., 0.];
            }
            if kind == Kind::Shadow as u32 && brush == Brush::Image as u32 {
                i.xform = [51., 0., 0., 37., 5.25, 4.5];
                i.rect = [0., 0., 1., 1.];
                i.radii = [0.; 4];
                i.param = [0.; 4];
            }
            if i.kind_brush & 65535 == Kind::Shadow as u32 {
                i.flags |= flags::CLIP1;
            }
            for slot in 0..2 {
                if i.flags & [flags::CLIP0, flags::CLIP1][slot] != 0 {
                    i.set_clip(
                        slot,
                        RoundedClip {
                            rect: [13.25, 12.5, 33., 24.],
                            radii: [7., 2., 5., 9.],
                        },
                    )
                    .unwrap();
                }
            }
            if clips & flags::MASK != 0 {
                i.set_mask(CoverageClip {
                    texture: alpha,
                    rect: [2., 2., 58., 43.],
                    uv: [0., 0., 1., 1.],
                });
            }
            // A narrower fixed program owns the overlapping flag combinations.
            let selected = super::programs::class(&i).expect("a fixed program");
            if selected != index {
                assert!(selected < index);
                continue;
            }
            visited[index] = true;
            e.push(i, bindings).unwrap();
            let frame = e.finish();
            d.general_only = true;
            d.render(&frame, target, Some([0.; 4])).unwrap();
            let general = d.readback(target).unwrap();
            d.general_only = false;
            d.render(&frame, target, Some([0.; 4])).unwrap();
            let specialized = d.readback(target).unwrap();
            assert_eq!(general, specialized, "program {index}, clips {clips}");
            assert!(
                specialized.as_chunks::<4>().0.iter().any(|p| p[3] > 0),
                "program {index} drew nothing"
            );
        }
    }
    assert!(visited.into_iter().all(|v| v));
}
