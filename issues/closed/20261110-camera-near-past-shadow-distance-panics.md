# A camera whose near plane is at or past the sun's shadow distance panics the renderer

**Status:** Closed
**Resolution:** A frame whose shadow distance is at or before the camera's near plane draws with no cascades (`Cascades::reach`), instead of reaching the assert.
**Systems:** game/render/src/shadows.rs, game/render/src/world/scene.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`. `Cascades::new` asserts that the shadow distance exceeds the
camera's near plane (`game/render/src/shadows.rs:26-32`):

```rust
let near = -inverse.project_point3(Vec3::ZERO).z;
let far = (-inverse.project_point3(Vec3::Z).z).min(settings.distance);
assert!(near > 0.0 && far > near, "shadow distance must exceed camera near");
```

Nothing before it checks the pair. `Camera::valid` (`game/engine/src/scene.rs:187`)
accepts any finite `near > 0` with `far > near`, `Shadows` has no validation,
the feed passes the world's `Shadows` resource or its default through unchanged
(`game/render/src/world/scene.rs:589-594`), and `prepare_effects` calls
`Cascades::new` for every frame with a shadowed sun
(`game/render/src/renderer/draw.rs:671-676`). A `fixed_box` takes the same
assert.

Every profile a game is built with sets `panic = "abort"` (`game/Cargo.toml:44`,
`:100`), and `gpu_render` is an `extern "C"` export that catches nothing
(`gpu/src/native.rs:848`). On a native host the GPU module runs in the app's
process, so the app aborts on its first frame; on the web the module traps.

## The failure

Either of these in `Game::setup`, with a `DirectionalLight::default()` (its
shadows are on):

```rust
// An isometric camera 140 m from the scene, its near plane pulled in for depth precision.
w.spawn((Transform::at(0., 100., 100.).looking_at(Vec3::ZERO, Vec3::Y),
    Camera { near: 80., far: 400., ..Camera::orthographic(40.) }));
```

```rust
// The default camera (near 0.1) and a short shadow reach.
w.insert_resource(Shadows { distance: 0.05, ..Shadows::default() });
```

Observed: the first frame panics at `render/src/shadows.rs:29:9: shadow distance
must exceed camera near`. Expected: the frame draws with no sun shadow (nothing
the camera sees is within the shadow distance), or the surface reports a named
error as the feed does for other refused values. `Shadows::distance` is
documented only as "Camera view depth reached by shadows, in metres (default
60)".

## Verification

Ran three cases as tests in `game/render` on a Mac, through `WorldSurface` and
the components above. A camera with `near: 50.` draws; `near: 80.` with the
default `Shadows`, and `distance: 0.05` with the default camera, both panic
with the message above. Not run in a built app: the abort is from the profiles
and the export.

## Direction

In `prepare_effects`, treat a shadow distance at or under the camera's near
plane as no cascades: the `else` branch already releases the maps, and the
shader draws no shadow past the distance anyway.

## Related

`docs/game-pitfalls.md` and QUEUE have no entry on `Shadows::distance` or the
camera's near plane.
