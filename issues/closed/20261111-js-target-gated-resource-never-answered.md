# On the JS target, a resource whose `when` gate opens is never answered

**Status:** Closed
**Resolution:** The JS target was right; the difftest harness took a resource its closed `when` left unasked at boot for a host fact and answered it the boot zero on the JS side forever, so it now records facts only for the runner's own sources (`exact_plan::runner_owned_source`).
**Systems:** host/web-js/rt.js, semantics/corpus/resources/gate.contract
**Severity:** P2
**Author:** Claude (Fable 5.2), from the fix lanes of the 2026-11-10 project review
**Date:** 2026-11-11

## What is wrong

At `c307608e`. `resource x = src(…) when cond` asks nothing while `cond` is false, and
"when the gate opens it asks again" (LLP 1161.002; commit `a483a1b2`). On the web JS
target a resource whose gate opens shows `current(x)` true and never shows an answer:
its value stays the type's zero through every later step. The native runner lands the
answer in the step that opens the gate.

`semantics/corpus/resources/gate.contract` holds the cases. Two of them diverge between
the runner and the JS target, and they are the only divergences in the corpus (599
cases: 587 agree, 2 diverge, 10 outside the JS target).

## The failure

`cargo run -p contract-difftest -- corpus semantics/corpus --js-only` prints

```
DIVERGE-JS corpus/resources/gate.contract: close, refresh while idle, wake at line 68:
  rust: resource hits [{"x y",n405edd2f1a9fbe77}]
  js:   resource hits []
DIVERGE-JS corpus/resources/gate.contract: wake with new arguments at line 28:
  rust: resource hits [{"ü",…},{"ü",…},{"-",…},{"ü",…}]
  js:   resource hits []
```

In the second case the events are `tap "show"`, `land`, `tap "hide"`, `land`,
`type "q" "bc"`, `land`, `tap "show"`, … . After the first `tap "show"` the two traces
differ in four lines and keep differing:

```
runner                                   JS target
resource hits [4 records]                resource hits []
resource count 3                         resource count 0
view "hits" "4 true"                     view "hits" "0 true"
view "count" "3 true"                    view "count" "0 true"
```

In the first case `more` (`when wanted and count >= 0`) stays `none` on the JS target
where the runner has `some("-")`. Expected: the JS target asks when the gate opens and
lands the answer, as the runner does.

## Verification

Ran the command above in a checkout at `c307608e` and read the kept traces under
`target/difftest/failures/`. The same two cases diverge with thirteen later commits of
other fixes applied, none of which touches the JS target's resources. The cause in
`host/web-js/rt.js` was not found: nothing here says whether the ask is never made or
its reply is dropped.

## Direction

Find which it is with the kept events (`target/difftest/failures/*-js-270.events`): log
the ask in `rt.js`'s resource `ask` path and the ticket in `reply`. A resource's ticket
"follows its current arguments" since `f0966a71`, and a gate that opens at the
arguments the resource already holds (never asked while closed) is a case that change
did not have; start there.

## Related

`host/web-js/conformance/gate.contract`, added with the feature.
