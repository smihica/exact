# The web JS target draws the browser's randomness under the driver, not the seeded stream

**Status:** Closed
**Resolution:** The web JS target draws the launch seed's stream under the driver (host/web-js/ts-fetch.js seeded; the page's entry loads the stream before the first render, under the driver only, so the boot graph is unchanged). host/web-js/seeded-entropy.test.mjs holds seed 1's first id to the native one. A web drive against a server that remembers a client now takes --seed, as a native one does.
**Systems:** host/web-js/ts-fetch.js, host/web/storage-environment.js, scripts/agent.mjs
**Author:** Claude (Fable 5.2), from expo/exact#54's thread
**Date:** 2026-11-10

## What is wrong

LLP 1069.005 D2b: under the driver `crypto.getRandomValues` and `crypto.randomUUID`
"draw from a deterministic generator seeded by the launch seed (`--seed`, default 1). The
native executor selects it at creation; the web realm's wrapper does the same." Two runs of
a drive then mint the same ids.

Native hosts do (`js/src/door.rs`, op 11; seed 1's first UUID is
`deb201fb-035c-4c32-bbbf-3da08991a485`). The web's wasm host does
(`host/web/module-glue.js`, `agentSeed` and `agentStream` from
`host/web/storage-environment.js`). The web JS target does not: its bindings call the
browser's (`host/web-js/ts-fetch.js`, the `crypto` entry: `globalThis.crypto.getRandomValues(a)`,
`globalThis.crypto.randomUUID()`), under the driver as outside it.

So one test draws a new id at every launch on the web and the same id at every launch on
macOS. expo/exact#54's reporter met it from the other side: a sync client's session id
repeated on native hosts and their server answered the second launch's writes as repeats,
while the web passed.

## To see it

An app whose source answers `crypto.randomUUID()`; `bun exact.mjs agent web "state id"` twice,
then `agent macos "state id"` twice. The web's two differ; macOS's two are
`deb201fb-035c-4c32-bbbf-3da08991a485`.

## What a fix is

`ts-fetch.js`'s `crypto` draws from `agentStream(agentSeed(), 'typescript')` when the page is
the driver's (a loopback page with `?agent`; `agentSeed` already answers `null` otherwise).
The stream must be there at the first draw, which an answer at boot may make, so it cannot
wait on a dynamic import; and the boot graph is counted (`scripts/boot.mjs`).

It changes what every web drive draws. A drive against a server that remembers a client (a
sync session, a device id) then repeats that client at each launch, as native drives do, and
takes a `--seed` for a new one. Native hosts now say the seeded draw once in `logs`.
