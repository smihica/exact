# A shadowed point light shows a lit line inside its shadows along each cube-face boundary

**Status:** Closed
**Resolution:** Point-light shadow face selection and projection use the same offset receiver, keeping shadows dark across cube-face boundaries.
**Systems:** game/render/src/shaders/shadow_sample.wgsl
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`. `light_visibility` picks a point light's cube face from the
fragment's own position (`game/render/src/shaders/shadow_sample.wgsl:153-160`,
the major axis of `from_light`), then projects a different point into that
face: the fragment moved along its normal by up to one and a half texels and
half a texel towards the light (`:168-171`). Near a face boundary the moved
point belongs to the neighbouring face, so its coordinates fall outside the
chosen face's map, and that case returns fully lit (`:175`):

```wgsl
if any(uv < vec2(0.0)) || any(uv > vec2(1.0)) || ndc.z > 1.0 { return 1.0; }
```

Every receiver within the offset's reach of a boundary, on the side its normal
pushes across, is lit whatever stands between it and the light. The band's
width is `1.5 * sin(theta)` shadow texels: about 0.2% of the distance to the
light at 45 degrees.

The face and the lookup must come from one point. three.js, which the README
follows for `normalBias`, offsets the position along the normal first and
takes both the cube direction and the compared depth from the result.

## The failure

A `PointLight` with `LightShadows` 2 m over a floor, and a 3 m plate halfway
between them over x 0.5 to 3.5 m, whose shadow covers the floor from x = 1 m to
7 m. The light's -Y and +X faces meet on the floor along x = 2 m, inside that
shadow. An orthographic camera looks straight down at 18 cm of floor centred on
(2, 0, 0), 320 pixels wide.

Observed, the middle row's red channel:

```
without the plate the row is 101 to 107; under its shadow 22 to 104
pixels over the shadow's level by more than 20: 11 of 320, columns 149 to 159
```

Eleven columns, 6 mm of floor just inside x = 2 m, are as bright as the
unshadowed floor. Expected: the whole row at the shadow's level, 22.

## Verification

Ran that scene as a test in `game/render` on a Mac (Metal); it printed the
lines above. The band's width matches the offset: `1.5 * sin(45 deg)` texels of
`2.83 m * 2 / 1024` is 5.9 mm. With the face chosen from
`world + offset - light`, the row under the plate was 22 in every column.
Spot lights have one face and no boundary inside their cone; not run for them.

## Direction

Choose the face from the offset point (`world + offset - light`), so the point
projected is inside the face it was chosen for.

## Related

`tests/lights.rs` covers a point light's two walls on +X and +Z, away from any
boundary. The README's account of local shadows records no limit at the
boundaries. No issue or QUEUE entry mentions the faces' seams.
