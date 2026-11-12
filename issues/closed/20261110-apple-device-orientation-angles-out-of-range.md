# Apple's `deviceOrientation` answers Core Motion's Euler angles, outside the W3C's ranges

**Status:** Closed
**Resolution:** Apple computes alpha, beta and gamma from the attitude quaternion as Chromium and `Motion.kt` do, so they fall in the W3C ranges, instead of answering Core Motion's yaw, pitch and roll.
**Systems:** packages/motion/apple/Motion.swift
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, `packages/motion/apple/Motion.swift:393-396` answers `deviceOrientation` with
`CMAttitude`'s angles unchanged:

```swift
// Device Orientation's Z-X'-Y'' angles from Core Motion's attitude.
let a = m.attitude
return ["alpha": a.yaw * deg, "beta": a.pitch * deg, "gamma": a.roll * deg, "absolute": false, "apple": NSNull()]
```

The interface is the web's `DeviceOrientationEvent`: alpha in [0, 360), beta in [−180, 180),
gamma in [−90, 90) ([Device Orientation](https://www.w3.org/TR/orientation-event/#deviceorientation);
LLP 1133.013 D1 repeats the ranges and declares no Apple deviation). Core Motion's angles are the
same Z-X′-Y″ rotations with the other choice of ranges: yaw and roll in [−180°, 180°], pitch in
[−90°, 90°]. The two agree only while alpha is under 180 and the screen faces upward (beta within
a quarter turn of level).

Android converts through the rotation matrix (`packages/motion/android/Motion.kt:362-395`, "as
Chromium computes them"), and WebKit does the same from `CMAttitude` for its own event
([WebCoreMotionManager.mm](https://github.com/WebKit/WebKit/blob/main/Source/WebCore/platform/ios/WebCoreMotionManager.mm)).
So one app reads different numbers on an iPhone than on Android or in Safari on the same iPhone.

## The failure

`resource o = deviceOrientation()` on an iPhone:

| Attitude | Android, the web | Apple |
|---|---|---|
| flat, turned a quarter turn right | alpha 270, beta 0, gamma 0 | alpha −90, beta 0, gamma 0 |
| upright, tipped 30° past vertical | alpha 0, beta 120, gamma 0 | alpha −180, beta 60, gamma −180 |
| screen toward the floor, turned and rolled | alpha 90, beta 100, gamma 20 | alpha −90, beta 80, gamma −160 |

A needle drawn as `rotate(alpha)` is unaffected by the first row. Code that compares alpha with
a bearing, or reads beta to tell that the device is tipped past vertical, is wrong on Apple alone.

## Verification

Core Motion's own conversion ran on a Mac (macOS 27); `CMMotionManager` did not, since a Mac and
the simulator have no motion hardware. `CMAttitude` objects were made from quaternions through the
framework's unexported `initWithQuaternion:` and read through the public `yaw`, `pitch`, `roll`
and `quaternion`:

- Over 200,000 random attitudes, yaw spanned −179.999° to 179.999°, pitch −89.797° to 89.814°,
  roll −180.000° to 179.999°, and each attitude equalled Z(yaw)·X′(pitch)·Y″(roll) to 1e-13.
- For attitudes composed from W3C angles, `yaw`, `pitch`, `roll` read as the Apple column above,
  and WebKit's conversion of the same three numbers gave back the W3C angles exactly.

The device check: an iPhone flat on a table, turned a quarter turn clockwise from where the
reader started, reads alpha −90 where 270 is expected.

## Direction

Take the angles from `m.attitude.rotationMatrix` as `Motion.kt`'s `orientation` does, so the two
platforms share one rule.

## Related

QUEUE "Qualify the motion screen-frame browser oracle on hardware" is the screen frame's sign, a
different conversion. `deviceorientation-absolute-ignored` (this review) is the `absolute` option
of the same source; under it Apple's alpha is also a quarter turn from the web's.
