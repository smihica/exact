# `contract vocab --json h1` says `h1` is not a tag and exits 1, where `contract vocab h1` explains it

**Status:** Closed
**Resolution:** Vocab shares heading descriptions across text, JSON and full listings, and h1 through h6 lookups succeed in both formats.
**Systems:** contract/cli/src/vocab.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

The parser reads `h1` to `h6` as headings, so no vocabulary table lists them (LLP 1116.017
D1). `contract vocab` answers for them in its text form with a special case at the top of
`one_text` (`contract/cli/src/vocab.rs:385-393`). The JSON form, `one_json`
(`vocab.rs:305-336`), has no such case: it finds no entry, answers with the compiler's
refusal and reports the name as unknown, so the process exits 1. One command gives two
answers for one name, and the JSON one is false: `h1 "Title"` compiles.

## The failure

```
$ contract vocab h1
h1: a heading of level 1: `h1 "Title"` is `text "Title" role="heading" aria-level=1`; a level the app computes is written that way
$ echo $?
0
$ contract vocab --json h1
{
  "name": "h1",
  "refused": "`h1` is not a tag, an attribute, a standard function, a recipe or a pattern; `bun exact.mjs docs <words>` prints the guides' section on a topic"
}
$ echo $?
1
```

Expected: the JSON form knows the name (exit 0) and says what the text form says. With
several names (`contract vocab --json h2 text`) the one heading makes the whole call exit 1.

## Verification

Ran the commands above with `target/debug/contract`; the output is quoted as printed.
`contract vocab --json h2 text` printed the refusal for `h2`, the entry for `text`, and
exited 1. The whole-vocabulary dump (`contract vocab --json`, no name) has no `h1` either.

## Direction

Answer `h1` to `h6` in one place that both forms read, so `one_json` returns a known entry
with the same sentence.

## Related

None found in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` for `vocab --json`.
