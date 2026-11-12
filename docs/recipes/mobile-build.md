# Recipe: phone release builds

Run in the app directory. `--json` prints `path`, `target`, `trust` and (on Apple)
`signing` on stdout; failures print `code` and `error`. Logs go to stderr.
No `--run`, `--prebuild` or `--test`. Releases omit the agent.

Updating production bakes need `deploy.signing.keys` and either
`EXACT_UPDATE_RECEIPT` (an authenticated publisher receipt) or
`EXACT_UPDATE_GENESIS=1` (a new stream), including simulator and Android builds.
Store `"0"` needs no updates, keys or receipts: set `deploy.store.ios: "0"`
(the first iOS build sets it when `host.ios` is absent).

**iOS Simulator**, no installation or signing team:

```sh
bun exact.mjs ios --release --no-install --json
```

`--sim <udid|name>` selects it; omit `--no-install` to install.

**Signed iPhone:** needs a phone this Mac knows, an Apple Development identity
with its private key, and a profile covering its UDID, bundle id and entitlements:

```sh
DEVELOPMENT_TEAM=<team-id> bun exact.mjs ios --device --phone <udid> --release --no-install --json
```

`DEVELOPMENT_TEAM=<team-id>` selects that team's installed profile and Apple
Development identity. `EXACT_PROFILE=<file.mobileprovision>` and
`EXACT_IDENTITY=<certificate-sha1>` override it. If none of its profiles covers this
bundle id and phone, unset it, or run an Xcode project with that bundle id and
team on that phone with Automatically manage signing, then retry. Connect to install.

**No team or phone:** an ad-hoc signed device `.app`, no installation:

```sh
bun exact.mjs ios --device --release --unsigned --json
```

Needs no profile, identity or icon. `--unsigned --run` refuses: a stock iPhone
cannot launch an ad-hoc signed bundle. Add `--archive .exact/app.ipa` for a service
that re-signs it; only the archive needs an AppIcon (a square icon at least 512 px).

**Standalone Android APK:** set `deploy.store.linux: "0"` in app.json first.
Android uses the Linux bake; an absent Linux store defaults to updates, even in
a new app.

```sh
EXACT_UPDATE_TRUST=production bun exact.mjs android --release --json
```

It installs when an adb device is ready (`ANDROID_SERIAL` selects one); with none,
it only builds. If modules name Maven libraries, use `--write-locks` once and keep
the lock/verification files. Missing locks refuse before Rust compiles and name
the app-local command to write them.

Read JSON's absolute `path`; `--archive` paths are relative to the working directory.
iOS defaults to size (games: speed); `--optimize speed` overrides.

Setup: `bun <exact2>/scripts/exact.mjs setup --check`.
Apple: Mac, Xcode/iOS SDK 27.1+, target Rust std/Hermes; simulator runtime to run.
Android: platform-tools, NDK, JDK 17+ (`JAVA_HOME` or Android Studio's on macOS),
`aarch64-linux-android` Rust std/Hermes. Setup reports the platform and build-tools
Gradle declares and names their `sdkmanager` install commands; run those yourself.
Without an SDK directory, the general report omits these rows; an Android report
still shows them. Install Xcode and Android command-line tools before setup.
