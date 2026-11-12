# `contract vocab` says `class`, `part` and `-exact-row-look` are not attributes, and the compiler admits all three

**Status:** Closed
**Resolution:** Vocab describes class, part and -exact-row-look in text, JSON and full listings, including their values and scope.
**Systems:** contract/cli/src/vocab.rs, contract/lower/src/vocab.rs, docs/contract-grammar.md
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`contract vocab <name>` is the lookup the guides send an author to for any attribute
(`AGENTS.md`: "`contract vocab <name>` answers for any tag or property";
`docs/contract-grammar.md:1216`: "`contract vocab` lists every tag, style and prop name the
compiler admits"). Three attributes are lowered by their own modules and are in none of the
tables `vocab` reads, so it denies them:

| Attribute | Lowered by | In the guides |
|---|---|---|
| `class` | `contract/lower/src/class.rs:27` | `docs/contract-for-agents.md:1430` |
| `part` | `contract/syntax/src/anatomy.rs:58` (a component's `parts`) | `packages/lists/switch-row.contract:22` |
| `-exact-row-look` | `contract/lower/src/row_look.rs:10` | `docs/contract-grammar.md:1275` |

`contract vocab --json` lists 611 attributes and none of the three.

## The failure

```
$ contract vocab class
`class` is not a tag, an attribute, a standard function, a recipe or a pattern; try `contract vocab glass` (a concept)
$ contract vocab -exact-row-look
`-exact-row-look` is not a tag, an attribute, a standard function, a recipe or a pattern; `bun exact.mjs docs <words>` prints the guides' section on a topic
$ contract vocab part
`part` is not a tag, an attribute, a standard function, a recipe or a pattern; `bun exact.mjs docs <words>` prints the guides' section on a topic
```

Each exits 1. Both of these compile:

```contract
style Card
  padding=12
component App
  view
    column class=Card -exact-row-look="platform"
      text "a"
```

```contract
component App
  state on = false
  action flip(v: bool)
    on = v
  view
    SwitchRow(id="a", checked=on, set=flip)
component SwitchRow
  props
    id: string
    checked: bool
    set: action
  parts
    switch: control
  view
    row
      input part="switch" type="checkbox" switch testId=id checked=checked change=set aria-label="x"
```

Expected: each name prints an entry (what it takes and where), as `id`, `testId`, `hatch` and
`atoms` do. An author who looks up `class` is told it does not exist and is pointed at glass.

## Verification

Ran the three lookups (exit 1 each) and the four that answer (exit 0 each), and
`contract build` on both programs, which built. The compiler knows each name: `class=Nope`
is refused as `lower-unknown-class`, and `part="switch"` in a component that declares no
such part as `lower-anatomy-part`. Searched the names of the `attributes` array of
`contract vocab --json` for the three.

## Direction

Give the three names entries in the vocabulary's attribute table (kind, accepted value, the
one-line rule each module's comment already states), so the text form and `--json` both carry
them.

## Related

`issues/20261110-vocab-json-refuses-headings.md` is the same lookup failing for `h1` to `h6`
in `--json` only; these three fail in both forms.
