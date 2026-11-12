# A hosted text field is told the previous plan's font file after a dev reload

**Status:** Closed
**Resolution:** Hosted field font files are cached by the owning text engine, so reloads and fresh boots resolve each plan's own faces.
**Systems:** host/linux/src/fields.rs, Android reader (LLP 1115.001 D8)
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`fields::family` (`host/linux/src/fields.rs:180-220`) resolves a hosted
field's family to the font file the shell's `EditText` loads, and caches the
answer in a thread-local map keyed by `(font_family, font_weight, italic)`
(`fields.rs:73`, `184-187`, `218`). `font_family` is an index into the booted
plan's font stacks, so a key means something only for one plan. Nothing
clears the map. After a reload on the same thread, a field whose stack index
now names another family is told the file cached for the old one.

Only Android hosts fields (`fields::host` is called from
`host/linux/src/canvas/field.rs:148`), and Android replaces a running plan
only in the dev loop (`poll_development`, `host/linux/src/canvas.rs:1211`;
that host has no update store, `apps/caltrain/android/build.rs:26`). The text
the host paints itself resolves through the new plan's engine, so after such
a reload a field's typeface disagrees with the text around it until the app
is started again.

## The failure

An app with one field, open on Android in a dev loop:

```
component App
  view
    column width=320 height=200
      input testId="f" font-family="Courier New"
```

Change the family to `"Georgia"` and save. The family string the reader is
told for the field (`reader_fields`, the statement's third string), on macOS
under the test stand-in for Android's shell:

- observed: `/System/Library/Fonts/Supplemental/Courier New.ttf#0`
- expected: `/System/Library/Fonts/Supplemental/Georgia.ttf#0`

## Verification

A test appended to `host/linux/src/presenter/hosted_tests.rs`, using its
`shell`, `boot`, `settle` and `told` helpers: boot the first plan, read the
statement, call `reload_module` (what the dev loop calls) with the second
plan, read it again, then boot the second plan afresh on the same thread. A
second test boots the Georgia plan first, on a thread of its own.

```
cargo test -p exact-linux --lib face_cache_verify -- --nocapture
reload Courier New -> Georgia: field told ".../Courier New.ttf#0" before, ".../Courier New.ttf#0" after
fresh boot of the Georgia plan on the same thread: field told ".../Courier New.ttf#0"
control, Georgia plan booted first: field told ".../Georgia.ttf#0"
```

Not run: an Android device, where the `EditText` then sets the stale file.

## Direction

Clear the map when a host boots a plan, or key it by the stack's family names
instead of the plan's stack index.

## Related

QUEUE "A fitted text and text flow are not measured again when a typeface
arrives after the first layout" is about a face arriving late within one
plan. QUEUE "Android native fields omit tabular-digit styling" is about what
`FieldFace` carries. Neither is a cache that crosses plans.
