use super::*;

fn quad() -> Instance {
    Instance::rounded([0., 0., 10., 10.], [2.; 4], [1.; 4])
}

fn clip(x: f32) -> RoundedClip {
    RoundedClip {
        rect: [x, x, 20., 20.],
        radii: [2.031, 3.5, 4.25, 5.],
    }
}

#[test]
fn batches_preserve_order_and_break_only_for_scissor_or_bindings() {
    let mut e = Encoder::new(100, 100);
    let a = Bindings::default();
    let b = Bindings {
        image: TextureId(7),
        ..a
    };
    e.push(quad(), a).unwrap();
    e.push(quad(), a).unwrap();
    e.push_clip(Clip::Rounded(clip(3.))).unwrap();
    e.push(quad(), a).unwrap();
    e.push(quad(), b).unwrap();
    e.push_clip(Clip::Scissor(Scissor {
        x: 10,
        y: 20,
        width: 100,
        height: 15,
    }))
    .unwrap();
    e.push(quad(), b).unwrap();
    e.pop_clip();
    e.push(quad(), b).unwrap();
    let f = e.finish();
    assert_eq!(
        f.batches
            .iter()
            .map(|b| b.instances.clone())
            .collect::<Vec<_>>(),
        [0..3, 3..4, 4..5, 5..6]
    );
    assert_eq!(
        f.batches[2].scissor,
        Scissor {
            x: 10,
            y: 20,
            width: 90,
            height: 15
        }
    );
    assert_eq!(f.instances[2].clip0, clip(3.).rect);
    assert_eq!(f.instances[0].flags & flags::CLIP0, 0);
}

#[test]
fn clips_fill_slots_refuse_overflow_and_restore_after_pop() {
    let mut e = Encoder::new(100, 100);
    let mask = CoverageClip {
        texture: TextureId(0),
        rect: [1., 2., 30., 40.],
        uv: [0.1, 0.2, 0.3, 0.4],
    };
    e.push_clip(Clip::Rounded(clip(1.))).unwrap();
    e.push_clip(Clip::Rounded(clip(2.))).unwrap();
    e.push_clip(Clip::Coverage(mask)).unwrap();
    e.push(quad(), Bindings::default()).unwrap();
    for extra in [
        Clip::Rounded(clip(3.)),
        Clip::Coverage(mask),
        Clip::Transformed,
    ] {
        assert_eq!(e.push_clip(extra), Err(NeedsLayer));
        assert_eq!(e.push(quad(), Bindings::default()), Err(NeedsLayer));
        e.pop_clip();
    }
    e.pop_clip();
    e.pop_clip();
    e.push(quad(), Bindings::default()).unwrap();
    let f = e.finish();
    assert_eq!(f.instances.len(), 2);
    assert_eq!(f.instances[0].clip0, clip(2.).rect);
    assert_eq!(f.instances[0].clip1, clip(1.).rect);
    assert_eq!(f.instances[0].mask_rect, mask.rect);
    assert_eq!(f.instances[1].flags & (flags::CLIP1 | flags::MASK), 0);
    assert_eq!(
        f.instances[0].clip_radii,
        [
            32 | (56 << 16),
            68 | (80 << 16),
            32 | (56 << 16),
            68 | (80 << 16)
        ]
    );
}

#[test]
fn replay_preserves_inner_clips_and_refuses_a_third_without_emitting() {
    let mut e = Encoder::new(20, 20);
    let mut row = quad();
    row.set_clip(0, clip(1.)).unwrap();
    e.push_clip(Clip::Rounded(clip(2.))).unwrap();
    e.push(row, Bindings::default()).unwrap();
    e.push_clip(Clip::Rounded(clip(3.))).unwrap();
    assert_eq!(e.push(row, Bindings::default()), Err(NeedsLayer));
    let f = e.finish();
    assert_eq!(f.instances.len(), 1);
    assert_eq!(f.instances[0].clip0, row.clip0);
    assert_eq!(f.instances[0].clip1, clip(2.).rect);
    assert_eq!(pack_radii([5000.; 4]), Err(NeedsLayer));
}

#[test]
fn disjoint_scissors_emit_nothing_and_pop_restores_the_viewport() {
    let mut e = Encoder::new(10, 10);
    e.push_clip(Clip::Scissor(Scissor {
        x: 20,
        y: 0,
        width: 10,
        height: 10,
    }))
    .unwrap();
    e.push(quad(), Bindings::default()).unwrap();
    e.pop_clip();
    e.push(quad(), Bindings::default()).unwrap();
    assert_eq!(e.finish().instances.len(), 1);
}

