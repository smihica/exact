# A grouped list's button row made native by a class is refused for a `ua:` row when its face holds a `match`

**Status:** Closed
**Resolution:** Grouped native buttons accept match faces after both arms lose the list's UA sheet; authored face attributes remain checked.
**Systems:** contract/lower/src/grouped.rs, contract/lower/src/controls.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

A grouped list writes its sheet into a row's parts as attributes named `ua:<row>` (`contract/lower/src/grouped.rs:135`, `part` at 1187). `part` reads through both `if` (1198) and `match` (1224), so the texts inside either get sheet rows. A `button` row that is native through its class (`class=Native`, the style saying `appearance="auto"`) is not seen as native when the sheet is written, because `styled` reads only the row's own `appearance` (885-889), so its face is marked like any row's.

Lowering then takes the marks off a native button's face with `grouped::unsheet` (165-218). Its `strip` and `marked` handle `Node::Element` and `Node::When` only: a `Node::Match` is cloned with its marks. `face_counts` (`contract/lower/src/controls.rs:977-1000`) then meets `ua:flex-grow` on the `text`, which no native button's face takes, and refuses the button.

## The failure

```contract
style Native
  appearance="auto"
component App
  state name: option<string> = none
  action go
  view
    column width=402 height=874
      list appearance="auto" flex=1
        section
          button class=Native press=go
            match name
              case some(n)
                text n
              case none
                text "None"
```

Observed:

```
13:17 [lower-button-style-attr] a native button's `text` takes no `ua:flex-grow`; write a custom `button` for other face rows
```

Expected: it compiles. The message names an attribute the author did not write and cannot write. A native button's face may hold a `match` (`face_counts` reads through it, `controls.rs:1083`), and each neighbouring form compiles:

| Row | Result |
| --- | --- |
| `button class=Native`, face a `match` | refused, as above |
| `button class=Native`, face an `if`/`else` of two texts | compiles |
| `button class=Native`, face one `text` | compiles |
| `button appearance="auto"`, face the `match` | compiles |
| `button` with no appearance, face the `match` | compiles |
| `button class=Native`, face the `match`, outside a list | compiles |

## Verification

At `dd6f5dcd` a temporary test in `contract/cli/tests` called `contract::compile` on each source in the table and printed the result: the refusal for the first row, `OK` for the rest.

## Direction

Make `unsheet`'s `strip` and `marked` read through `Node::Match` as they do `Node::When`: the two regions `part` marks through.

## Related

No issue, `QUEUE.md` entry or `docs/agent-pitfalls.md` entry mentions a `ua:` row or a class-made native button in a grouped list.
