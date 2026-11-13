use super::*;

const SHADOW: [u8; 4] = [34, 34, 34, 51];

/// Cards as `paint` draws them: a shadow 4 below the border box, sigma 9, then the box.
fn card(ink: &mut Ink, (x, y): (f32, f32), ts: Transform) {
    let outer = Shape::new((x, y, 150., 50.), [16.; 4]);
    let shape = Shape::new((x, y + 4., 150., 50.), [16.; 4]);
    assert!(ink.blurred_shadow(&shape, SHADOW, 9., &outer, ts));
    ink.fill(&outer, [255; 4], ts);
}

fn scene(ink: &mut Ink) {
    card(ink, (20., 14.), Transform::identity());
    ink.push_opacity(0.6);
    card(ink, (0., 0.), Transform::from_translate(20.3, 84.65));
    ink.pop_opacity();
    // A rounded clip takes the first clip slot; the exclusion keeps the second.
    ink.push_clip(
        &Shape::new((10., 150., 120., 75.), [24.; 4]),
        Transform::identity(),
    );
    card(ink, (20.5, 154.5), Transform::identity());
    ink.pop_clip();
}

/// The largest byte difference and the percentage of pixels that differ.
/// RGBA8 storage, subpixel filtering and composition can change pixel values.
fn difference(a: &Pixmap, b: &Pixmap) -> (u8, f64) {
    let mut max = 0;
    let mut pixels = 0;
    for (a, b) in a.data().chunks_exact(4).zip(b.data().chunks_exact(4)) {
        let d = a.iter().zip(b).map(|(a, b)| a.abs_diff(*b)).max().unwrap();
        max = max.max(d);
        pixels += usize::from(d != 0);
    }
    (max, pixels as f64 * 100. / (a.width() * a.height()) as f64)
}

#[test]
fn a_rastered_shadow_draws_as_the_analytic_one_wherever_it_lands() {
    let mut analytic = Ink::new().unwrap();
    analytic.shadows = Shadows::new(false);
    let mut cached = Ink::new().unwrap();
    cached.shadows = Shadows::new(true);
    cached.shadows.bands = true;
    for (n, scale) in [1., 1.5, 2.].into_iter().enumerate() {
        let hits = cached.shadows.hits;
        for ink in [&mut analytic, &mut cached] {
            ink.begin(200., 240., scale);
            scene(ink);
        }
        let (max, percent) = difference(&analytic.finish().unwrap(), &cached.finish().unwrap());
        // Two shadows overlap.
        assert!(
            max <= 2 && percent < 2.,
            "scale {scale}: {max}, {percent:.2}%"
        );
        // One raster per scale, whatever the cards' positions, clips and opacity.
        assert_eq!(cached.shadows.rasters.len(), n + 1);
        assert_eq!(cached.shadows.hits - hits, 2);

        // A kept row moved by whole device pixels keeps its fractional phase.
        let bounds = (-10., -10., 190., 90.);
        cached.begin(200., 240., scale);
        cached.row_begin(1, (20.3, 14.65), None);
        card(&mut cached, (20.3, 14.65), Transform::identity());
        assert_eq!(cached.row_end(bounds), 1);
        for delta in [(0., 0.), (10., 90.)] {
            let origin = (20.3 + delta.0, 14.65 + delta.1);
            cached.begin(200., 240., scale);
            cached.row_draw(1, origin, bounds);
            analytic.begin(200., 240., scale);
            card(&mut analytic, origin, Transform::identity());
            let (max, percent) = difference(&analytic.finish().unwrap(), &cached.finish().unwrap());
            assert!(
                max <= 1 && percent < 2.,
                "scale {scale}, row moved {delta:?}: {max}, {percent:.2}%"
            );
        }
    }
}

