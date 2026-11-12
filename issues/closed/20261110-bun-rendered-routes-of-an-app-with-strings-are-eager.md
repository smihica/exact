# Under the Bun renderer every undeclared route of an app with a strings table is `eager`, where the native render infers `interaction`

**Status:** Closed
**Resolution:** e2f7cca3 keeps strings-only Bun pages waiting for interaction and fresh request pages reusable; host/web-js/render-language.test.mjs now guards activation, module preloads, request reuse, and random draws.
**Systems:** host/web-js/render.mjs, host/web/navigation.js (reportPlace)
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at `fe4632ed`. An undeclared route's activation is inferred from what its render left to do (LLP 1048.007 D1). The Bun renderer's `infer` (`host/web-js/render.mjs:50-55`) answers `interaction` only when, among other things, the render formatted nothing in the default time zone (`!out.zoned`). It learns that from a proxy on `Intl.DateTimeFormat` that sets a flag whenever a formatter is made with no `timeZone` (`render.mjs:33-39`).

An app with a strings table asks for its language while it renders. `language()` calls `locale()`, which calls `reportPlace()` (`host/web/navigation.js:1117`), which makes an `Intl.DateTimeFormat` with no zone to read the place's defaults. No date is formatted, but the proxy counts the call, so `out.zoned` is true and every such page is `eager`. The native render of the same plan infers `interaction` (`exact_runner::activation`).

The same call draws from `crypto.getRandomValues` for its seed, which sets `read.random` (`render.mjs:136`). A `request` page is kept only when `!read.random` (`render.mjs:233`), so by reading such a page is never kept either.

## The failure

An app with one strings table and a route that formats no date, built and rendered both ways. Observed: the Bun renderer's page carries `data-activate="eager"` and a `modulepreload` for the app's module; the native page carries `data-activate="interaction"`. Expected: the two agree, and a page that formats no date waits for the reader.

Twelve apps under `apps/` have a strings table.

## Verification

At `dd6f5dcd`, built an app with one table and compared the two pages' `data-activate`. A temporary log in `infer` printed `zoned: true` as the only condition that failed. Not run: the kept `request` page (read only), and nothing again at `fe4632ed`, where the cited lines were read.

## Direction

Have `reportPlace` read the place's defaults without passing through the counted constructor (the proxy's realm can expose the original for that read), or count a formatter only when it formats. Draw the place's seed outside the render's measured reads.

## Related

None found in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md`.
