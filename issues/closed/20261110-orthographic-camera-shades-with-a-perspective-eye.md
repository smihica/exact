# Under an orthographic camera, surfaces are shaded as if seen from the camera's position

**Status:** Closed
**Resolution:** Orthographic lighting uses unit parallel view rays, and fog and soft particles measure distances along those rays; perspective behavior is preserved.
**Systems:** game/render/src/shaders/forward.wgsl, game/render/src/shaders/model.wgsl
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`. The primitive and model fragment shaders take the view vector
from the camera's position:

```wgsl
let v = normalize(frame.camera_alpha.xyz - input.world);   // forward.wgsl:82, model.wgsl:64
```

An orthographic camera's rays are parallel: every fragment is seen along the
camera's forward axis, wherever the camera stands on it. With this `v` the sun's
specular lobe, the fill and local lights' lobes, the Fresnel term and the
environment's reflection are those of a perspective eye at the camera's
position. The frame already carries the right vector, and the toon rim uses it
ten lines later (`model.wgsl:74`), as the water's fog does (`frame.wgsl:91`):

```wgsl
let rim_view=select(v,-frame.view_depth.xyz,frame.toon.w>0.5);
```

No game in the repository lights meshes under an orthographic camera; the two
fixtures that use one draw sprites and particles. The agent's plan view,
`screenshot <png> world from top <height>`, is an orthographic camera over a
game's lit scene (`game/render/src/look.rs:138-145`), so it shades this way.

## The failure

A 400 m plane with `Material { color: [0.5, 0.5, 0.5, 1.], roughness, .. }`,
`Camera::orthographic(10.)` 6 m straight above it, a `DirectionalLight` at 45
degrees without shadows, a one-colour sky, no fog or bloom,
`ToneMapping::None`. Every pixel sees the floor from the same direction with
the same normal and light, so the floor is one colour.

Observed, the middle row at x = -6 m, 0 and +6 m on the floor (8-bit):

```
roughness 0.3: 255, 166, 165
roughness 0.5: 203, 168, 165   (the default roughness)
roughness 0.8: 170, 168, 167
roughness 1.0: 167, 166, 167
```

A highlight sits where an eye at the camera's position would see the sun
mirrored. Expected: equal values across the row. three.js takes
`isOrthographic ? vec3(0, 0, 1) : normalize(vViewPosition)` for the same vector.

## Verification

Ran that scene as a test in `game/render` on a Mac (Metal); it printed the rows
above. With `v` taken as `-frame.view_depth.xyz` for an orthographic frame in
`forward.wgsl`, the rows were 166, 168, 168 and 166 in all three columns.

The exponential fog starts at the camera's position too unless the frame has
water (`frame.wgsl:116`); with the default fog over a rough floor from 30 m it
moved one level across 60 m (`161, 162, 161`), so the visible defect is the
lobes. The soft particles' gap (`soft.wgsl:16-17`) was not measured.

## Direction

Take `v` as `-frame.view_depth.xyz` when `frame.toon.w > 0.5` in both shaders,
as the rim does.

## Related

No issue, QUEUE entry or pitfall covers shading under an orthographic camera,
and the README records no limit for it. `cull_tests.rs` renders an orthographic
field but compares culled against unculled frames, which share the shader.
