# On Android a package's call of an app handler always answers `InvalidStateError`, so no Google Pay payment can be authorized

**Status:** Closed
**Resolution:** Android extension handler calls run on the host's owner thread and publish the handler's changes.
**Systems:** host/android/canvas/src/main/java/dev/exact/canvas/Extensions.kt, host/android/canvas/src/main/java/dev/exact/canvas/Owner.kt, packages/payments/android
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

At `dd6f5dcd`. The host lives on one thread: `canvas_jni!` keeps it in a `thread_local!` set by `Native.start` (`host/linux/src/canvas/jni.rs:73` to `:86`, `:169`), and every JNI method that needs it goes through `with`, which answers `None` on any other thread. The reader starts the host on a thread of its own (`Owner.boot`, `Owner.kt:190` to `:194`) unless the launch says `owner=0`, a benchmark's switch (`ExactActivity.kt:229` to `:235`). The main thread reaches it through `Exact`'s wrappers, which post to that thread (`Owner.kt:522` to `:699`): `Exact.answer`, `Exact.changed`, `Exact.picture` and the rest.

`ExtensionContext.invoke` (LLP 1133.016 D15 item 1: a package runs one of the app's handlers) has no wrapper. It calls the JNI method on the main thread, where every extension call runs (`Extensions.kt:299` to `:302`):

```kotlin
val id = try { Native.invoke(body.toString()) } catch (e: UnsatisfiedLinkError) { -1 }
if (id < 0) return reply(JSONObject().put("error", "InvalidStateError").put("message", "no session runs"))
```

and the method is (`host/linux/src/canvas/jni_extensions.rs:287` to `:294`):

```rust
let body = string(env, body);
with(|h| h.module_invoke(&body)).unwrap_or(-1)
```

With the host on its own thread `Native.invoke` is always -1 and the runner never hears the call.

**The failure.** `@exact/payments` is the one package that calls back (`packages/payments/android/Payments.kt:203` to `:211`, `ask`, reached from `authorized` and `changed`, both posted to the main thread). With Google Pay's sheet up:

- observed (by reading): `invoke` replies `{"error":"InvalidStateError","message":"no session runs"}` at once, which `ask` takes as no answer. `onpaymentauthorized` never runs: the result is `fail` with `authorize-timeout`, and the sheet is told the payment did not go through (`Payments.kt:275` to `:287`). An address or a shipping option chosen in the sheet is answered "unavailable" (`:237`);
- expected: the app's handler runs and its answer reaches the sheet, as on Apple and the web.

No app in the repository binds payments on Android, and the sheet has not been driven there (LLP 1133.034 §12), where this would have shown. Any later package that calls the app back meets the same.

**Verification.** Read only: the lines above. `Native.invoke` is the one call `Extensions.kt` makes directly whose Rust needs `with`; `underAgent`, `appFile`, `now`, `surfaceHas`, `sensorInput` and `invoked` read process statics. Not run: the JNI module is `cfg(target_os = "android")` and no Android device was available. The device check: in a default launch, an extension that calls `context.invoke` from its `later` is answered `InvalidStateError`, "no session runs".

**Direction.** Add `Exact.invoke(body)`, which runs `Native.invoke` through `owner.sync` (it returns the call's number and publishes what the handler changed) and directly where there is no owner.

**Related.** `QUEUE.md`, "Android's package context has no `invoke`" (2026-10-24), predates the method, which LLP 1133.034 §12 records as built on 2026-10-27. The entry is stale and should go with this fix. `QUEUE.md`, "Payments: what is owed", item 4, lists the emulator drive as not run.
