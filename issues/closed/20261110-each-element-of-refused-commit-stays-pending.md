# A list resource's element first asked in a refused commit is never asked again on the JS target

**Status:** Closed
**Resolution:** A refused commit removes newly joined list resource elements so their next join asks again.
**Systems:** host/web-js/each.js, host/web-js/rt.js
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, a refused commit restores each resource from the checkpoint taken at its start, and only those:
`host/web-js/rt.js:193` skips `k >= saved.length` with the comment "a resource the commit made (an element's,
each.js) has nothing to put back". An element of a list resource made during the commit (`host/web-js/each.js:23`)
does. Its first settlement set `r.ticket`, wrote `pending` true and queued the request in `Out` (`rt.js:522-524`).
The refusal empties `Out` (`rt.js:195`), so the request is never sent, while the ticket and `pending` stay. The
element also stays in `members`: the settlement over the restored state marks it `left` and queues its removal with
`stood` (`each.js:29-37`), and a refused commit returns before running what `stood` queued (`rt.js:209`).

When the key joins again, `resEach` finds the member, clears `left` and reads its memo (`each.js:20-27, 40`).
Nothing the memo reads has changed, so it does not run. The element shows its zero and the list resource reads
`pending` for good.

The runner asks again: the tables a refused commit puts back hold nothing for a member it added
(`runner/src/runner/each.rs:243-250`, `restore_members` and `fit_resources`).

## The failure

```
component App
  state ids: number[] = [1]
  state bad = false
  resource items = item(n) for n in ids as shape number
  derive shown = items.map(v => `${v}`).join(",")
  derive share = 100 / (items.length + 1 - (bad ? items.length + 1 : 0))
  action both
    ids = [1, 2]
    bad = true
  action add
    ids = [1, 2]
  view
    column
      text shown testId="shown"
      text (pending(items) ? "pending" : "settled") testId="state"
      text `${share}` testId="share"
      button press=both testId="both"
        text "both"
      button press=add testId="add"
        text "add"
```

`item(n)` answers `n * 10`, later. Let element 1 settle, press `both` (element 2 is asked, then `share` refuses the
commit: 100 / 0), press `add`, and deliver every reply.

- Runner: `shown` reads `10,20` and `state` reads `settled`. The source was asked 1, 2 (the refused commit) and 2
  again (`request 2 (items): GET …/2`).
- JS target: `shown` reads `10,0` and `state` reads `pending` for good.

## Verification

- The plan above on the runner (a `DataSource` answering `Answer::Later`, the presses dispatched, each request
  fulfilled): `press both: Err("NonFiniteValue { name: \"share\" }")`, `asked [1.0, 2.0, 2.0]`, `after its reply:
  shown "10,20" state settled in flight 0`.
- The same plan built with `bun host/web-js/build.mjs` and driven in Chrome (`tap "both"`, `clock settle`, `tap
  "add"`, `clock settle`): `text of "shown" is "10,0", expected "10,20"` and `text of "state" is "pending"`, with
  "1 request still in flight: items".
- `resEach` over the real `rt.js` and `each.js` in Bun, the same sequence: the source was asked `[1, 2]` and not
  again; `items()` is `[10, 0]`, `items.p()` is `true`.

## Direction

Undo what a refused commit made: end and forget the elements whose resource is past `saved.length`, or let their
ticket and `pending` go and mark their memo to run again.

## Related

`QUEUE.md`, "The JS runtime starts every element of a list resource from one kept answer", is about a rendered
page's kept answers. No issue names `resEach`, and no test under `host/web-js` or `host/web/tests` names it.
