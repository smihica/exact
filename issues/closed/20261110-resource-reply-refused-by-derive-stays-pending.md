# A resource whose reply's commit is refused by a derive stays pending forever on the JS target

**Status:** Closed
**Resolution:** A refused resource reply releases its ticket, records the failure and keeps its last value, including when a derive refuses the commit.
**Systems:** host/web-js/rt.js, runner/src/runner/commit.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, `reply` (`host/web-js/rt.js:443-457`) lands a request's answer in a commit of its own. When that
commit is refused it lets the ticket go only for a data or shape failure, or for a mutation (`rt.js:453`):

```js
... !== false || !(Refused instanceof Failed || !t.r) || !held()) return;
```

For a resource (`t.r`) refused by anything else, it returns. The rollback has put the ticket back (`rt.js:193`,
`r.restore`), the one reply is spent, and nothing asks again: `pending(x)` stays true and the resource never fails.
The usual other refusal is a derive that reads the answer: a value that is not finite, a trap, a string past its
bound (each a `Refusal`, not a `Failed`: `rt.js:104`, `budget.js:9`).

The runner releases the request whatever refused the reply's commit (`runner/src/runner/commit.rs:1216-1232`: "A
one-shot reply is spent whatever refused its commit … whether the source failed to take it … or a value it took
trapped in a derive"), and does the same for a stream's message (`commit.rs:1296-1298`).

## The failure

```
component App
  resource count = count() as shape number
  derive share = 100 / (count - 5)
  view
    column
      text `${share}` testId="share"
      text (pending(count) ? "pending" : "settled") testId="state"
```

with a source that answers `count` with 5 after a moment. The reply makes `share` 100 / 0.

- Runner: `fulfil 1 (count) … refused: NonFiniteValue { name: "share" }`, then `request 1 (count) failed and is no
  longer pending: it keeps its last value: NonFiniteValue { name: "share" }`. `state` reads `settled`, no request is
  in flight, one resource is failed.
- JS target: `refused reply count …: derive "share": value does not conform to its type`, and `state` reads
  `pending` after `clock settle`. The driver reports "1 request still in flight: count".

## Verification

- The plan above on the runner (a `DataSource` answering `Answer::Later`, then `fulfill`): the journal lines above,
  and `after the reply: share -20 state settled in flight 0 failed resources 1`.
- The same plan built with `bun host/web-js/build.mjs` and driven in Chrome (`clock data`, `clock settle`, `expect
  text "state" == "settled"`): `text of "state" is "pending", expected "settled"`.
- The real `rt.js` in Bun: `pending true failed false ticket held true`.

A stream's refused message was not run on the JS target.

## Direction

Drop the `Refused instanceof Failed || !t.r` test: when the reply's commit was refused and the target still holds
the ticket, call `gone` with the refusal's message (code `error` when it is no `Failed`) and make the following
commit, for a resource as for a mutation.

## Related

LLP 1071, "Failed replies and the document's head" (2026-10-02), describes the rule `rt.js` still has ("Any other
refusal keeps the ticket"). The runner's rule is wider now, and that paragraph needs the same change.
`issues/closed/20260901-fulfill-drops-failed-ticket.md` is the runner's earlier rule, which its comment at
`commit.rs:1216` replaces. Nothing in `QUEUE.md` or `docs/agent-pitfalls.md` covers this.
