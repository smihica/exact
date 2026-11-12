# On the web JS target a `path()` binding whose parameter becomes empty poisons the app; the runner unsets the binding

**Status:** Closed
**Resolution:** The web JS target traps empty and dot path parameters as PathParameter, so bindings unset, journal once, and recover without poisoning later updates.
**Systems:** host/web-js/budget.js, host/web-js/router.js, semantics/corpus/routes/binding-trap.contract
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at `dd6f5dcd`.

A prop or style binding that traps is unset, journaled once, and the update goes on (LLP 1017.003 D3, LLP 1090 D6). The JS target guards such a binding with `tg`, which catches a `Trap` and rethrows anything else (`host/web-js/budget.js:17`). `path(route, …)` compiles to `x_encodeRouteSegment`, which hands a parameter that is empty, `.` or `..` to `segmentOf` (`budget.js:285`), and that throws a plain `Refusal` (`host/web-js/router.js:33`), the class `Trap` extends. The refusal escapes the guard, the tree update fails, and the runtime poisons itself (`host/web-js/rt.js:207`): every later event is refused with "the runner is poisoned; reload" (`rt.js:168`).

The runner makes the same condition `Trap::PathParameter` (`runner/src/vm.rs:953`) and absorbs it like any binding trap (`runner/src/instance/trapped.rs:40`): the `href` is unset and the app goes on. The Lean semantics agrees with the runner.

## The failure

```contract
routes nav
  tab chats "/"
    chat "/chat/:id"

component App
  state id = "first"
  state n = 0
  action clear
    id = ""
  action bump
    n = n + 1
  view
    column
      link href=path("chat", id) testId="chat-link"
        text "Chat"
      button "Clear" press=clear testId="clear"
      button "Bump" press=bump testId="bump"
      text `${n}` testId="n"

test "an empty parameter then another tap"
  tap "clear"
  tap "bump"
  expect state n == 1
```

Runner and semantics: both taps commit and `n` is 1; the journal says `href trapped Trap(PathParameter { pc: 22 }); unset`. JS target: `tap "clear"` ends ``poisoned: a path parameter cannot be empty, `.` or `..` ``, and `tap "bump"` is `refused action: the runner is poisoned; reload`.

An app reaches this when the parameter becomes empty, `.` or `..` after launch: a row id from a data source (`link href=path("chat", r.id)` in `apps/agents`, `path("book", book.id)` in `apps/shelf`), or typed text (with an `input` bound to `id` in the program above, typing `..` or clearing the field poisons the page). A parameter that is empty in the first frame fails the build instead (`bake-binding-trap`).

## Verification

`cargo run -p contract-difftest -- corpus --js <file>` on the program above printed "1 cases: 1 agree" for the semantics and `DIVERGE-JS … rust: outcome ok / js: outcome poisoned … lean: agrees with the runner`. `semantics/corpus/routes/binding-trap.contract` diverges the same way in both its tests, which fails the async lane's `difftest-js` check (`scripts/async.mjs:317`). With `budget.js:285` changed to `throw new Trap("PathParameter", pc)`, those cases and all 24 of `semantics/corpus/routes` agreed; the change was reverted. The JS side ran under difftest's headless driver (the build's compiler and runtime, under Bun), not in a browser.

## Direction

Throw `Trap("PathParameter", pc)` from `x_encodeRouteSegment` for the three values. The `pc` is at hand there, and the journal then carries the runner's text.

## Related

`issues/closed/20261030-binding-trap-poisons-the-runner.md` set the rule and fixed the traps budget.js throws itself; the route segment's refusal was left a plain `Refusal`. Nothing in `QUEUE.md` or `docs/agent-pitfalls.md` names it.
