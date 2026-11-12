# On Apple, a failed `takePhoto` capture replies with a Swift struct, so its error name never reaches the app

**Status:** Closed
**Resolution:** A failed capture replies with the failure's name, `{"error": failure.code}`, and `packages/camera/test/photo-failure.swift` checks that reply is the JSON object `{"error":"InvalidStateError"}`.
**Systems:** packages/camera/apple/Camera.swift
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, `PhotoOutput.capture` finishes with `Result<Data, PhotoFailure>`, where `PhotoFailure` is a struct holding the error's name (`packages/camera/apple/Camera.swift:402`). The module's failure arm binds that struct and puts it in the reply:

```swift
case .failure(let code):        // Camera.swift:236
    holder.leave()
    reply(["error": code])      // code is a PhotoFailure, not its String
```

The reply goes unchanged to the host's `ExactReply.send` (`host/apple/compose.mjs:34`), which sends only what `JSONSerialization.isValidJSONObject` accepts and otherwise fails the call with "the reply was not JSON" (`host/apple/modules/ExactNativeModule.swift:210` to `217`). A dictionary holding a Swift struct is not a JSON object, so every capture failure is that host failure, and the names the module chose are lost: `InvalidStateError` when the photo output's connection is not active (line 396) and `UnknownError` when AVFoundation reports an error or gives no file data (lines 420, 421, 426).

The other failure replies in the same function pass a string (`reply(["error": "InvalidStateError"])`, line 207; `"UnknownError"`, lines 224, 225, 241, 246) and arrive as declared.

## The failure

`takePhoto(track=t)` on iOS or macOS, outside the agent, when the capture's photo connection is inactive, or when `photoOutput(_:didFinishProcessingPhoto:error:)` carries an error.

Observed: the call fails with the host's "the reply was not JSON". Expected: the answer `{"error":"InvalidStateError"}` (or `UnknownError`), the names LLP 1133.008 D1 gives `takePhoto`, as the web implementation answers (`packages/camera/web/index.js:200`).

## Verification

Ran the two sites in Swift, with `PhotoFailure` and the failure arm's binding as the file has them and the test `ExactReply.send` applies:

```swift
struct PhotoFailure: Error { let code: String; init(_ code: String) { self.code = code } }
func reply(_ value: [String: Any]) -> Bool { JSONSerialization.isValidJSONObject(value) }
let result: Result<Data, PhotoFailure> = .failure(PhotoFailure("InvalidStateError"))
if case .failure(let code) = result {
    print(reply(["error": code]))       // false: fail("the reply was not JSON")
    print(reply(["error": code.code]))  // true:  {"error":"InvalidStateError"}
}
```

It printed `false`, then `true`. Not run through a real capture: under the agent `takePhoto` copies the agent's picture and returns before `PhotoOutput` is reached (lines 223 to 228), which is why no drive shows it.

## Direction

Reply with `code.code`.

## Related

QUEUE "The camera's open ends" lists unbuilt cells and device checks, not this. No issue names it.
