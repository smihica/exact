# The starter's production `app.js` is over its 44 KiB budget

**Status:** Closed
**Resolution:** focus, blur, setScheme, copyText and haptic link into an app only where an action names them, and the starter's budget is 45 KiB (Charlie, 2026-11-11); it ships 44.2 KiB.
**Systems:** host/web-js, host/web/navigation.js, scripts/metrics.mjs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the fix lanes of the 2026-11-10 project review
**Date:** 2026-11-11

## What is wrong

`scripts/metrics.mjs:136` gates the JS target's production `app.js`, Brotli 11, for five apps (`JS_TARGET_KIB`; LLP 1047.002 D5), and the async lane runs `--web-bytes` as a step. `636cf947` (2026-11-10) set the starter's budget to 44 KiB and measured it at 43.5. At `1fb06ccb` it is over.

## The failure

`bun scripts/metrics.mjs --web-bytes` at `1fb06ccb`, in a clean worktree:

```
web app.js: hello          36 KiB   12.8 KiB brotli-11 production; budget 14 KiB, within
web app.js: starter       134 KiB   44.0 KiB brotli-11 production; budget 44 KiB, VIOLATION, 49.7 KiB gzip; entry by module: rt.js 28% · navigation.js 20% · ts-data.js 12% · ts-fetch.js 6% · router.js 6% · symbols.js 5%
```

Expected: within. The app is the one `exact new` writes, so this is what a new app ships.

Caltrain, RealWorld and the video player are also reported over; `636cf947` records that they were over before it and stayed so. This issue is the starter alone.

## Verification

The run above (204 s for the five apps). Between `636cf947` and `1fb06ccb`, 47 commits touch `host/web-js` or `host/web/navigation.js`; most fix a runtime defect and add code to modules the starter links (`rt.js`, `navigation.js`, `ts-data.js`, `ts-fetch.js`). Which of them carried the starter over was not measured.

## Direction

Measure the starter at the commits between the two, find what grew, and take it back out of the starter's closure: link it by use, as `636cf947` did for commands and grant admission, where a plan that never needs it should not carry it. The budget is the owner's number (LLP 1047.002 D5); a fix does not raise it.

## Related

`issues/20261110-line-cap-met-by-packed-lines-and-overflow-files.md`: its split of `navigation.js` changes what the JS target bundles from that file, and should be measured against this number.
