use super::*;
use crate::ink::ordered_tests::Fake;

fn ink() -> Ink {
    let mut ink = Ink::from_device(Selected::Fake(Fake::default()));
    ink.shadows = Shadows::new(true);
    ink.shadows.bands = false;
    ink.begin(80., 80., 1.);
    ink
}

fn fake(ink: &mut Ink) -> &mut Fake {
    let Selected::Fake(device) = &mut ink.device else {
        unreachable!()
    };
    device
}

// Four of these rasters fit in the cache, and a fifth needs a victim.
fn shadow(ink: &mut Ink, variant: u32) -> Option<TextureId> {
    let before = ink.shadows.clock;
    let shape = Shape::new((20., 20., 974., 974.), [8. + variant as f32; 4]);
    assert!(ink.blurred_shadow(&shape, [0, 0, 0, 160], 4., &shape, Transform::identity()));
    ink.shadows
        .rasters
        .values()
        .find(|r| r.used > before)
        .map(|r| r.texture)
}

fn contains(ink: &Ink, texture: TextureId) -> bool {
    ink.shadows.rasters.values().any(|r| r.texture == texture)
}

#[test]
fn shadow_lru_keeps_this_frames_references_across_flushes() {
    let mut ink = ink();
    let a = shadow(&mut ink, 0).unwrap();
    ink.push_opacity(0.5);
    let [b, c, d] = [1, 2, 3].map(|n| shadow(&mut ink, n).unwrap());
    ink.flush().unwrap();
    assert_eq!(shadow(&mut ink, 4), None);
    assert_eq!(ink.shadows.rasters.len(), 4);
    assert!(fake(&mut ink).retired.is_empty());
    ink.pop_opacity();
    ink.flush().unwrap();

    ink.begin(80., 80., 1.);
    assert_eq!(shadow(&mut ink, 0), Some(a));
    let e = shadow(&mut ink, 4).unwrap();
    assert!(
        !contains(&ink, b),
        "the hit on A makes B the least recently used"
    );
    for texture in [a, c, d, e] {
        assert!(contains(&ink, texture));
    }
    ink.flush().unwrap();

    for variant in 5..30 {
        ink.begin(80., 80., 1.);
        assert_eq!(shadow(&mut ink, 0), Some(a));
        assert!(shadow(&mut ink, variant).is_some());
        assert!(ink.shadows.bytes <= BUDGET);
        ink.flush().unwrap();
    }
    assert!(!fake(&mut ink).retired.contains(&a));
}

#[test]
fn shadow_lru_uses_a_completed_victim_without_waiting_for_an_older_one() {
    let mut ink = ink();
    let a = shadow(&mut ink, 0).unwrap();
    ink.push_opacity(0.5);
    let [b, _, _] = [1, 2, 3].map(|n| shadow(&mut ink, n).unwrap());
    ink.pop_opacity();
    ink.flush().unwrap();
    let done = ink.device.last_use(b).unwrap();
    assert!(
        ink.device.last_use(a).unwrap() > done,
        "the parent pass submits last"
    );

    fake(&mut ink).complete = Some(Submission(0));
    ink.begin(80., 80., 1.);
    assert_eq!(
        shadow(&mut ink, 4),
        None,
        "all candidates are still on the GPU"
    );
    assert!(fake(&mut ink).retired.is_empty());
    fake(&mut ink).complete = Some(done);
    assert!(shadow(&mut ink, 4).is_some());
    assert!(contains(&ink, a));
    assert!(!contains(&ink, b));
    assert!(ink.shadows.bytes <= BUDGET);
    ink.flush().unwrap();
}

#[test]
fn trimmed_shadow_storage_counts_until_gpu_completion() {
    for hidden in [false, true] {
        let mut ink = ink();
        let old = [0, 1, 2, 3].map(|n| shadow(&mut ink, n).unwrap());
        ink.flush().unwrap();
        let bytes = ink.shadows.bytes;
        fake(&mut ink).complete = Some(Submission(0));
        ink.trim(hidden);
        assert!(ink.error.is_none());
        assert!(ink.shadows.rasters.is_empty());
        assert_eq!(ink.shadows.bytes, bytes);
        assert_eq!(ink.target.is_none(), hidden);
        for texture in old {
            assert!(fake(&mut ink).retired.contains(&texture));
        }

        ink.begin(80., 80., 1.);
        assert_eq!(shadow(&mut ink, 4), None);
        assert_eq!(ink.shadows.bytes, bytes);
        fake(&mut ink).complete = None;
        ink.begin(80., 80., 1.);
        assert_eq!(ink.shadows.bytes, 0);
        let fresh = shadow(&mut ink, 4).unwrap();
        assert!(!old.contains(&fresh));
        ink.flush().unwrap();
    }
}

