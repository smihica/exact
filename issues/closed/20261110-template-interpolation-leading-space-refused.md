# A space after `${` in a template is refused as "an indented block"

**Status:** Closed
**Resolution:** Template interpolations accept leading spaces without treating them as indentation and retain exact source spans.
**Systems:** contract/syntax (lexer, template parsing)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

The parser lexes each `${…}` fragment of a template with the file lexer
(`contract/syntax/src/parser/expr.rs:457`, `Lexer::tokenize_at(inner, …)`). The
file lexer treats the leading spaces of every line as indentation
(`contract/syntax/src/lexer.rs:125`, `let indent = line.len() - trimmed.len()`),
and at bracket depth zero an indent deeper than the current level emits `Indent`
(`lexer.rs:178-186`). A fragment is one line whose current level is 0, so a
fragment that starts with a space lexes as `Indent`, the expression, `Newline`,
`Dedent`. The fragment's parser then meets `Indent` where an expression begins.

Trailing space is harmless (`raw.trim_end()`, `lexer.rs:120`), so `${a }`
compiles and `${ a }` does not.

## The failure

```
component App
  state a = 1
  view
    column
      text `${ a }`
```

`contract build` prints:

```
app.contract:5:15 [syntax-expected-expression] expected an expression, found an indented block
 5 |       text `${ a }`
   |               ^
```

Expected: the plan `` text `${a}` `` compiles to. `docs/contract-grammar.md`
("Lexical rules") says: "Templates use backticks and `${expression}`.
Interpolation scanning balances nested braces, strings, and templates." Neither it
nor `docs/contract-for-agents.md` says the expression must touch the `${`, and an
expression takes spaces around it everywhere else (`( a )`, `f( a )`). JavaScript,
whose template syntax this is, accepts `${ a }`. The message is also false: there
is no block on the line.

## Verification

Ran `contract build` (debug build) on the file above and on the same file with
other templates:

| Template | Result |
| --- | --- |
| `` `${ a }` `` | refused as above, 5:15 |
| `` `${ a}` `` | refused as above, 5:15 |
| `` `x ${ a + 1 } y` `` | refused as above, 5:17 |
| `` `${ a > 0 ? "pos" : "neg" }` `` | refused as above, 5:15 |
| ``column testId=`row-${ a }` `` | refused as above, 4:26 |
| `` `${a }` `` | compiles: "1 slots, …, 2 nodes, 0 regions, 546 bytes" |
| `` `${a}` `` | compiles, the same line |

## Direction

Skip a fragment's leading spaces before lexing it, moving the fragment's origin
column by the same amount so spans stay exact (`expr.rs:456`), or have the lexer
take no indentation from a fragment's first line.

## Related

`issues/closed/20260901-template-closes-first-brace.md` is about where a fragment
ends, not its leading space. Nothing in `QUEUE.md` or `docs/agent-pitfalls.md`
mentions it.
