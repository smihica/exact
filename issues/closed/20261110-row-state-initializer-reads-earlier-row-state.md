# On the web JS target a child state initialized from the child's earlier state fails the build, or builds clean and throws when the page runs

**Status:** Closed
**Resolution:** Web JS initializes child state in declaration order within its owning row or arm, so later initializers can read earlier state.
**Systems:** host/web-js/src/regions.rs, host/web-js/src/code.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at `dd6f5dcd`.

A component used inside a `for`, `if` or `match` keeps its state on the row or arm (LLP 1017.000 P4c). The runner starts those slots in declaration order, and a later one may read an earlier one (`semantics/corpus/rows/row-init-chain.contract`).

The JS emitter writes them as one object literal, `const $a1={...$r0,2:sig(…),3:sig(…)}` (`Em::owned`, `host/web-js/src/regions.rs:90-128`). Each initializer is compiled at `:102` while `scope.rows` still names the enclosing instance's object; it names the new object only at `:126`. A read of an earlier slot of the same instance (`host/web-js/src/code.rs:396-404`) is therefore compiled against the wrong object:

- where no enclosing row or arm owns state, `scope.rows` is `None` and the web build fails;
- where one does, the read goes to that object, which lacks the slot: the build is clean and the page throws.

## The failure

```contract
component App
  state items = [1, 2]
  view
    column
      for i in items key=i
        Outer(n=i)

component Outer
  props
    n: number
  state open = true
  view
    column
      if open
        Inner(n=n)

component Inner
  props
    n: number
  state a = n * 2
  state b = a + 1
  view
    text `${a} ${b}` testId=`inner-${n}`
```

Emitted: `const $a1={...$r0,2:sig((i0()*2),"n"),3:sig(($r0[2]()+1),"n")};`.

Runner: `inner-1` reads `2 3` and `inner-2` reads `4 5`. JS target: the build reports 0 warnings and the page never boots, `boot refused while building the view: TypeError: $r0[2] is not a function`. With `state open = false` and a button that sets it, the press poisons the runtime (`poisoned: $r0[3] is not a function`), and every later event is refused until reload.

With `Inner` used directly in the `for`, or in an `if` at the root, `bun host/web/build.mjs` exits 1 with `child state b#1: a row slot read with no row in force`, which names no source line. The other hosts build and run all of these.

No app in `apps/` and no guide example initializes a child's state from its earlier state today.

## Verification

`cargo run -q -p contract-difftest -- corpus <file> --js-only` printed `DIVERGE-JS … rust: outcome ok / js: outcome refused` for the program above and `js: outcome poisoned` for the opened-later form. A development web build of the program above, opened in the pinned Chromium by `bun exact.mjs agent web tree`, ended `the page never booted` with the same refusal.

With `owned` changed as below, those programs and `rows/row-init-chain.contract` (`OUTSIDE-JS` before) agreed, and `semantics/corpus/rows` and `semantics/corpus/components` had no divergence. The change was reverted.

## Direction

Emit `const $a1={...$r0};`, point `scope.rows` at it, then one `$a1[k]=sig(…);` per slot in declaration order.

## Related

`QUEUE.md:1346-1348` notes, inside the parked item "A JS-target refusal fails the web build only", that child-state initializers must run in declaration order and that `row-init-chain.contract` is refused. It records the refused build only, and its fix is on a parked branch; the clean build that throws at run time is not recorded. `issues/closed/20260901-row-init-misses-item.md` fixed the Contract compiler's side, an initializer reading the item.
