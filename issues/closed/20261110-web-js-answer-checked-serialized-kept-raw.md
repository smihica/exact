# The web JS target checks an answer as JSON would write it, then keeps the unwritten value

**Status:** Closed
**Resolution:** `checked` returns the answer as `JSON.stringify` writes it (`toJSON`, boxed primitives, non-finite numbers as null, -0 as 0, left-out values gone), made in the same pass that checks it and copying only the parts JSON changes, so the web converts what a native host receives.
**Systems:** host/web-js/ts-data.js, host/web-js/rt.js
**Severity:** P2
**Author:** Claude (Opus 5.5), from GPT-6 Astra's whole-project review (2026-11-10)
**Date:** 2026-11-10

Line references are at `46f3d608`.

## What is wrong

On native hosts a data module's answer crosses as the text `JSON.stringify` makes of it
(`js/src/binary.js:497`), so a `Date` arrives as its ISO string and a non-finite number
as `null`. The web JS target imitates that check: `outside` applies `toJSON` and maps a
non-finite number to `null` before comparing with the shape
(`host/web-js/ts-data.js:143-166`). But `checked` returns the original value, not the one
it checked (`ts-data.js:168-172`), and the converters keep a leaf as it is
(`ts-data.js:37`, `:176`). The runtime's own shape check then sees the `Date` or the
`NaN` (`host/web-js/rt.js:473-474`, `host/web-js/shape.js:10`, `:15`) and fails the
resource, `the answer does not conform to its shape`.

So an answer a native host takes is refused on the web:

- a value with `toJSON` (a `Date`, or a library's decimal or date-time type) where the
  shape says `string`;
- `NaN` or `Infinity` where it says `option<number>` (native: `null`).

## To see it

Confirmed by reading the code only. A source declared to answer `string` returns
`new Date(0)`. On macOS the resource holds `"1970-01-01T00:00:00.000Z"`; on the web
(`bun exact.mjs agent web state`) it fails with the shape refusal above.

## What a fix is

Make `outside` return the normalized value (or normalize once, then check and convert
that), and pass it to `conv` at each place `shaped` is used (`ts-data.js:500`, `:507`,
`:510`, `:527`). Add cases to the cross-target tests (`contract-difftest` or the
conformance run) for a `toJSON` value that fits the shape, one that does not, and a
non-finite number in an option.
