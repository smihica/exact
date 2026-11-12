#![cfg(target_vendor = "apple")]
//! A canvas that clips every frame without `save` keeps one region (HTML's
//! `clip()` intersects it), so each frame's scene holds a bounded number of
//! clip layers. No GPU: the scenes are encoded, not rendered.
use exact_canvas::Context2d;
use exact_canvas_vello::replay::{Frame, Host, Replayer};
use vello::kurbo::BezPath;
use vello::peniko::ImageData;
use vello::Scene;

struct NoHost;

impl Host for NoHost {
    fn text(&mut self, _: &[f64], _: &[u32], _: bool) -> Option<BezPath> {
        None
    }
    fn image(&mut self, _: &str) -> Option<ImageData> {
        None
    }
}

/// Replays `frames` draws as the GPU module does (`gpu.rs` `replay`): each
/// frame a new scene over the last frame's pixels. Returns each frame's clip
/// count.
fn clips_per_frame(frames: usize, draw: impl Fn(&Context2d, usize)) -> Vec<u32> {
    let ctx = Context2d::new();
    let mut replayer = Replayer::new(200, 100, 1.0);
    (0..frames)
        .map(|i| {
            draw(&ctx, i);
            let mut frame = Frame {
                scene: Scene::new(),
                keep: true,
                shadows: Vec::new(),
            };
            replayer.begin(&mut frame.scene);
            for list in ctx.take_lists() {
                replayer.apply(&list, &mut frame, &mut NoHost).unwrap();
            }
            replayer.end(&mut frame.scene);
            frame.scene.encoding().n_clips
        })
        .collect()
}

fn assert_bounded(what: &str, clips: &[u32]) {
    let (second, last) = (clips[1], clips[clips.len() - 1]);
    assert!(
        last <= second,
        "{what}: frame 2 has {second} clips, frame {} has {last}",
        clips.len()
    );
}

#[test]
fn the_same_rectangle_clipped_every_frame_without_save_stays_one_layer() {
    let clips = clips_per_frame(600, |ctx, i| {
        ctx.clear_rect(0.0, 0.0, 200.0, 100.0);
        ctx.begin_path();
        ctx.rect(10.0, 10.0, 100.0, 50.0);
        ctx.clip();
        ctx.fill_rect(20.0 + (i % 60) as f64, 20.0, 30.0, 30.0);
    });
    assert_bounded("same rectangle", &clips);
}

#[test]
fn the_same_circle_clipped_every_frame_without_save_stays_one_layer() {
    let clips = clips_per_frame(600, |ctx, i| {
        ctx.clear_rect(0.0, 0.0, 200.0, 100.0);
        ctx.begin_path();
        ctx.arc(50.0, 50.0, 40.0, 0.0, std::f64::consts::TAU)
            .unwrap();
        ctx.clip();
        ctx.fill_rect((i % 60) as f64, 20.0, 30.0, 30.0);
    });
    assert_bounded("same circle", &clips);
}

#[test]
fn a_moving_rectangle_clipped_every_frame_without_save_folds_into_one_layer() {
    let clips = clips_per_frame(600, |ctx, i| {
        ctx.clear_rect(0.0, 0.0, 200.0, 100.0);
        ctx.begin_path();
        ctx.rect((i % 50) as f64, 0.0, 150.0, 100.0);
        ctx.clip();
        ctx.fill_rect(0.0, 0.0, 200.0, 100.0);
    });
    assert_bounded("moving rectangle", &clips);
}

#[test]
fn a_folded_clip_still_ends_at_its_restore_and_an_outer_one_does_not() {
    let ctx = Context2d::new();
    let mut replayer = Replayer::new(200, 100, 1.0);
    let mut frame = Frame {
        scene: Scene::new(),
        keep: false,
        shadows: Vec::new(),
    };
    let replay = |replayer: &mut Replayer, frame: &mut Frame| {
        for list in ctx.take_lists() {
            replayer.apply(&list, frame, &mut NoHost).unwrap();
        }
    };
    ctx.save();
    for x in [0.0, 10.0] {
        ctx.begin_path();
        ctx.rect(x, 0.0, 100.0, 100.0);
        ctx.clip();
    }
    replay(&mut replayer, &mut frame);
    assert!(replayer.clipped());
    ctx.restore();
    replay(&mut replayer, &mut frame);
    assert!(!replayer.clipped(), "restore drops the folded clip");

    ctx.begin_path();
    ctx.rect(0.0, 0.0, 100.0, 100.0);
    ctx.clip();
    ctx.save();
    ctx.begin_path();
    ctx.rect(10.0, 0.0, 100.0, 100.0);
    ctx.clip();
    ctx.restore();
    replay(&mut replayer, &mut frame);
    assert!(
        replayer.clipped(),
        "a clip inside save does not fold into the one outside it"
    );
}

#[test]
fn a_rectangle_folded_across_frames_paints_only_the_intersection() {
    use exact_canvas_vello::surface::make;
    use exact_canvas_vello::{ecg_canvas_free, ecg_canvas_new, ecg_canvas_replay};
    let (w, h) = (40u32, 10u32);
    let canvas = ecg_canvas_new(w, h, 1.0);
    if canvas.is_null() {
        let why = "missing gpu: the folded clip's pixels need Metal";
        assert_ne!(std::env::var("EXACT_REQUIRED").as_deref(), Ok("1"), "{why}");
        eprintln!("SKIP {why}");
        return;
    }
    let surface = make::surface(w, h);
    let ctx = Context2d::new();
    for (x, paint) in [(0.0, false), (10.0, true)] {
        ctx.begin_path();
        ctx.rect(x, 0.0, 20.0, 10.0);
        ctx.clip();
        if paint {
            ctx.fill_rect(0.0, 0.0, 40.0, 10.0);
        }
        let lists = ctx.take_lists();
        let pointers: Vec<_> = lists.iter().map(|l| l.as_ptr()).collect();
        let lengths: Vec<_> = lists.iter().map(|l| l.len()).collect();
        // SAFETY: a live canvas and surface, and lists that outlive the call.
        let result = unsafe {
            ecg_canvas_replay(
                canvas,
                surface.0,
                std::ptr::null_mut(),
                pointers.as_ptr(),
                lengths.as_ptr(),
                lists.len(),
                std::ptr::null(),
            )
        };
        assert_eq!(result, 0);
    }
    let px = make::pixels(surface, w, h);
    let alpha: Vec<u8> = (0..w as usize)
        .map(|x| px[(5 * w as usize + x) * 4 + 3])
        .collect();
    let painted: Vec<usize> = (0..w as usize).filter(|&x| alpha[x] == 255).collect();
    assert_eq!(
        painted,
        (10..20).collect::<Vec<_>>(),
        "alpha by column: {alpha:?}"
    );
    // SAFETY: no replay uses the canvas or surface after this point.
    unsafe { ecg_canvas_free(canvas) };
    make::release(surface);
}
