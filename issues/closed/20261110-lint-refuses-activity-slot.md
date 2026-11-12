# A type error anywhere makes the lint refuse every `slot` of an activity's placements

**Status:** Closed
**Resolution:** Fallback lint leaves activity placement slots to activity compilation while checking their other attributes and children.
**Systems:** contract/lower/src/lint.rs, contract/cli/src/lib.rs, contract/cli/src/snapshot/activities.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

When the type or analysis pass refuses, the driver appends `contract_lower::lint(file)` to the diagnostics (`contract/cli/src/lib.rs:1056-1064`). The lint walks every component of the file (`contract/lower/src/lint.rs:681-704`) and asks `tags::attr_valued` for each attribute (555-561, 592).

An activity's component declares its placements with `view slot="lockScreen"` (`docs/reference.md:565`, LLP 1126.001 WP8). `slot` is no attribute of `tags`: the activity compile reads it and takes it out before lowering (`contract/cli/src/snapshot/activities.rs:63-95`), and the app's own compile never lowers the activity component, because the root does not use it. The lint knows neither, so it reports `lower-unknown-attr` for each placement of a file that compiles.

## The failure

```contract
component App
  state n = 1 + "a"
  view
    text "x"

component Departure
  props
    train: number
  view
    view slot="lockScreen"
      text `${train}`
    view slot="minimal"
      text `${train}`
```

Observed:

```
2:13 [type-operand] `+` joins two strings, given `number` and `string`: write a template, `` `Total: ${n}` `` for `"Total: " + n`
10:10 [lower-unknown-attr] `view` has no attribute `slot`
12:10 [lower-unknown-attr] `view` has no attribute `slot`
```

Expected: the first line only. With `state n = 1` the file compiles with no diagnostic. In `apps/caltrain`, one type error in `app.contract` brings 18 such lines from `activities.contract`; in `apps/activity-fixture`, 9. The lint keeps at most 20 refusals and the driver 20 diagnostics in all (`lint.rs:712`, `lib.rs:1062`), so in Caltrain the 18 crowd out the lint's real findings. An author or agent who acts on them deletes the placements.

## Verification

At `dd6f5dcd`, `contract build` on the source above printed the three lines, and compiled it with `state n = 1`. A temporary test in `contract/cli/tests` then took every `.contract` file in the checkout that compiles as a root (637), appended a component holding one type error, and printed each diagnostic other than that error. The 27 `slot` lines (18 for `apps/caltrain/app.contract`, 9 for `apps/activity-fixture/app.contract`) were the lint's only output. (`examples/calendar/app.contract` is one `use` line, so the appended component became its root and three type errors followed; they are not the lint's.)

## Direction

Leave `slot` out of the lint's attribute walk, as it already leaves `class`, `delete` and the row-look mark (`lint.rs:555-561`). A `slot` on an element the root does lower is still refused by lowering: `view slot="lockScreen"` inside the root's view prints `3:10 [lower-unknown-attr]` with no other error in the file.

## Related

No issue, `QUEUE.md` entry or `docs/agent-pitfalls.md` entry mentions the lint with an activity's `slot`.
