# A page rendered under Bun is always `dir="ltr"`, and reads the strings table the manifest's `lang` names instead of the base

**Status:** Closed
**Resolution:** A render under Bun has no launch locale and reads the base table, and its head writes the resolved table's `lang` and `dir` (the manifest's `lang` and `ltr` for a plan with no table), as the native render host does.
**Systems:** host/web-js/render.mjs, host/web-js/dom.js, host/web-js/rt.js
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

LLP 1060 D4 says a render has no launch locale and starts at the base table ("Without a launch locale, including a bake, it starts at the base"), and that the resolved table "supplies HTML `lang` and `dir` on the web document element and rendered pages". The native render host does both: it writes `lang` and `dir` from the runner's resolved table (`host/render/src/direct.rs:127-128`, `host/render/src/page.rs:41-47`, `:335-352`), and keeps the manifest's `lang` only for a plan with no table (`page.rs:295-314`, LLP 1115.000 D4.6).

The JavaScript render server takes both from the manifest instead:

- It composes every head from the shell: the language is the one the build wrote from the manifest's `lang` (`en` when absent), and the direction is the literal `ltr` (`host/web-js/render.mjs:113`, `:119`). The runtime's own answer never reaches it: `language` sets `document.documentElement.lang` and `.dir` (`host/web-js/rt.js:1481`), and in a render that element is `dom.js`'s body, on which the two are plain properties nothing reads (`host/web-js/dom.js:153`).
- It gives the render that language as `navigator.language` (`render.mjs:154`), which the runtime reads as the reader's locale and resolves to a table (`rt.js:1452-1461`).

Pages render under Bun for any app with no native render entry (`host/web-js/build.mjs:686`), and per request under `serve`.

## The failure

`strings/ar.json` holds `{ "hello": "مرحبا" }`, `strings/en.json` `{ "hello": "Hello" }`, and

```
routes nav
  home "/" render=build

component App
  view
    column
      text t("hello") testId="hello"
```

| `app.json` | tables | `bun host/web-js/build.mjs <app> --render js` | the native render entry, same plan and shell |
|---|---|---|---|
| `"strings": { "base": "ar" }` | `ar` | `<html lang="en" dir="ltr">`, مرحبا | `<html lang="ar" dir="rtl">`, مرحبا |
| `"strings": { "base": "ar" }` | `ar`, `en` | `<html lang="en" dir="ltr">`, Hello | `<html lang="ar" dir="rtl">`, مرحبا |
| `"lang": "ar"` | `en` (the base), `ar` | `<html lang="ar" dir="ltr">`, مرحبا | `<html lang="en" dir="ltr">`, Hello |

Expected: the native column.

In Chromium the first app's page is served `dir="ltr"`, `lang="en"`, and turns to `rtl`, `ar` when the runtime boots: its text, aligned to the start, changes sides. On a route declared `activate=interaction`, which waits for the reader, it was still `ltr` 2.5 s later; a crawler or a reader without JavaScript gets the served document.

## Verification

Built each app with `--render js` and ran `target/host-dev/caltrain-render --plan dist/app.plan --shell dist/shell.html --build` over the same plan (the app given Caltrain's id, which that entry requires); the table is a diff of the two documents. The Chromium readings are `document.documentElement.dir` and `.lang`, and the text's computed `direction` and `text-align`, as the page loaded and 2.5 s later.

Not run: `render.mjs --serve`, which composes its heads with the same `headOf`.

## Direction

Give the render no reader's locale, so it reads the base. Have it report the resolved table's name and direction (the server bundle's `__start` can, once `app()` has run) and write them in `headOf`, keeping the shell's `lang` for a plan with no table.

## Related

Nothing in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` names a rendered page's direction or table. Three other differences between the two renderers are filed: `issues/20261110-js-render-server-links-markdown-paths-to-render-invalid.md`, `issues/20261110-document-markdown-external-link-has-no-target.md` and `issues/20261110-render-checkpoint-reads-resource-types-by-position.md`.
