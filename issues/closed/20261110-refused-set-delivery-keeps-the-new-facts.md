# A refused `set_delivery` or `set_activities` keeps the new facts, so telling them again asks nothing

**Status:** Closed
**Resolution:** Refused delivery and activities updates restore the previous facts, so identical retries are evaluated again.
**Systems:** runner/src/runner/delivery.rs, runner/src/runner/activities.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Every host fact is told the same way: store the new fact, ask its readers again in one commit (`recommit`), and put the old fact back if that commit is refused. `replace_viewport` (`runner/src/runner/viewport.rs:85-93`, at `dd6f5dcd`), `set_time` and `set_place` (`time.rs:66-78`, `:146-151`), `set_page` (`page.rs:44-46`), `set_surface_record` (`surface_record.rs:78-88`) and `set_root_font_size` (`root_font.rs:185-198`) all do the last step.

`set_delivery` does not (`delivery.rs:127-139`): it assigns `self.delivery` and returns `recommit`'s result. Nor does `set_activities` (`activities.rs:96-109`), which assigns `self.activities.snapshot` first. After a refusal the runner holds the new facts while every reader still shows the old answer, and each setter's first check (`delivery == self.delivery`, `snapshot == self.activities.snapshot`) makes a host's retry with the same facts return `Ok(None)` without asking. `state.delivery` then disagrees with what the app's `exactDelivery()` resource shows until some other fact changes.

`link_activities` (`activities.rs:61-67`) also keeps `linked` after a refusal, but it has no such check, so calling it again asks again.

## The failure

```
shape Delivery
  seq: number
component App
  resource delivery = exactDelivery() as shape Delivery
  derive spare = 1 / (5 - delivery.seq)
  view
    text `${spare}`
```

```rust
let mut d = r.delivery().clone();
d.seq = 5;
assert!(r.set_delivery(d.clone()).is_err()); // `spare` is not finite: refused
r.delivery().seq                              // 5
r.resource("delivery")                        // Record([Number(0.0)])
r.set_delivery(d)                             // Ok(None)
```

Expected, as `set_viewport` behaves beside it (width 500 refused, `r.viewport().width` still 400): `r.delivery().seq` is 0 after the refusal, and telling seq 5 again asks the resource again.

The same with `resource acts = exactActivities() as shape Acts` (fields `frequentPushesEnabled` and `linked`) and `derive spare = 1 / (acts.frequentPushesEnabled ? 0 : 1)`: `set_activities` with `frequentPushesEnabled` true is refused, the same JSON again returns `Ok(None)`, and the JSON the resource still shows (false) is taken as a change and commits.

## Verification

Two temporary tests booted these apps with `Runner::boot`. The first printed:

```
viewport after a refused set_viewport(500): runner 400, resource Some(Record([Number(400.0)]))
delivery after a refused set_delivery(seq 5): runner seq 5, resource Some(Record([Number(0.0)]))
the same facts again: Ok(None)
```

The second printed:

```
set_activities(frequentPushesEnabled true): Err(NonFiniteValue { name: "spare" })
the same facts again: Ok(None)
resource after: Some(Record([Bool(false), Bool(false)]))
the facts the resource shows, told: Ok(Some(commit))
```

and, with the derive reading `acts.linked`, `link_activities` returned the refusal both times it was called.

## Direction

Keep the previous `Delivery` and the previous activities snapshot, and put each back when `recommit` returns an error, as `replace_viewport` does.

## Related

No issue or QUEUE entry names `set_delivery`'s or `set_activities`' rollback.
