# On Android TV, and on Android 10 and 11, a date or time control's Material picker reads dynamic colours the device does not have

**Status:** Closed
**Resolution:** Material pickers use the baseline scheme when dynamic colours are unavailable and preserve authored accent tones without API 31 calls.
**Systems:** host/android/app/src/compose/java/dev/exact/app/Material.kt
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

At `dd6f5dcd`. An app whose plan has a date or time control links Compose Material 3 (`host/android/app/build.gradle.kts:9` to `:12`), and the shell shows its picker on every device (`MainActivity.kt:17`). The picker's theme starts from the device's dynamic scheme with no check (`Material.kt:106` to `:115`):

```kotlin
val device = if (dark) dynamicDarkColorScheme(activity) else dynamicLightColorScheme(activity)
val scheme = if (accent == null) device else {
    val tone = { t: Float -> Color(ColorStateList.valueOf(accent).withLStar(t).defaultColor) }
```

Both functions are `@RequiresApi(Build.VERSION_CODES.S)` and read `android.R.color.system_*` resources (androidx `DynamicTonalPalette.android.kt`); `ColorStateList.withLStar` is API 31 too. The shell supports two kinds of device without those colours, and the reader's own palette handles both (`Palette.kt:31` to `:39`):

```kotlin
Build.VERSION.SDK_INT < 31 -> Legacy.palette()
…
} catch (e: Resources.NotFoundException) { Legacy.palette() }
```

- Android TV, whose images have "resource ids and no colours behind them" (LLP 1115.001.003 D2).
- Android 10 and 11, when `app.json` says `host.android.minimumOS` `"10"` or `"11"`, which makes the shell's `minSdk` 29 or 30 (`app/build.gradle.kts:27` to `:28`, `:48`; 31 when absent). `host/android/README.md:1115` says of these releases that "a call of a later release is behind a `Build.VERSION.SDK_INT` check (lint's `NewApi` finds none unguarded)". That holds for the reader, not for this file of the shell.

**The failure.** `input type="date"`. Press the control.

- on an Android TV image (by reading): composing `Theme` reads a colour that is not there, the `Resources.NotFoundException` that `Palette.read` catches. `Requests.pick` catches it (`Requests.kt:134` to `:141`), logs `picker date: …` and answers null. No picker shows, and the control cannot be set;
- on Android 10 or 11, built with `minimumOS` `"10"` (by reading): the ids name nothing those releases publish, so the read throws as above or returns another resource's colour. With an `accent-color`, a read that returns goes on to `withLStar`, a `NoSuchMethodError`, which `pick`'s `catch (e: Exception)` does not catch;
- expected: Material's picker in the baseline scheme, which is what the page is drawn in there (`host/android/README.md:1130`).

**Verification.** Read only. `dynamicLightColorScheme`'s annotation and its resource reads were checked in androidx's source. Not run: no Android device, TV image or Android 10 image was available, so which of the two outcomes Android 10 gives is not known. The device check: press a date control on a TV image and on an Android 10 image with `adb logcat -s exact`, then again with `accent-color` set.

**Direction.** Choose the scheme as `Palette.read` does: dynamic from API 31 where the read succeeds, else `lightColorScheme()` or `darkColorScheme()`; and make the accent's tones without `withLStar` below API 31.

**Related.** `issues/20261110-date-press-without-picker-runs-checkbox-toggle.md` is the painted hosts' missing picker. Nothing in `QUEUE.md` or `host/android/README.md` names this.
