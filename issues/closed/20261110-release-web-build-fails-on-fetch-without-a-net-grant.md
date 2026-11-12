# A release web build fails when `app.ts` calls `fetch` and the app holds no `net.fetch` grant

**Status:** Closed
**Resolution:** Release web builds resolve the generated fetch module for sources without a network grant, preserving asset access and named refusals.
**Systems:** host/web-js/build.mjs, js/bake/src/typescript.mjs (replayCaptured)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at `fe4632ed`. For a production build of an app with a TypeScript source and no `net.fetch` grant, the build sets `fetchRefused` (`host/web-js/build.mjs:593`), writes `ts-fetch-refused.js` into the generated directory (`:641`) and makes the source's `fetch` resolve to it (the transform's `inject`, `:754`). The bundle's imports are then replayed from the checked capture, and `replayCaptured` is told of one generated module it may resolve, `ts-fetch.js` (`:761`). The import of `ts-fetch-refused.js` is refused, so the build fails whenever the source names `fetch`.

## The failure

In a new app (`bun scripts/exact.mjs new ~/fetchsite`), keep `export const grants = '';` in `app.ts` and make the source call `fetch`:

```ts
greeting: async ([name]): Promise<Result<'greeting'>> => {
  const r = await fetch('/assets/a.txt');
  return { text: `Hello from ${name}. ${r.status}` };
},
```

```sh
EXACT_APP_DIR=~/fetchsite bun <exact2>/host/web/build.mjs fetchsite --out dist
```

Observed (exit 1):

```
error: app.ts imports '…/dist/.gen/ts-fetch-refused.js', which the checked capture did not resolve; build again
fetchsite: the web build (the JS target) failed; the wasm target is internal (--wasm)
```

Building again fails the same way. Expected: the build succeeds, as the development build of the same app does, and the page has the `fetch` the build wrote for an app with no grant.

## Verification

Ran the commands above at `c047f991`: the output shown. At `dd6f5dcd` the same app built in development (no `--out`), in release with no `fetch` in the source, and in release with a `net.fetch` grant. `host/web-js/empty-admission.test.mjs` covers a source that never names `fetch`.

## Direction

Give `replayCaptured` the refused module's paths as it is given `ts-fetch.js`'s, and add the case of a source that names `fetch` to `empty-admission.test.mjs`.

## Related

None found in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md`.
