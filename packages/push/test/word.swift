// The branch `subscribePush()` takes for each notification authorization
// (`PushModule.step`, which it switches on), and the state
// `pushPermissionState()` reads for it (`PushModule.word`), compiled with
// Push.swift by drive.mjs and run on this Mac: the simulator cannot be put
// into provisional authorization. Only `notDetermined` asks; every
// authorization read as granted registers without asking, so an embedder's
// provisional one never prompts.
import ExactPackage
import Foundation
import UserNotifications

@main struct Word {
    static func main() {
        var failures = 0
        // `.ephemeral` (4), which the macOS SDK does not name.
        let want: [(UNAuthorizationStatus, String, PushModule.Step, String)] = [
            (.notDetermined, "notDetermined", .ask, "prompt"), (.denied, "denied", .refuse, "denied"),
            (.authorized, "authorized", .register, "granted"), (.provisional, "provisional", .register, "granted"),
            (UNAuthorizationStatus(rawValue: 4)!, "ephemeral", .register, "granted"),
        ]
        for (status, name, step, word) in want {
            let (gotStep, gotWord) = (PushModule.step(status), PushModule.word(status))
            let ok = gotStep == step && gotWord == word
            if !ok { failures += 1 }
            print("\(ok ? "PASS" : "FAIL") \(name): subscribePush() takes \(step.rawValue), pushPermissionState() reads \(word)\(ok ? "" : ": got \(gotStep.rawValue), \(gotWord)")")
        }
        exit(failures == 0 ? 0 : 1)
    }
}
