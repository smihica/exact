# The iOS hello app is twice its binary budget

**Status:** Open
**Systems:** host/apple (iOS binary size), scripts/metrics.mjs
**Severity:** P2
**Author:** Claude (Opus 5.5), async lane triage
**Date:** 2026-11-13

`bun scripts/metrics.mjs --long` measures the hello app's stripped iOS simulator executable over an empty UIKit app's (LLP 1047.001 D8) and gates it at `IOS_HELLO_BYTES`. Both rows are VIOLATIONs at 69668888a on the async lane's Mac (Xcode 27.2):

| | measured | budget |
|---|---|---|
| optimized for speed | 18,789,856 B (17.92 MB) | 8,600,000 B |
| optimized for size (the iOS app and embed default) | 12,319,936 B (11.75 MB) | 6,050,000 B |

The lane's run at afde7b25e measured 18,443,344 B and 12,103,872 B, so the binary grew by some 200–350 KB across those 545 commits as well.

## How to reproduce

```
bun scripts/metrics.mjs --long
```

The two `iOS: hello app` rows. The run takes hours on a loaded machine; the rows' build is `metrics.mjs`'s `IOS_HELLO_BYTES` section.
