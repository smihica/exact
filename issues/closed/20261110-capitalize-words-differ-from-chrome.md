# `text-transform: capitalize` finds different words on native hosts than Chrome does on the web

**Status:** Closed
**Resolution:** `capitalize` breaks words at `.` and `:` and keeps word context across UAX #29 WB4 characters within a run, as Chromium does; LLP 1064 D5 states the rule.
**Systems:** kernel/src/text/case.rs, LLP 1064 D5
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

The web hosts leave `text-transform` to CSS, and every other host shows the string
`kernel/src/text/case.rs` makes (LLP 1064 D5). `capitalize` (`case.rs:167-202`) finds a
word's start by a rule of its own: a word character is `char::is_alphanumeric`, `_` or
one of five combining-mark blocks, and a word joins across `'`, `’`, `.`, `:` or `·`
between letters. D5 gives the rule as Chromium's: "a word joins across an apostrophe,
`.`, `:` or `·` between letters, and "3d" stays "3d", as in Chromium". CSS Text 3 §2.1
leaves the definition of a word to the browser (it suggests UAX #29), so Chrome is the
reference because D5 names it and because the web host shows Chrome's string.

Chrome 156 differs in two ways:

- It does not join across `.` or `:`, though UAX #29 does.
- It keeps a word together across the characters UAX #29's rule WB4 ignores: a soft
  hyphen, a zero-width joiner, a word joiner, a variation selector, and a combining mark
  outside the five blocks (U+0483). The kernel starts a new word after each.

So one app shows different text, of a different width, on the web and on a phone.

## The failure

```
text text-transform="capitalize" "u.s.a. at 9 a.m."
```

| Source | Chrome 156 | Kernel |
|---|---|---|
| `u.s.a. at 9 a.m.` | `U.S.A. At 9 A.M.` | `U.s.a. At 9 A.m.` |
| `example.com` | `Example.Com` | `Example.com` |
| `re:invent` | `Re:Invent` | `Re:invent` |
| `auto` U+00AD `matic` | `Auto` U+00AD `matic` | `Auto` U+00AD `Matic` |
| `hello` U+200D `world` | `Hello` U+200D `world` | `Hello` U+200D `World` |

Of 50 strings tried, 31 agree, among them the apostrophe, `·`, `_`, digits, a hyphen
and a no-break space. `uppercase` and `lowercase` agree on all six tried.

Not part of this issue: Chrome titlecases the first letter with the simple mapping, which
leaves `ß` and the ligatures as they are, where `titlecase` (`case.rs:208`) applies the
full one. D5 chooses `ß` → `Ss` by name, and CSS Text 3 §2.1 asks for the full mapping.

## Verification

- A temporary kernel test printed `TextTransform::Capitalize.apply(source, "")` for the
  50 strings.
- Headless Chrome 156 gave `innerText` of a `span` with `text-transform: capitalize` for
  the same strings. For each row above, the span's width in 40px Times equals the width
  of Chrome's string written out and differs from the kernel's: `u.s.a. at 9 a.m.` is
  280.11 px wide, `U.s.a. At 9 A.m.` 257.84.
- No native host was run. Each shows the kernel's string (`Arena::text_runs`).

## Direction

Drop `.` and `:` from `joiner` (`case.rs:190-192`), and carry the previous word
character across the characters WB4 ignores (Format, Extend and ZWJ) in place of the
list of mark blocks. Correct D5's sentence, the comment at `case.rs:162-166` and the
test at `case.rs:328`, which pin `E.g.`.

## Related

`issues/closed/20260927-capitalize-across-runs.md` fixed the context carried between
runs against live Chrome. It did not measure `.`, `:` or ignorable characters.
