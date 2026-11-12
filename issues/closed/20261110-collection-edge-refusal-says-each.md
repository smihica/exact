# The `reachend` refusal, `contract vocab reachend` and docs/reference.md tell the author to write `each`, which the parser refuses

**Status:** Closed
**Resolution:** Collection-edge refusals, vocabulary entries and reference guidance require one direct `for` in a virtualized list.
**Systems:** contract/lower/src/vocab.rs, docs/reference.md
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`. A view repeats with `for`. `each` is a retired
spelling: the parser refuses it (`syntax-view-keyword`, "a view repeats with `for`"). Four
texts still say `each` where an author must write `for`:

- `contract/lower/src/vocab.rs:150`, the `lower-collection-edge` refusal: "write
  `list virtualized=true` with one direct `each`"
- `contract/lower/src/vocab.rs:98` and `100`, printed by `contract vocab reachstart` and
  `contract vocab reachend`: "only on `list virtualized=true` with one direct `each`"
- `docs/reference.md:2931-2932`: "`reachstart` and `reachend` belong only to
  `list virtualized=true` with one direct `each`"
- `docs/reference.md:2938-2939`: "one more item of the `each`'s list … The `each`'s one root"

## The failure

```contract
component App
  state xs = ["a"]
  state n = 0
  action more
    n = 1
  view
    list height=300 reachend=more
      for x in xs key=x
        text x
```

```
app.contract:7:21 [lower-collection-edge] `reachend` belongs to a virtualized list (LLP 1010): write `list virtualized=true` with one direct `each`. …
```

Writing what it says:

```contract
    list virtualized=true height=300 reachend=more
      each x in xs key=x
        text x
```

```
app.contract:8:7 [syntax-view-keyword] a view repeats with `for`: write `for x in xs key=x.id`
```

With `for` in place of `each` the second program builds.

Expected: the refusal, the vocabulary and the reference say "one direct `for`", as
`docs/contract-for-humans.md:452` does ("exactly one direct `for`").

## Verification

Ran `contract build` on the three programs and `contract vocab reachstart` and
`contract vocab reachend`; the outputs above are what they printed, and both `vocab` entries
print the "one direct `each`" line and the refusal's text. Searched `contract/`, `runner/` and
`docs/` for a backticked `each`: these are the only texts an author reads that have it.

## Direction

Replace `each` with `for` in the three strings of `vocab.rs` and the two sentences of the
reference.

## Related

No entry in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` names the refusal's wording.
`runner/src/runner/each.rs` and the web runtime's comments keep the old word as an internal
name, which no author sees.
`issues/20261110-vscode-extension-knows-each-not-for.md` is the editor grammar's copy of the same retired word.
