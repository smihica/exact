# `contract tamber` lists `gen ts|rust` and `bun exact.mjs tamber --help` lists `gen ts|rust|swift`; the command takes `kotlin` too

**Status:** Closed
**Resolution:** Tamber's compiler and app help list all four binding languages: TypeScript, Rust, Swift and Kotlin.
**Systems:** contract/cli/src/tamber.rs, scripts/help.mjs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`. `contract tamber gen` writes a design's tokens as
bindings for four languages. Two of the three usage texts that describe it are short:

| Where | It lists |
|---|---|
| `contract tamber gen` with no language (`contract/cli/src/tamber_gen.rs:40`) | `ts\|rust\|swift\|kotlin` |
| `contract tamber` (`contract/cli/src/tamber.rs:16`) | `ts\|rust` |
| `bun exact.mjs tamber --help` (`scripts/help.mjs:386`) | `ts\|rust\|swift` |

`docs/contract-for-agents.md:4960` gives all four. An author who reads either help does not
learn that `gen kotlin` writes `TamberTokens.kt`, and one who reads the compiler's does not
learn of `gen swift`, which `docs/reference.md:2583` tells them to run.

## The failure

```
$ contract tamber | grep gen
  contract tamber gen ts|rust <app dir | app.contract | file.tamber> [-o <dir>]   (bindings, in the app's .exact/tamber/)
$ contract tamber gen
usage: contract tamber gen ts|rust|swift|kotlin <app dir | app.contract | file.tamber> [-o <dir>]
$ contract tamber gen kotlin app -o out
out/TamberTokens.kt
$ contract tamber gen swift app -o out
out/tamber.swift
```

Expected: each usage text lists the four languages the command takes.

## Verification

Ran `contract tamber`, `contract tamber gen` and `contract tamber gen <language>` for `ts`,
`rust`, `swift` and `kotlin` on a directory holding this `app.tamber` beside a one-line
`app.contract`:

```
tamber sample

theme
  surface.card = light-dark(#f1f3f5, #16181c)
```

Each wrote its file (`tamber.ts` and `tamber.css`, `tamber.rs`, `tamber.swift`,
`TamberTokens.kt`) and exited 0; `gen java` printed the four-language usage. Ran
`bun exact.mjs tamber --help` in an app `exact new` made: it prints `gen ts|rust|swift`.

## Direction

Make both lines read `gen ts|rust|swift|kotlin`, as `tamber_gen.rs` does.

## Related

`issues/20261110-top-level-help-omits-flags.md` is the same kind of gap in `contract --help`
and in `bun exact.mjs contract --help`; it notes that the `tamber` line there names no `gen`
at all. It does not cover `contract tamber`'s own usage or `bun exact.mjs tamber --help`.
