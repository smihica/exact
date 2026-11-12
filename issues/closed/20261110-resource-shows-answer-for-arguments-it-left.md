# A resource whose arguments return to its settled ones keeps the request it left in flight and shows its answer, on the JS target

**Status:** Closed
**Resolution:** Resource reuse follows the request in flight, so returning to settled arguments replaces an intervening request and drops its late reply.
**Systems:** host/web-js/rt.js
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, a resource's settlement returns its value as soon as the arguments equal the ones it settled with
(`host/web-js/rt.js:495`), before it looks at the ticket in flight (`rt.js:496`). When a request for other arguments
is in flight at that moment, the ticket stays held and `pending` stays true. Its reply then lands as the resource's
answer (`rt.js:480`, `take(p.v, t.args, …)`): the resource shows the value for arguments it no longer has, and only
then asks for the current ones.

LLP 1016 D5 says what should happen: "When settlement finds a pending resource whose arguments changed, the old
ticket is forgotten (its outcome is dropped on arrival) and a new request goes out." The runner does that
(`runner/src/runner/settlement.rs:577-587, 630`: reuse follows the arguments of the request in flight).

## The failure

```
component App
  state key = "A"
  resource r = src(key) as shape string
  action set(k: string)
    key = k
  view
    column
      text r testId="r"
      text (pending(r) ? "pending" : "settled") testId="state"
      button press=set("B") testId="b"
        text "B"
      button press=set("A") testId="a"
        text "A"
```

`src` answers `"v" + key`, later. Let `r` settle at A, press B, press A before B's reply, then deliver B's reply.

- Runner: the second press logs `forget request 2 (r)` and `request 3 (r): GET …/A`; B's reply logs `reply 2 dropped:
  no such request in flight`. `r` reads `vA` at every step.
- JS target: after the second press the ticket for B is still held. After B's reply `r` reads `vB` while `key` is
  `"A"`, pending, until the third request answers.

An author meets this with a search field after a character is typed and deleted, or a filter toggled twice, while
the first request is still out.

## Verification

- The plan above on the runner (a `DataSource` answering `Answer::Later`, the presses dispatched, B's ticket
  fulfilled last): the journal lines above, and `after B's reply: value vA pending in flight 1`.
- The same plan built with `bun host/web-js/build.mjs` and driven in Chrome, B's answer 200 ms late (`clock data`,
  `tap "b"`, `tap "a"`, `clock +250`, `expect text "r" == "vA"`): `text of "r" is "vB", expected "vA"`.
- The real `rt.js` in Bun: `back to A: value vA pending true ticket B`, then `after B's reply: value vB pending true
  ticket A`; the source was asked `A,B,A`.

## Direction

In `res`, when the arguments equal the settled ones and a ticket for other arguments is held, forget that ticket
before returning the value (`release()` journals the forget and clears `pending`), or fall through and ask again as
the runner does. Either way the late reply finds its ticket no longer held and is dropped.

## Related

`issues/closed/20260927-failed-request-relabels-answer.md` is about a failed request's arguments. `QUEUE.md`, "The
fragment after LLP 1153 D5", item 1, is about `held.js` counting a ticket the resource already let go; here the
resource does not let it go.