#[test]
fn source_over_never_batches_with_replace_or_blend() {
    let mut encoder = Encoder::new(8, 8);
    for flags in [0, flags::REPLACE, 0, 1 << flags::BLEND_SHIFT, 0] {
        let mut i = Instance::rounded([0., 0., 8., 8.], [0.; 4], [1.; 4]);
        i.flags = flags;
        encoder
            .push(
                i,
                Bindings {
                    destination: TextureId(7),
                    ..Default::default()
                },
            )
            .unwrap();
    }
    let frame = encoder.finish();
    assert_eq!(frame.batches.len(), 5);
}

#[test]
fn snapshot_bounds_cover_transformed_quads_and_intersect_the_effective_clips() {
    let mut encoder = Encoder::new(1920, 1080);
    let mut image = Instance::new(Kind::RoundedRect, Brush::Image, [10., 20., 30., 40.]);
    image.xform = [0., 1., -1., 0., 300., 200.];
    assert_eq!(
        encoder.bounds(&image),
        Scissor::from_rect([239., 209., 42., 32.])
    );
    let scissor = Scissor::from_rect([250., 220., 15., 10.]);
    encoder.push_clip(Clip::Scissor(scissor)).unwrap();
    assert_eq!(encoder.bounds(&image), scissor);
    encoder.pop_clip();
    encoder
        .push_clip(Clip::Rounded(RoundedClip {
            rect: [250., 220., 15., 10.],
            radii: [2.; 4],
        }))
        .unwrap();
    assert_eq!(
        encoder.bounds(&image),
        Scissor::from_rect([248., 218., 19., 14.])
    );
    encoder
        .push_clip(Clip::Coverage(CoverageClip {
            texture: TextureId(1),
            rect: [252., 221., 8., 7.],
            uv: [0., 0., 1., 1.],
        }))
        .unwrap();
    assert_eq!(
        encoder.bounds(&image),
        Scissor::from_rect([252., 221., 8., 7.])
    );
    image = Instance::new(Kind::Coverage, Brush::Image, [254., 222., 3., 2.]);
    image.xform = [2., 1., 1., 3., 99., 17.];
    assert_eq!(encoder.bounds(&image), Scissor::from_rect(image.rect));
}

#[test]
fn draw_bounds_unite_clipped_draws_and_stop_at_the_limit() {
    let mut e = Encoder::new(100, 100);
    e.push(quad(), Bindings::default()).unwrap();
    e.push_clip(Clip::Scissor(Scissor {
        x: 50,
        y: 50,
        width: 10,
        height: 10,
    }))
    .unwrap();
    e.push(
        Instance::rounded([40., 40., 30., 30.], [0.; 4], [1.; 4]),
        Bindings::default(),
    )
    .unwrap();
    let frame = e.take_frame();
    let full = Scissor::full([100, 100]);
    let clipped = Scissor {
        x: 50,
        y: 50,
        width: 10,
        height: 10,
    };
    let quad_bounds = Scissor::from_rect(Encoder::device_rect(&quad()));
    assert_eq!(
        frame.draw_bounds(None, full),
        Some(quad_bounds.union(clipped))
    );
    // The walk stops once the bound covers the limit, before the clipped draw.
    assert_eq!(frame.draw_bounds(None, quad_bounds), Some(quad_bounds));

    // Every backend refuses a range past the instances; it adds nothing here.
    let mut refused = frame.clone();
    refused.batches[0].instances = 0..9;
    assert_eq!(refused.draw_bounds(None, full), Some(clipped));

    // A skewed draw's vertices are uncertain: it takes its batch's scissor.
    let mut e = Encoder::new(100, 100);
    let scissor = Scissor {
        x: 30,
        y: 30,
        width: 40,
        height: 40,
    };
    e.push_clip(Clip::Scissor(scissor)).unwrap();
    let mut skewed = Instance::rounded([40., 40., 10., 10.], [0.; 4], [1.; 4]);
    skewed.xform = [1., 0.5, 0., 1., 0., 0.];
    e.push(skewed, Bindings::default()).unwrap();
    assert_eq!(e.take_frame().draw_bounds(None, full), Some(scissor));
    // So does a draw too far out for the CPU and the shader to agree.
    e.push(
        Instance::rounded([70000., 40., 10., 10.], [0.; 4], [1.; 4]),
        Bindings::default(),
    )
    .unwrap();
    assert_eq!(e.take_frame().draw_bounds(None, full), Some(scissor));
    // And a draw whose extent is NaN, which no bound contains.
    e.push(
        Instance::rounded([40., 40., f32::NAN, 10.], [0.; 4], [1.; 4]),
        Bindings::default(),
    )
    .unwrap();
    assert_eq!(e.take_frame().draw_bounds(None, full), Some(scissor));
    // A coverage quad is in device space: its brush transform does not move it.
    let mut coverage = Instance::new(Kind::Coverage, Brush::Solid, [40., 40., 10., 10.]);
    coverage.xform = [1., 0.5, 0., 1., 0., 0.];
    e.push(coverage, Bindings::default()).unwrap();
    assert_eq!(
        e.take_frame().draw_bounds(None, full),
        Some(Scissor::from_rect([40., 40., 10., 10.]))
    );

    let empty = Scissor {
        x: 200,
        y: 200,
        width: 0,
        height: 0,
    };
    assert_eq!(empty.union(clipped), clipped);
    assert_eq!(clipped.union(empty), clipped);
}

