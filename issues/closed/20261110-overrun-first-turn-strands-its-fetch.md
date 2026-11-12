# An answer whose first turn runs over budget after starting a fetch is never let go, so the fetch never settles and its `finally` never runs

**Status:** Closed
**Resolution:** Refused first turns release their calls, withdraw queued fetches and run rejection continuations before retry.
**Systems:** js/src/lib.rs (`Module::begin`), js/src/turns.rs (`forget_calls`), js/src/prelude.js
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, `Module::begin` (`js/src/lib.rs:854`) refuses a first turn that ran over its budget, or that an interrupt ended, without telling the prelude. At `lib.rs:944-947` it clears the reply and prunes secret calls; at `lib.rs:977-981` it returns `Overrun`. The prelude's call stays in `calls` as `pending`, and the fetch it started is neither sent nor rejected. `Module::resume` handles the same outcome by letting the call go (`lib.rs:1268-1271`, `forget_calls`), which rejects its fetches as `Aborted` so its continuation runs.

The runner asks an overrun resource again at 250 ms (LLP 1027 D10; LLP 1041 D2, "the one automatic retry"). What the refused turn left behind decides that ask: a busy flag its `finally` would have reset stays set, and a promise it shared stays pending for the life of the engine.

## The failure

```ts
let busy = false, first = true;
async function flag() {                      // source "flag"
  if (busy) return "busy";
  busy = true;
  try {
    const reply = fetch(URL + "/flag");
    if (first) { first = false; spin(); }    // about 600 ms of work, once
    return await (await reply).text();
  } finally { busy = false; }
}
```

With `resource m = flag() as shape string else empty()` and the 100 ms budget:

```
t=0    resource m: `flag` took 605.2 ms of its own (CPU) time (606.1 ms of wall), over the
       100 ms budget for one turn; … asked again in 250 ms (1 of 2)
t=250  m = "busy"     calls_open=1 unsent=1
```

`m` stays `"busy"` at every later ask, and no request is handed to the host. A source that memoizes the fetch's promise across answers fails at 250 ms instead: "the answer is pending on nothing: it awaits a promise that no fetch or storage step in flight in this module will settle". Expected: the fetch is rejected, the `finally` runs, and the ask at 250 ms fetches and answers.

## Verification

Ran both sources in a temporary `exact-js` test that boots a `Runner` over the module with `set_budget_ms(100.0)`, advances its clock by 250, 1,000 and 5,000 ms, and answers each fetch with `DATA`. It printed the lines above. With `forget_calls` for the call added to `begin`'s refusal, the `flag` run printed `request 2 (m): GET https://fixture.exact.test/flag`, `m="DATA"`, `failed=[]` and `calls_open=0`, and the memoized source recovered the same way; `unsent` stayed 1 in both. Not run: the interrupt path, which leaves `begin` through the same lines.

## Direction

Keep the call id `__exact_call` returns and, when `begin` refuses the turn, `forget_calls` it as `resume` does. Withdraw its recorded request too.

## Related

QUEUE "A large reply's owed Hermes collection can wait forever" notes a call left in `calls` as `done` when its serialization throws; this is a `pending` call left by a refused turn. `issues/closed/20261102-runner-overrun-replays-effectful-requests.md` is about a reply's overrun after a request with effects.
