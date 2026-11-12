# SVG use instances whose ids differ by 256 receive the same scene UID

**Status:** Closed
**Resolution:** An instance's scene key is given to its whole `use` path, step by step, from a hash moved past any key already given, so distinct paths never share one, and it stays below 2^50 so the bits Apple sets for a part's layer cannot collide with another key.
**Systems:** kernel/src/svg/scene.rs, host/apple/Sources/ExactKit/SvgScene.swift
**Severity:** P2
**Author:** Codex (GPT-6 Astra), from the fix lanes of the 2026-11-10 project review
**Date:** 2026-11-11

## What is wrong

At `09cb236d64007f51a668e38926fda6f64dfb6fb5`, `Resolver::uid` folds a use id into bits 32 and above, rotates left by 13, multiplies, then truncates to 53 bits (`kernel/src/svg/scene.rs:871`). For small ids, the use id's bit 8 becomes bit 53 and is discarded. Two distinct use paths can therefore receive exactly the same UID.

Apple's `SvgScene.attach` keys retained layers by that UID (`host/apple/Sources/ExactKit/SvgScene.swift:473`, `:482`). By reading, the second instance reuses the first instance's shape layer, so the two placements cannot both retain their own layer.

## The failure

Create a rectangle with node id 2 and authored id `shape`. Under one SVG, create use nodes with ids 10 and 266, `href="#shape"`, at x=10 and x=40.

Expected: two shape items with different scene UIDs. Observed:

```
use 10 and 266, shape 2: [2427526773555200, 2427526773555200]
```

The collision occurs with only two instances; it is not a probabilistic large-scene case.

## Verification

Ran `CARGO_BUILD_JOBS=5 cargo test -p exact-kernel --test review_findings -- --nocapture`. The temporary test created the real kernel scene, called `svg::scene::resolve` and walked the two instantiated shape items. There were two items, and the assertion that their UIDs differ failed with the number above. The temporary test was removed. The Apple layer reuse consequence was checked by reading; native rendering was not run. Device check: show this scene on an iPhone and verify that both rectangle instances remain visible at x=10 and x=40 after a scene update.

## Direction

Assign stable scene identities to complete use paths without truncating away distinguishing input bits. Resolve collisions before emitting UIDs rather than treating a shortened hash as guaranteed unique.

## Related

`issues/20261110-svg-use-and-text-have-no-bounding-box.md` concerns use geometry, not identity. No matching issue or QUEUE entry was found.
