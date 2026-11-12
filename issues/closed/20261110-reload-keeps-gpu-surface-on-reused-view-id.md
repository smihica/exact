# A reload or an activated update that renumbers the tree leaves the old GPU surface on another node, and the new canvas cannot publish

**Status:** Closed
**Resolution:** The Linux presenter's `replaced()`, which a reload and an activated update both call, retires every GPU surface and the controls bound to them, so each canvas of the new tree makes its own surface and owns its name; on Android the reader's windows stay and are attached to the new canvases.
**Systems:** host/linux/src/surfaces.rs, host/linux/src/presenter.rs (replaced), host/linux/src/presenter/delivery.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

The painting host keeps GPU canvases in a map by view id
(`host/linux/src/surfaces.rs:444`) and retires one only when no node has that id
(`surfaces.rs:533-538`):

```rust
.filter(|id| host.kernel().node(*id).is_none())
```

Two paths replace the host and its kernel and keep the presenter:
`Presenter::reload_module`, a development reload (`presenter.rs:671`), and
`Presenter::activate_update`, a staged update applied by `deliveryActivate()` or
immediate activation in a release build (`presenter/delivery.rs:359`). Both call
`replaced()`, which clears the state keyed by view id that it lists
(`presenter.rs:1386-1412`) and leaves `self.surfaces` as it was.

The new runner numbers its nodes from 1 again. When the new plan puts the canvas at
another id, its old id names some other node, so the old surface is never destroyed.
The canvas's new node gets a second surface of the same name, which
`surfaces.rs:657-664` makes a non-owner, and only an owner publishes
(`surfaces.rs:887`), so its record never reaches the app. `postMessage` and the
agent's `post` go to the lowest view id of the name (`surfaces.rs:1053-1058`), the
stale one.

The web host handles this: a surface whose element changed is destroyed
(`host/web/gpu-glue.js:953`) and `reset` carries a unique surface name across the
restart (`gpu-glue.js:995-1000`).

## The failure

A plan with a GPU canvas, then the same plan with a line added above it, delivered as
a reload or as an update:

```
column
  text "new"                                  # added
  canvas surface=world(n) width=50 height=50
```

Observed after either: two surfaces named `world`, view 2 (now the `text`,
`owner: true`) and view 3 (the canvas, `owner: false`); the journal says
`surface world: duplicate canvas cannot publish`; a post to `world` goes to view 2.
Expected: one surface, on the canvas, owning its name.

When the new plan leaves the canvas at its id the surface stays with it. Within one
session ids are not reused, so a canvas replaced by another node in a commit is
retired.

## Verification

Temporary tests in `surfaces/tests.rs`, with the C module fixture those tests compile,
bound a canvas and then delivered the edited plan through `Presenter::reload` and,
separately, through `set_updates` and `Presenter::activate_update`. Each left
`[(2, "world", true, Some(Text)), (3, "world", false, Some(Canvas))]` as (view, name,
owner, node type), with the duplicate line in the journal. The unedited plan through
`activate_update` left `[(2, "world", true, Some(Canvas))]`.

Not run: a real GPU module, the display loop, Android, Windows.

## Direction

When the host is replaced, retire every surface before the new host's updates are
taken, or match them to the new tree by name as the web's `reset` does. A surface
should also go when its node is no longer a canvas.

## Related

`QUEUE.md`, "A restart keeps per-node state that a new node with the same id then
picks up" records this class for the Apple presenters' `Presenter.reset` and lists
only their state. Its proposal to number nodes per session rather than per runner
would cover this map too.
`issues/closed/20260901-gpu-artifacts-cross-app.md` fixed the web's pending surfaces on
reload.
