# On Android a permission request that overlaps another, or is made while the activity is paused, records a permanent denial the person never gave

**Status:** Closed
**Resolution:** Android queues overlapping permission requests and records permanent denial only for permissions with a platform result.
**Systems:** host/android/canvas/src/main/java/dev/exact/canvas/Extensions.kt, packages/*/android
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

At `dd6f5dcd`. `ExtensionContext.requestPermissions` runs its callback without showing a dialog in two cases (`Extensions.kt:589` to `:596`):

```kotlin
fun ask(permissions: Array<String>, done: () -> Unit) {
    val activity = Exact.reader?.get()
    if (activity == null || !foreground) return done()   // the activity is paused: nothing is asked
    waiting += done
    if (asking) return                                    // a request is up: this one's permissions are dropped
    asking = true
    activity.requestPermissions(permissions, CODE)
}
```

Every caller then decides whether the platform will ask again by one test: not granted, and no rationale owed. `ExtensionContext.askPermission` does (`Extensions.kt:326` to `:332`), and so do the packages that call `requestPermissions` themselves (`packages/geolocation/android/Geolocation.kt:128`, `packages/media-capture/android/MediaCapture.kt:355`, and calendar, geofencing and photos):

```kotlin
if (permission(name).state != "granted" && a != null && !a.shouldShowRequestPermissionRationale(name)) {
    …edit().putBoolean(name, true).apply()               // "denied": never asked again
}
```

That test is also true of a permission that was never requested: Android owes a rationale only after a refusal ([Request runtime permissions, "Explain why your app needs the permission"](https://developer.android.com/training/permissions/requesting#explain)). So a request that showed nothing writes the permanent denial, `permission(name).state` is `denied` from then on (`Extensions.kt:274` to `:281`), and each package answers `denied` without asking (`ContactStore.kt:106`, `VideoCapture.kt:100`, `Pedometer.kt:89`). Only a grant in Settings clears it.

**The failure.** Two modules ask in one turn, as two resources first read on one screen do: `@exact/geolocation`'s `getCurrentPosition` and `@exact/pedometer`'s `requestPedometerPermission`.

- observed (by reading): the location dialog shows. The pedometer's request joins `waiting`, and its callback runs when the location dialog closes: `ACTIVITY_RECOGNITION` is not granted and no rationale is owed, so it is recorded as permanently denied. The pedometer's status is `denied` and it never prompts;
- expected: the second dialog after the first (LLP 1133.002 D9: "two modules asked together prompt one after the other"), and nothing recorded for a request that showed no dialog.

A request that arrives while the first dialog is up takes the other branch, since a permission dialog pauses the activity, and records the same. So does one made under a picker or the share sheet.

**Verification.** Read only: the lines above, and AOSP's `Activity.requestPermissions`, which refuses a second request while one is pending ("Can request only one set of permissions at a time", answered with empty results) and says the activity "may be paused and resumed". Not run: no Android device was available. The device check: a fixture that binds both packages and asks both from one handler; afterwards `adb shell run-as <package> cat shared_prefs/exact.permissions.xml` should not name `ACTIVITY_RECOGNITION`.

**Direction.** Queue a request made while one is up and ask it when the first settles, and have `ask` tell its caller whether a dialog was shown, so that `askPermission` and the packages record a denial only after a request the platform presented.

**Related.** `issues/20261110-android-contacts-calendar-status-change-not-announced.md` is about a change made in Settings. `QUEUE.md`, "The camera's open ends" (1), moves two packages' histories onto `ExtensionContext.permission`; it does not describe this.
