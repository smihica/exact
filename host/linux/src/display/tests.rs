use super::*;

#[test]
fn pending_animation_does_not_spin_but_real_timer_deadlines_remain() {
    assert_eq!(work_timeout(true, true, None, 100.), -1);
    assert_eq!(work_timeout(true, true, Some(250.), 100.), 150);
    assert_eq!(work_timeout(true, true, Some(250.), 250.), 0);
    assert_eq!(work_timeout(false, true, None, 100.), 0);
    assert_eq!(work_timeout(false, false, Some(250.), 100.), 150);
    assert_eq!(work_timeout(false, false, None, 100.), -1);
}

#[test]
fn poll_distinguishes_input_readiness_drm_readiness_and_invalid_fd() {
    use std::io::Write;
    use std::os::fd::AsRawFd;
    use std::os::unix::net::UnixStream;
    let (input, mut input_signal) = UnixStream::pair().unwrap();
    let (drm, mut drm_signal) = UnixStream::pair().unwrap();
    let fds = [input.as_raw_fd(), drm.as_raw_fd()];
    assert!(!poll(&fds, drm.as_raw_fd(), 0).unwrap());
    input_signal.write_all(&[1]).unwrap();
    assert!(
        !poll(&fds, drm.as_raw_fd(), 0).unwrap(),
        "input wake can make progress without completing a flip"
    );
    drm_signal.write_all(&[1]).unwrap();
    assert!(poll(&fds, drm.as_raw_fd(), 0).unwrap());
    assert!(poll(&[i32::MAX], i32::MAX, 0).is_err());
    assert!(
        poll(&[i32::MAX, input.as_raw_fd()], input.as_raw_fd(), 0).is_ok(),
        "other FD lifetime policy is unchanged"
    );
}

#[test]
fn copy_xrgb_fixed_nonopaque_and_transparent_pixels_force_x() {
    let frame = Pixmap::from_vec(
        vec![1, 7, 99, 100, 0, 0, 0, 0, 21, 42, 63, 255],
        tiny_skia::IntSize::from_wh(3, 1).unwrap(),
    )
    .unwrap();
    let before = frame.data().to_vec();
    let mut dst = [165; 16];
    copy_xrgb(&frame, &mut dst[1..14], 13, 3, 1);
    assert_eq!(
        dst,
        [165, 99, 7, 1, 255, 0, 0, 0, 255, 63, 42, 21, 255, 165, 165, 165]
    );
    assert_eq!(frame.data(), before.as_slice());
}

#[test]
fn copy_xrgb_fixed_crop_source_stride_and_odd_destination_pitch() {
    let frame = Pixmap::from_vec(
        (1u8..=60).collect(),
        tiny_skia::IntSize::from_wh(5, 3).unwrap(),
    )
    .unwrap();
    let before = frame.data().to_vec();
    let mut dst = [165; 54];
    let mut expected = [165; 54];
    expected[1..13].copy_from_slice(&[3, 2, 1, 255, 7, 6, 5, 255, 11, 10, 9, 255]);
    expected[18..30].copy_from_slice(&[23, 22, 21, 255, 27, 26, 25, 255, 31, 30, 29, 255]);
    copy_xrgb(&frame, &mut dst[1..52], 17, 3, 2);
    assert_eq!(
        dst, expected,
        "prefix, row padding and uncopied bottom/suffix stay intact"
    );
    assert_eq!(frame.data(), before.as_slice());
}

#[test]
fn copy_xrgb_fixed_seventeen_distinct_pixels_preserve_tail_and_guards() {
    let frame = Pixmap::from_vec(
        (1u8..=68).collect(),
        tiny_skia::IntSize::from_wh(17, 1).unwrap(),
    )
    .unwrap();
    let before = frame.data().to_vec();
    let mut dst = [165; 74];
    let mut expected = [165; 74];
    expected[1..69].copy_from_slice(&[
        3, 2, 1, 255, 7, 6, 5, 255, 11, 10, 9, 255, 15, 14, 13, 255, 19, 18, 17, 255, 23, 22, 21,
        255, 27, 26, 25, 255, 31, 30, 29, 255, 35, 34, 33, 255, 39, 38, 37, 255, 43, 42, 41, 255,
        47, 46, 45, 255, 51, 50, 49, 255, 55, 54, 53, 255, 59, 58, 57, 255, 63, 62, 61, 255, 67,
        66, 65, 255,
    ]);
    copy_xrgb(&frame, &mut dst[1..72], 71, 17, 1);
    assert_eq!(dst, expected);
    assert_eq!(frame.data(), before.as_slice());
}

#[test]
fn copy_xrgb_fixed_min_dimensions_and_zero_extents() {
    let frame =
        Pixmap::from_vec(vec![3, 5, 7, 8], tiny_skia::IntSize::from_wh(1, 1).unwrap()).unwrap();
    for (width, height) in [(0, 1), (1, 0), (0, 0)] {
        let mut dst = [165; 16];
        copy_xrgb(&frame, &mut dst[1..15], 9, width, height);
        assert_eq!(dst, [165; 16]);
    }
    let mut dst = [165; 16];
    copy_xrgb(&frame, &mut dst[1..15], 9, 3, 4);
    assert_eq!(
        dst,
        [165, 7, 5, 3, 255, 165, 165, 165, 165, 165, 165, 165, 165, 165, 165, 165]
    );
    assert_eq!(frame.data(), [3, 5, 7, 8]);
}

#[test]
fn copy_xrgb_fixed_in_bounds_overlapping_pitch_keeps_row_order() {
    let frame = Pixmap::from_vec(
        (1u8..=16).collect(),
        tiny_skia::IntSize::from_wh(2, 2).unwrap(),
    )
    .unwrap();
    let before = frame.data().to_vec();
    let mut dst = [165; 14];
    copy_xrgb(&frame, &mut dst[1..13], 3, 2, 2);
    assert_eq!(
        dst,
        [165, 3, 2, 1, 11, 10, 9, 255, 15, 14, 13, 255, 165, 165]
    );
    assert_eq!(frame.data(), before.as_slice());
}
