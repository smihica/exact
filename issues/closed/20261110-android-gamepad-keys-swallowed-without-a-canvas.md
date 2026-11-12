# On Android the reader consumes every game controller event, so a controller cannot go Back in any app, or drive an app on a TV

**Status:** Closed
**Resolution:** Android observes controller state but consumes buttons and joystick motion only while a canvas receives pad input, leaving fallback navigation keys unhandled.
**Systems:** host/android/canvas/src/main/java/dev/exact/canvas/Pads.kt, host/android/canvas/src/main/java/dev/exact/canvas/ExactActivity.kt
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

At `dd6f5dcd`. `ExactActivity.dispatchKeyEvent` and `dispatchGenericMotionEvent` (`ExactActivity.kt:63` to `:65`) offer every event to `Pads` first and stop when it returns true. `Pads.key` (`Pads.kt:90`) returns true for every button of a device whose sources include `SOURCE_GAMEPAD` or `SOURCE_JOYSTICK`, the D-pad's four keys included, and `Pads.motion` (`Pads.kt:101`) returns true for every joystick move. Neither asks whether a canvas takes pad input, so the events are consumed in an app that has no canvas at all.

Android builds a controller's navigation from the events an app leaves unhandled, and asks apps to "avoid marking events as handled (returning true) unless that event is used by gameplay" ([Handle controller actions](https://developer.android.com/develop/ui/views/touch-and-input/game-controllers/controller-input)):

- an unhandled `BUTTON_A` becomes `DPAD_CENTER` and an unhandled `BUTTON_B` becomes `BACK` (AOSP `frameworks/base/data/keyboards/Generic.kcm`: `key BUTTON_B { base: fallback BACK }`);
- an unhandled joystick move becomes D-pad key events (`ViewRootImpl`'s `SyntheticInputStage`), and Android reports a pad's D-pad itself as the axes `AXIS_HAT_X` and `AXIS_HAT_Y`.

Because the reader handles the originals, none of these is made, and `Keys.remote` (`Keys.kt:79`), which turns `DPAD_*` and `DPAD_CENTER` into a TV's arrows and Select, never hears a controller.

**The failure.**

- A phone or tablet with a controller paired, any app: B does not go Back.
- Caltrain on an Android TV driven by a game controller, one of the controllers a TV is used with ([Manage TV controllers](https://developer.android.com/training/tv/start/controllers)): the D-pad and the left stick move nothing, A selects nothing and B does not go Back.

Expected: the controller drives the app as a remote does (LLP 1115.001.003 D3, D4), and only a canvas with the focus hears the pad. A remote's or a keyboard's keys are not affected.

**Verification.** Read only: the lines above, Android's documentation and AOSP's source. Not run: no Android device or emulator was available. The check to make on a device: pair a controller, push a route in any app, press B.

**Direction.** LLP 1133.022 D4 states the rule: the registry observes every pad event and consumes it only while a canvas has the focus. The presenter knows which canvas, if any, has the pads (`pad_target`, `host/linux/src/surface_controls.rs:330`) and can tell the reader, as it tells it when Back has somewhere to go. The D-pad keys Android synthesizes carry the pad's device id and `FLAG_FALLBACK`, so `Pads.key` must let those through too.

**Related.** LLP 1133.022 records the mechanism as unfinished (its stage 2 record: "Pad keys are still consumed whatever holds the focus, as before") and places the cost in "every app with a canvas". It reaches every app, and the sticks and hat as well as the keys. Nothing in `QUEUE.md` or `issues/` tracks it.