#[cfg(not(windows))]
#[test]
fn the_cache_keys_geometry_not_position_and_keeps_its_budget() {
    let mut ink = Ink::from_device(Selected::Fake(ordered_tests::Fake::default()));
    ink.shadows = Shadows::new(false);
    ink.begin(80., 80., 2.);
    let outer = Shape::new((10.25, 20.5, 40., 30.), [6.; 4]);
    let draw = |ink: &mut Ink, dx: f32, sigma: f32| {
        let shape = Shape::new((10.25 + dx, 20.5, 40., 30.), [6.; 4]);
        assert!(ink.blurred_shadow(&shape, SHADOW, sigma, &outer, Transform::identity()));
        ink.encoder.take_frame()
    };
    let analytic = draw(&mut ink, 0., 3.).instances[0];
    ink.shadows = Shadows::new(true);
    ink.shadows.bands = false;
    for dx in [0., 0.375, 7.] {
        let frame = draw(&mut ink, dx, 3.);
        let i = frame.instances[0];
        assert_eq!(
            i.kind_brush,
            Kind::Shadow as u32 | (Brush::Image as u32) << 16
        );
        assert_ne!(frame.batches[0].bindings.image, TextureId(0));
        // The quad's device origin, unrounded, and the same live exclusion.
        assert_eq!(i.xform, [120., 0., 0., 100., 2. * (10.25 + dx - 10.), 21.]);
        assert_eq!(
            (i.clip1, i.clip_radii),
            (analytic.clip1, analytic.clip_radii)
        );
        assert_eq!((i.color, i.param[0]), (analytic.color, 0.));
    }
    assert_eq!((ink.shadows.rasters.len(), ink.shadows.hits), (1, 2));
    assert_eq!(ink.shadows.bytes, 120 * 100 * 4);

    // Too little blur to resample, or no budget left, is drawn analytically.
    assert_eq!(
        draw(&mut ink, 0., 1.5).instances[0].kind_brush,
        Kind::Shadow as u32
    );
    let huge = Shape::new((0., 0., 1100., 1100.), [6.; 4]);
    assert!(ink.blurred_shadow(&huge, SHADOW, 3., &huge, Transform::identity()));
    let frame = ink.encoder.take_frame();
    assert_eq!(frame.instances[0].kind_brush, Kind::Shadow as u32);
    assert!(ink.shadows.full);
    assert_eq!(ink.shadows.rasters.len(), 1);
}

#[test]
fn bands_preserve_the_full_cached_shadow_at_fractional_positions() {
    let mut full = Ink::new().unwrap();
    full.shadows = Shadows::new(true);
    full.shadows.bands = false;
    let mut bands = Ink::new().unwrap();
    bands.shadows = Shadows::new(true);
    bands.shadows.bands = true;
    for scale in [1., 1.5, 2.] {
        for phase in [0., 0.25, 0.5, 0.75] {
            for ink in [&mut full, &mut bands] {
                ink.begin(260., 380., scale);
                ink.push_opacity(0.7);
                ink.push_clip(
                    &Shape::new((5., 10., 245., 360.), [25.; 4]),
                    Transform::identity(),
                );
                for (n, (w, h)) in [(180., 40.), (40., 150.), (40., 40.)]
                    .into_iter()
                    .enumerate()
                {
                    let x = 20.3 + phase;
                    let y = 25.65 + n as f32 * 100. + phase;
                    let outer = Shape::new((x, y, w, h), [2., 16., 5., 12.]);
                    // Different offset and spread: the hole belongs to outer, not shape.
                    let shadow =
                        Shape::new((x - 9., y + 3., w + 12., h + 12.), [8., 22., 11., 18.]);
                    assert!(ink.blurred_shadow(
                        &shadow,
                        [0, 0, 0, 255],
                        6.,
                        &outer,
                        Transform::identity()
                    ));
                    // No covering fill: clipping errors remain visible inside the box.
                }
                ink.pop_clip();
                ink.pop_opacity();
            }
            let (max, percent) = difference(&full.finish().unwrap(), &bands.finish().unwrap());
            assert!(
                max <= 1 && percent < 0.1,
                "scale {scale}, phase {phase}: {max}, {percent}%"
            );
        }
    }
}

#[cfg(not(windows))]
#[test]
fn bands_skip_most_of_a_wide_cards_shadow_quad_without_extra_batches() {
    let mut ink = Ink::from_device(Selected::Fake(ordered_tests::Fake::default()));
    ink.shadows = Shadows::new(true);
    ink.shadows.bands = true;
    ink.begin(800., 1280., 1.);
    let outer = Shape::new((32., 50., 736., 160.), [16.; 4]);
    let shape = Shape::new((32., 54., 736., 160.), [16.; 4]);
    assert!(ink.blurred_shadow(&shape, SHADOW, 9., &outer, Transform::identity()));
    let frame = ink.encoder.take_frame();
    assert_eq!(frame.instances.len(), 4);
    assert_eq!(frame.batches.len(), 1);
    let area: f32 = frame
        .instances
        .iter()
        .map(|i| {
            assert_eq!(i.rect, i.uv);
            i.rect[2] * i.rect[3]
        })
        .sum();
    assert!(area < 0.4, "fraction of full quad: {area}");
}