#[test]
fn a_kept_rows_shadow_slots_and_layers_recreate_after_trim() {
    let mut ink = ink();
    ink.row_begin(1, (0., 0.), None);
    shadow(&mut ink, 0);
    ink.slot_begin(7);
    shadow(&mut ink, 1);
    ink.slot_end();
    assert!(ink.layer_begin(
        2,
        Transform::identity(),
        (0., 0.),
        (0., 0.),
        [0., 0., 1., 0., 0.5, 0., 0., 1.]
    ));
    shadow(&mut ink, 2);
    ink.layer_end();
    ink.row_end((0., 0., 1000., 1000.));
    assert!(
        ink.shadows.rasters.is_empty(),
        "a kept drawing owns geometry, not a raster"
    );
    ink.row_draw(1, (0., 0.), (0., 0., 1000., 1000.));
    ink.flush().unwrap();
    assert_eq!(ink.shadows.rasters.len(), 3);
    let old: Vec<_> = ink.shadows.rasters.values().map(|r| r.texture).collect();

    ink.trim(false);
    assert!(ink.error.is_none());
    assert_eq!(ink.shadows.bytes, 0);
    ink.begin(80., 80., 1.);
    assert!(ink.row_reusable(1, (0., 0.)));
    assert!(ink.slot_again(1, 7));
    shadow(&mut ink, 3);
    ink.slot_end();
    ink.row_draw(1, (0., 0.), (0., 0., 1000., 1000.));
    ink.flush().unwrap();
    assert_eq!(ink.shadows.rasters.len(), 3);
    for texture in old {
        assert!(!contains(&ink, texture));
    }
    ink.row_free(1);
    ink.trim(true);
    assert_eq!(ink.shadows.bytes, 0);
}

#[test]
fn failed_shadow_rasters_retire_once_and_preserve_device_loss() {
    for (draw, cleanup, expected, lost) in [
        (Error::Device("draw".into()), None, "device: draw", false),
        (
            Error::Device("draw".into()),
            Some(Error::Device("cleanup".into())),
            "device: draw",
            false,
        ),
        (
            Error::DeviceLost("draw".into()),
            Some(Error::Device("cleanup".into())),
            "device lost: draw",
            true,
        ),
        (
            Error::Device("draw".into()),
            Some(Error::DeviceLost("cleanup".into())),
            "device lost: cleanup",
            true,
        ),
    ] {
        let mut ink = ink();
        fake(&mut ink).fail_render = Some(draw);
        fake(&mut ink).fail_retire = cleanup;
        assert_eq!(shadow(&mut ink, 0), None);
        assert_eq!(ink.error.as_deref(), Some(expected));
        assert_eq!(ink.device_lost, lost);
        assert_eq!(fake(&mut ink).retired.len(), 1);
        assert!(ink.shadows.rasters.is_empty());
        assert_eq!(
            shadow(&mut ink, 0),
            None,
            "the failed frame cannot allocate again"
        );
        if !lost {
            ink.begin(80., 80., 1.);
            assert_eq!(ink.shadows.bytes, 0);
            assert!(shadow(&mut ink, 0).is_some());
            ink.flush().unwrap();
            assert_eq!(fake(&mut ink).retired.len(), 1);
        }
    }

    // wgpu's last_use reads the error latch before looking up the texture.
    let mut ink = ink();
    fake(&mut ink).fail_render = Some(Error::Device("draw".into()));
    fake(&mut ink).fail_last_use = Some(Error::DeviceLost("last use".into()));
    fake(&mut ink).fail_retire = Some(Error::Device("cleanup".into()));
    shadow(&mut ink, 0);
    assert_eq!(ink.error.as_deref(), Some("device lost: last use"));
    assert!(ink.device_lost);
    assert_eq!(
        fake(&mut ink).retired.len(),
        1,
        "still release after last_use fails"
    );
    assert!(ink.shadows.rasters.is_empty());
}

#[test]
fn allocation_and_retirement_errors_leave_the_shadow_cache_retryable() {
    let mut ink = ink();
    fake(&mut ink).fail_create = Some(Error::Device("allocation".into()));
    assert_eq!(shadow(&mut ink, 0), None);
    assert_eq!(ink.error.as_deref(), Some("device: allocation"));
    assert_eq!(ink.shadows.bytes, 0);
    assert!(fake(&mut ink).retired.is_empty());
    ink.begin(80., 80., 1.);
    for n in 0..4 {
        assert!(shadow(&mut ink, n).is_some());
    }
    ink.flush().unwrap();
    ink.begin(80., 80., 1.);
    fake(&mut ink).fail_retire = Some(Error::Device("retirement".into()));
    assert_eq!(shadow(&mut ink, 4), None);
    assert_eq!(ink.error.as_deref(), Some("device: retirement"));
    assert_eq!(ink.shadows.rasters.len(), 3);
    assert_eq!(fake(&mut ink).retired.len(), 1);
    ink.begin(80., 80., 1.);
    assert!(shadow(&mut ink, 4).is_some());
    ink.flush().unwrap();
    assert_eq!(
        fake(&mut ink).retired.len(),
        1,
        "do not retire an invalidated ID twice"
    );
    assert!(ink.shadows.bytes <= BUDGET);
}
