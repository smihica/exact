# `contract migrate` puts `:active` ahead of an element's continued attribute lines, which then become rows of the part

**Status:** Closed
**Resolution:** Press migration inserts a new :active block after continued attributes, preserving their ownership and multiline values.
**Systems:** contract/syntax/src/migrate/press.rs, contract migrate
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at `dd6f5dcd`. `contract migrate` moves a retired
`-exact-press-background-color` or `-exact-press-color` into an `:active` part
(`docs/reference.md:195`). The compiler's refusal of either attribute tells
the author to run it (`contract/lower/src/lint.rs:311-312`).

For an element with no `:active` part yet, `Paints::owner` inserts the part at
the start of the line after the owner's first line
(`contract/syntax/src/migrate/press.rs:161-175`), indented to the element's
children (`body_indent`, `:89-117`). Attributes may continue on lines indented
deeper than the children (`docs/contract-grammar.md:582`). Those lines then
follow the inserted `:active` at a deeper indent and parse as its rows.

## The failure

```
component App
  state n = 0
  action up()
    n = n + 1
  view
    column
      row press=up
          color="#111"
          -exact-press-background-color="#eee"
        text "Go"
```

`contract build` refuses the retired attribute and says "`contract migrate
<file.contract>` moves this attribute". `contract migrate` prints "migrated
retired spellings", exits 0 and writes:

```
      row press=up
        :active
          background-color="#eee"
          color="#111"

        text "Go"
```

`color="#111"` is now a row of `:active`: the row has that colour only while
pressed. The file builds with no diagnostic. Expected: `color` stays the
element's, with `:active` after the continued lines.

With a continued row that a state part does not take (`padding=8` in place of
`color`), the migrated file no longer builds:

```
10:11 [lower-state-row] `padding` in `:active`: a state part takes paint rows only, …
```

Continued lines at the children's indent, an element with no children, a
`style` block and a one-line element migrate correctly.

## Reach

The migration is current, and it exists for apps outside the repository: no
tracked `.contract` file has either attribute. The layout that triggers it is
legal but is not the one `contract fmt` writes: of the 116 elements under
`apps/`, `examples/` and `docs/` that have children and continued attributes,
none continues deeper than its children. Running `contract fmt` before
`contract migrate` avoids the defect.

## Verification

Ran `contract migrate` on the program above, then `contract fmt --stdout` and
`contract build` on the result: the formatter prints `color` under `:active`,
and the build is silent. The same source with `:active` placed after `color`
by hand builds with the `lower-colour-one-scheme` warning for `#111` on
`color`, and its plan differs from the migrated file's. The `padding` variant
prints the `lower-state-row` refusal. Not run: a host, to see the colour.

## Direction

Insert the part after the element's last attribute line (the line after the
last attribute's or positional's token), not after its first line; keep
`body_indent` for the indent.

## Related

`QUEUE.md`, "Two `-exact-press-*` paints duplicate the `:active` state part",
records the retirement this migration serves. Nothing in `issues/` or
`docs/agent-pitfalls.md` names the migration.