fn outer_shadow() -> Instance {
    let mut i = Instance::new(Kind::Shadow, Brush::Solid, [0., 0., 200., 100.]);
    i.uv = [20., 20., 160., 60.];
    i.radii = [8.; 4];
    i.param[0] = 6.;
    i.set_clip(
        1,
        RoundedClip {
            rect: i.uv,
            radii: i.radii,
        },
    )
    .unwrap();
    i
}

#[test]
fn shadow_quads_keep_order_clips_and_shape_in_one_batch() {
    let mut e = Encoder::new(300, 200);
    let mut shadow = outer_shadow().translated([11.25, 7.5]);
    let parent = RoundedClip {
        rect: [0., 0., 250., 150.],
        radii: [4.; 4],
    };
    let mask = CoverageClip {
        texture: TextureId(7),
        rect: parent.rect,
        uv: [0., 0., 1., 1.],
    };
    e.push_clip(Clip::Rounded(parent)).unwrap();
    e.push_clip(Clip::Coverage(mask)).unwrap();
    e.push(quad(), Bindings::default()).unwrap();
    e.push(shadow, Bindings::default()).unwrap();
    e.push(quad(), Bindings::default()).unwrap();
    let frame = e.finish();
    assert_eq!(frame.batches.len(), 1);
    assert_eq!(frame.batches[0].instances, 0..6);
    shadow.set_clip(0, parent).unwrap();
    shadow.set_mask(mask);
    let mut area = 0.;
    for piece in &frame.instances[1..5] {
        assert_eq!(
            *piece,
            Instance {
                rect: piece.rect,
                ..shadow
            }
        );
        assert_eq!(frame.batches[0].bindings.mask, mask.texture);
        area += piece.rect[2] * piece.rect[3];
    }
    assert!(area < shadow.rect[2] * shadow.rect[3] * 0.75);
    assert_eq!(frame.instances[0].rect, quad().rect);
    assert_eq!(frame.instances[5].rect, quad().rect);
}

#[test]
fn shadow_quads_leave_small_transformed_and_unsafe_inputs_whole() {
    let shadow = outer_shadow();
    assert!(shadow_quads(&shadow).is_some());
    let mut cases = [shadow; 7];
    cases[0].clip1[2] = 10.;
    cases[1].xform[1] = 0.2;
    cases[2].xform[0] = -1.;
    cases[3].clip1[0] = 32768.;
    cases[4].param[0] = 1e-35;
    cases[5].rect[0] = f32::NAN;
    cases[6].flags |= flags::REPLACE;
    for i in cases {
        assert!(shadow_quads(&i).is_none(), "{i:?}");
    }
}

#[test]
fn shadow_quads_keep_the_original_under_large_affine_cancellation() {
    for (rect, scale) in [
        ([-3_333_336_576., -3_333_336_576., 9984., 9984.], 0.3),
        ([-2., -2., 4., 4.], 1e9),
    ] {
        let mut i = Instance::new(Kind::Shadow, Brush::Solid, rect);
        i.xform = [scale, 0., 0., scale, 1e9, 1e9];
        i.uv = rect;
        i.radii = [16.; 4];
        i.param[0] = 4.;
        i.color = [0.2, 0.1, 0.05, 0.5];
        i.set_clip(
            1,
            RoundedClip {
                rect: [100., 100., 800., 150.],
                radii: [16.; 4],
            },
        )
        .unwrap();
        let mut e = Encoder::new(1000, 500);
        e.push(i, Bindings::default()).unwrap();
        assert_eq!(e.finish().instances, [i]);
    }
}
