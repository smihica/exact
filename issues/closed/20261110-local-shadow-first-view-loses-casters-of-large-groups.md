# A shadowed local light's first shadow layer draws the wrong casters from a draw group of 961 instances or more

**Status:** Closed
**Resolution:** Local shadow culls keep every visible caster in large draw groups without camera depth sorting.
**Systems:** game/render/src/renderer/local.rs, game/render/src/cull.rs, game/render/src/shaders/cull.wgsl
**Severity:** P1
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`. Each local shadow cull reuses the camera cull's setup words.
`local_setups` (`game/render/src/renderer/local.rs:108-124`) calls `begin_setup`,
which zeroes `sorted` (`game/render/src/cull.rs:384`), copies the words and
rewrites only word 7 of each group, the view flags. Word 13 stays as
`write_cull_setup` wrote it (`game/render/src/renderer/culling.rs:187-192`): a
histogram region for every group of 16 chunks or more (961 instances).

So the local cull's `scan` sorts those groups (`cull.wgsl:361-411`), but
`finish_setup` sized `scratch` with `sorted == 0` (`cull.rs:435-441`). Unless
the buffer's power-of-two capacity leaves 16,384 spare words a group, the
sort's histogram accesses are out of bounds. View 0 reads its chunk counts and
offsets through the order the sort should have written, so its list and
indirect count are wrong. Views 1 to 3 go by chunk index and are right.

View 0 of a local cull is a real layer: the frame's first (a spot light, or a
point light's +X face) and its fifth.

Forest meets this by default: 2,000 trees as instances of one model
(`game/games/forest/logic/src/lib.rs:36`), a shadowed campfire light
(`camp.rs:151`) and a shadowed flashlight (`player.rs:150`).

## The failure

```rust
w.spawn((Transform::default(), Mesh::plane(60., 60.), Material::rgb(0.6, 0.6, 0.6)));
for i in 0..1100 {
    let (x, z) = ((i % 40) as f32, (i / 40) as f32);
    w.spawn((Transform::at(x * 0.6 - 11.7, 1., z * 0.6 - 8.), Mesh::cube(0.3), Material::rgb(0.8, 0.3, 0.2)));
}
for _ in 0..2 {
    let spot = w.spawn((Transform::at(0., 6., 0.).looking_at(Vec3::ZERO, Vec3::Z),
        SpotLight { intensity: 40_000., range: 14., inner: 0.5, outer: 0.6, ..Default::default() }));
    w.insert(spot, LightShadows);
}
```

With a default `Camera` at (0, 9, 9) looking at the origin and no fog or bloom,
at 320 by 180. The two lights give one local cull two identical views.
Instances each view keeps, and pixels that differ from the frame drawn with
`cull.indirect_execution = false`:

```
1100 cubes: view 0 keeps 18, view 1 keeps 144; 987 and 1,024 pixels differ (two runs)
 900 cubes: view 0 keeps 144, view 1 keeps 144; 0 pixels differ
```

Expected: 144 in both views and no differing pixel (README: "Culling never
changes pixels").

## Verification

Ran both cases as a test in `game/render` on a Mac (Metal). Writing `UNSORTED`
into word 13 in `local_setups` gave 144 and 144 and no differing pixel.

A scene built from Forest's pine, tree placement, night lights and camera
(2,000 trees, 640 by 360) drew 32 instances in view 0 where 16 to 38 are
visible. Up to 2,539 pixels differed from the direct frame, 378 by more than 16
levels, and with nothing moving 9 of 40 successive frames changed by about
2,930 pixels (at most 4 with the `UNSORTED` write). A player sees flashlight or
campfire light pass through trees, in patches that blink.

Not run: Forest itself, Vulkan, WebGPU.

## Direction

A local cull's views are all shadow views and need no depth order: write
`UNSORTED` into word 13 of every group in `local_setups`, beside the flag
rewrite. `cull_tests.rs` has no shadowed local light over a group this large.

## Related

No issue or QUEUE entry names local shadow culls or `sort_chunks`. QUEUE's
"Castle Town intermittent absent shadows on launch" concerns sun cascades.
