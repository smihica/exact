# Without the GPU cull, a skinned `ModelLod` entity past its first level draws with a palette nobody computed

**Status:** Closed
**Resolution:** Direct opaque LOD draws skin level zero for the current frame, including the first far frame and later poses.
**Systems:** game/render/src/renderer/draw.rs, game/render/src/skinning.rs, game/render/src/renderer/culling.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`. Two rules about levels of detail disagree on the direct path,
which draws every group without the GPU cull.

`cull_groups` draws level 0 only there, whatever level the entity selected
(`game/render/src/renderer/culling.rs:132-135`), as the README says: "Direct
drawing ... has no per-instance cull and draws level 0 only."

`select_levels` still marks every record of a level that was not selected as
hidden and hands that to the skinning
(`game/render/src/renderer/draw.rs:749-753`), and `Skinning::activate`
dispatches no job for a hidden record (`game/render/src/skinning.rs:593-605`).

So once the entity is past its first level's distance, level 0 is the level
drawn and also the level whose skin is not computed. Its vertices read a
palette that is zero if the entity was never near; by the same code it keeps
the last near pose otherwise.

The direct path is taken on a device without indirect execution, which is the
iOS simulator's Metal (`game/render/src/cull.rs:93`), or when the lists pass
the device's storage limits. The web is WebGPU, which always has indirect
draws (`cull.rs:734-737`). No game in the repository has a skinned entity with
`ModelLod`: Garden's are unskinned.

## The failure

A skinned model with a `Pose`, and
`ModelLod { levels: vec![LodLevel { distance: 2., model: "rock.model".into() }], hide: None }`,
seen from 5 m with `renderer.cull.indirect_execution = false`, against the same
scene without the `ModelLod`.

Observed:

```
direct, no ModelLod: 1 skin jobs, palette zero false, level 0 drawn true
direct, ModelLod past its level: 0 skin jobs, palette zero true, level 0 drawn true
direct frames with and without ModelLod: 205 of 4096 pixels differ
```

The model is gone: a zero palette puts every vertex at one point. Expected:
the two frames are the same, since both draw level 0.

## Verification

Ran that as a test in `game/render` on a Mac (Metal), with the cull forced
direct as `cull_tests.rs` and `lod.rs`'s own test do; it printed the lines
above. Not run on an iOS simulator. `lod.rs`'s
`far_levels_skip_their_skinning_and_direct_draws_keep_level_zero` asserts the
two rules one after the other and never that a drawn level is skinned.

## Direction

When `cull.direct`, activate the jobs of level 0 records and no others
(`batch.level` is what `cull_groups` reads), instead of the selected level's.

## Related

No issue, QUEUE entry or pitfall mentions `ModelLod` on the direct path.
