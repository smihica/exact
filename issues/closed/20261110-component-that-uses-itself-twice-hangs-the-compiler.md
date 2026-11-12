# A component that uses itself twice hangs `contract build` while its memory grows without bound

**Status:** Closed
**Resolution:** The inliner keeps the components on the current expansion path and refuses a use of one already on it, at once, naming the cycle; a slot's fill is inlined with its use site's path.
**Systems:** contract/syntax/src/inline/use_node.rs, contract build
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at `dd6f5dcd`. A component cannot use itself: LLP 1017.002
lists "recursive components" as absent and says "Recursion and an expansion
beyond the declared node budget reject by name." The inliner finds a cycle only
by counting: a use more than 32 uses deep is refused
(`contract/syntax/src/inline/use_node.rs:20-27`, "nests too deeply (a
cycle?)"). The refusal is recorded and expansion goes on
(`Ctx::refuse`, `contract/syntax/src/inline.rs:465`; `inline_use` returns an
empty list), so every other path is expanded to the same depth before anything
is printed. A component whose view uses itself twice, or a cycle of components
with two uses, has about 2^33 paths.

## The failure

```
component App
  view
    Tree(n=1)

component Tree
  props
    n: number
  view
    column
      text `${n}`
      Tree(n=n + 1)
      Tree(n=n + 2)
```

Observed: `contract build` prints nothing. Stopped after 8 s it had used 6.2 s
of CPU and 3.2 GB of memory. The same happens with the two uses under
`if depth < 3`, which is how a tree is usually written, and with `A` using `B`
twice and `B` using `A` twice.

Expected: the refusal the one-use form (the same file without the last line)
gets in 10 ms:

```
11:7 [syntax-inline-depth] component `Tree` nests too deeply (a cycle?)
```

## Verification

Ran `contract build` (debug) on the four programs under an alarm: the one-use
form is refused at once; the other three ran to the alarm (5 to 8 s) with no
output, at 1.2 to 4.4 GB when stopped.

The growth is the expansion's. Acyclic chains of the same shape (`T0` uses `T1`
twice, down to `Tk`) compile, with these costs:

| Levels | Nodes | Time | Memory |
| --- | --- | --- | --- |
| 12 | 12,286 | 0.38 s | 60 MB |
| 14 | 49,150 | 1.66 s | 193 MB |
| 16 | 196,606 | 8.07 s | 755 MB |

Each level doubles the nodes; the self-using form goes to 33 levels.

## Direction

Keep the names of the components being expanded on the current path in `Ctx`,
and refuse a use whose component is already on it, with the same
`syntax-inline-depth` refusal. A use is expanded unconditionally, so a
component on its own path never terminates; the count of 32 can stay for long
acyclic chains.

## Related

`issues/closed/20261009-compiler-small-stack-depth.md` bounded stack depth for
nested sites and component chains; it measured no cycle with more than one use.
`QUEUE.md`, "An action parameter with no type makes the Contract compiler
spin", is another input: this one has no action.
