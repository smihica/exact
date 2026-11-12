# `deviceOrientation(absolute=true)` is honoured only for the web's first reader, is a quarter turn off on Apple, and is ignored on Android

**Status:** Closed
**Resolution:** Each platform keys an absolute `deviceOrientation` reader apart from a relative one and answers it from its own source with `absolute` true: `deviceorientationabsolute` on the web, the x-north attitude turned to y-north on Apple, and `TYPE_ROTATION_VECTOR` on Android.
**Systems:** packages/motion/web/index.js, packages/motion/apple/Motion.swift, packages/motion/android/Motion.kt
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

`deviceOrientation` declares `absolute` (`packages/motion/exact.json`), and LLP 1133.013 D1 and D2
map it to `deviceorientationabsolute` on the web, `xMagneticNorthZVertical` on Apple and the
rotation vector on Android. At `dd6f5dcd` each implementation handles it differently, and none
does what the table says.

- **Web** (`packages/motion/web/index.js`). The page's stream reads `absolute` once, when it is
  made (`:173`), and the listener's event type comes from that (`:130`). A reader group's key
  leaves `absolute` out (`:87`), and `restart` (`:182-188`) returns when the frequency is
  unchanged. So whichever reader made the stream decides for all: a reader that asks
  `absolute=true` after one that did not keeps receiving `deviceorientation`, and a relative
  reader after an absolute one is answered north-referenced.
- **Apple** (`Motion.swift`). An absolute reader restarts device motion in magnetic north and
  turns on Core Motion's calibration display (`:283-285`, `:330`), but the answer is
  `"absolute": false` whatever was asked (`:396`), and the angles are not turned from Core
  Motion's x-north frame to the W3C's y-north as `absoluteOrientationSensor`'s quaternion is
  (`northUp`, `:427`): alpha is 0 when the device's right edge points north, a quarter turn from
  the web's.
- **Android** (`Motion.kt`). `deviceOrientation` always registers `TYPE_GAME_ROTATION_VECTOR`
  (`:66`); `absolute` is not read, and the answer says `absolute` false (`:393`).

**The failure.** On the web, with one reader whose argument changes:

```
resource o = deviceOrientation(absolute=compass)   // compass: false, then true
```

Observed: after `compass` turns true the page still listens to `deviceorientation` and
`o.absolute` is false. Expected: `deviceorientationabsolute` and `o.absolute` true, as the same
call gives on a fresh page.

**Verification.** Web: run in a page of the pinned Chromium (156), the implementation imported
as written and called in the order `host/web-js/modules.js` calls it (the new arguments asked,
then `releaseArgs` for the old). The page dispatched its own `DeviceOrientationEvent`s on both
types, alpha 10 on `deviceorientation` and alpha 200 on `deviceorientationabsolute`:

```
deviceOrientation(absolute=false): alpha 10, absolute false | listening to deviceorientation
the same reader, now absolute=true: alpha 10, absolute false | listening to deviceorientation
absolute=true on a fresh page: alpha 200, absolute true | listening to deviceorientationabsolute
two readers, the absolute one first: the relative one reads absolute true | listening to deviceorientationabsolute
```

Apple and Android: by reading; neither sensor stack runs on this Mac. The check on each phone:
`deviceOrientation(absolute=true)` with the top edge pointing north reads `absolute` true and
alpha near 0.

**Direction.** Web: keep `absolute` on the group and run a stream for each event type its
readers need. Apple: answer `absolute` true under the north frame and turn the attitude to
y-north before taking the angles. Android: register `TYPE_ROTATION_VECTOR` for an absolute
reader, keyed apart from a relative one.

**Related.** `issues/20261110-apple-device-orientation-angles-out-of-range.md` is the angle
ranges of the same answer. LLP 1133.013 §13's as-built notes do not list `absolute` as left
out.
