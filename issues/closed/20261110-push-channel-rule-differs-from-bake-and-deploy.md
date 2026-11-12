# `exact push` ignores the only `deploy.channels` entry and uses `prod`, where the bake and `deploy` use that channel

**Status:** Closed
**Resolution:** Push and the deployment smoke share deploy's channel selector, including the sole deploy.channels entry.
**Systems:** scripts/push.mjs, scripts/smoke.mjs, scripts/deploy.mjs, contract/cli/src/manifest.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

Which channel a build belongs to is one rule: `deploy.channel`, else the only key of
`deploy.channels`, else `prod` (`scripts/app.schema.json:1083-1086`). Two places implement it:

- `Manifest::channel` (`contract/cli/src/manifest.rs:300-319`), which the bake calls to write
  the channel into the binary (`bake/src/compat.rs:463`);
- `channelOf` (`scripts/deploy.mjs:690-696`), which `deploy` calls (`scripts/deploy.mjs:1312`).

`scripts/push.mjs:200` leaves out the middle step:

```js
const channel = validateChannel(opts.channel ?? app.manifest.deploy?.channel ?? 'prod');
```

`scripts/smoke.mjs:183` reads the channel the same shortened way.

## The failure

An app whose `app.json` names one channel and no `deploy.channel`:

```json
"deploy": {
  "channels": { "dogfood": "/path/to/origin" },
  "signing": { "key": "shelf", "keys": { "shelf": "…" } }
}
```

```
$ bun scripts/push.mjs <app>
exact push: no origin for the channel prod: pass --origin <dir>, or name deploy.channels.prod ("appdrop" needs no server)
```

The binary built from this manifest checks the stream `(dogfood, compatibility id)` and
`deploy` publishes there. Expected: `push` publishes to `dogfood`, "the stream a deploy or a
production build started" (AGENTS.md).

When the manifest also has `app.origin`, `push` does not refuse: `channelOrigin(manifest,
'prod')` falls back to that origin (`scripts/origin.mjs:184-187`), and `streams` looks under
`.exact/prod` there (`scripts/push.mjs:71-93`). With a production build on the machine it
starts a new `prod` stream for that build and publishes to it, a stream no installed binary of
this app reads; with none it refuses with `no stream on the channel prod …`.

## Verification

On the manifest above (the origin a local directory):

- `bun scripts/push.mjs <app>` printed the refusal quoted and exited 1; with `--channel
  dogfood` it went on to look for the signing key.
- `Manifest::channel()` on the same file, in a temporary unit test, returned `("dogfood",
  Some("/path/to/origin"))`.
- `channelOf` from `scripts/deploy.mjs`, evaluated on the same file, returned `dogfood`.

The `app.origin` case was read, not run: it needs a signing key and a production build.

## Direction

Export `channelOf` from one script module and call it in `push.mjs` and `smoke.mjs`, so the
scripts have one copy beside the Rust one.

## Related

`issues/closed/20261102-push-channel-can-be-named-blobs.md` and
`issues/closed/20261102-push-missing-flag-values-use-defaults.md` are about `--channel`'s
value, not the default. No issue names the missing step.
