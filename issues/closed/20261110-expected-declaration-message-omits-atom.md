# The refusal of an unknown top-level word lists the declarations a file may hold and leaves out `atom`

**Status:** Closed
**Resolution:** The expected-declaration diagnostic lists atom beside decoration among the accepted top-level declarations.
**Systems:** contract/syntax/src/parser.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`. `Parser::declaration`
(`contract/syntax/src/parser.rs:518-615`) dispatches on the first word of a top-level line.
Its fallback refusal names what it expected (`parser.rs:604-610`):

> expected `routes`, `font`, `sound`, `shape`, `style`, `decoration`, `keyframes`, `timeline`,
> `color-profile`, `theme`, `fn`, `use`, `component`, or `members`, found …

The dispatch also takes `atom` (`parser.rs:542`), which the message does not list. An author
who misspells a declaration reads the list as the whole of what a file may hold.
`docs/contract-grammar.md:96` gives a list that differs the other way (it has `atom` and lacks
`timeline` and `color-profile`), so neither text is complete.

## The failure

```contract
banana
component App
  view
    text "a"
```

```
app.contract:1:1 [syntax-expected-declaration] expected `routes`, `font`, `sound`, `shape`, `style`, `decoration`, `keyframes`, `timeline`, `color-profile`, `theme`, `fn`, `use`, `component`, or `members`, found `banana`
```

This compiles, with a declaration the message does not name:

```contract
atom mention prefix="@" border-radius="4px"
component App
  view
    text "hi @x" markup="markdown" atoms="mention"
```

Expected: the message lists `atom` with the rest.

## Verification

Ran `contract build` on both files: the first prints the message above, the second builds.

## Direction

Add `atom` to the message, beside `decoration`. `test` and the property words `always`,
`after` and `never` are test-file forms and are reasonably left out.

## Related

The grammar reference's side of the same list (`docs/contract-grammar.md:96`) belongs to the
issue for that document. `issues/20261110-byte-order-mark-refused-as-an-invisible-character.md`
quotes this message for another input and says nothing of its list.
