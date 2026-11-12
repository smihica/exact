# The agent's file picker refuses an SVG, BMP or ICO file for an input that accepts images

**Status:** Closed
**Resolution:** Agent pickers accept SVG, BMP and ICO by image MIME type, deliver those types on the web, and recognize PDF in the runner.
**Systems:** `runner/src/runner/picker.rs`, `host/web-js/files.js`
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

Under the agent a `showPicker` is held and answered with `type @t <path>` (LLP
1069.002 D9). `check_pick` takes the answer only when `accept` admits each file, "what
the real picker would have let the person choose"
(`runner/src/runner/picker.rs:235-271`). It types a file by its name with `mime_for`,
"as a browser guesses it" (`picker.rs:156-183`). That table has no entry for `svg`,
`bmp` or `ico`, so each is `application/octet-stream`, which neither `image/*` nor the
file's own type matches.

The real picker takes them. The Mac's open panel is given `allowedContentTypes` from
`UTType(mimeType:)` and `.image` (`host/apple/Sources/ExactKit/Picker.swift:304-305`,
`363`), and a pick is typed with `UTType(filenameExtension:)?.preferredMIMEType`
(`Picker.swift:141`, `176`).

The web JS target's copy of the table (`host/web-js/files.js:101-102`) lacks the same
three. It has `pdf`, which the runner's lacks, so the two disagree on `report.pdf`
under `accept="application/pdf"`.

## The failure

```
input type="file" accept="image/*" id="one"
```

`showPicker("one")` under the agent, answered with `/a/logo.svg`:

```
@1 (pick): logo.svg (application/octet-stream) is not among accept="image/*"
```

The same for `/a/scan.bmp` and `/a/icon.ico`. It is also the answer to `logo.svg`
under `accept="image/svg+xml"` and to `scan.bmp` under `accept="image/bmp"`, both of
which compile: no file answers an input that names only SVG. `photo.png` under
`image/*` and `scan.tiff` under `image/tiff` are taken.

Expected: each is taken, with its own type.

## Verification

- A temporary test in `contract/cli/tests/it/` compiled the input with each `accept`,
  held the picker with `Runner::hold_picker("one")` and answered it with
  `Runner::answer_hold`. It printed the refusals above and `Ok` for the PNG and TIFF.
- On macOS 27.0.1, `UTType(filenameExtension:)` for `svg`, `bmp` and `ico` printed
  `public.svg-image image/svg+xml`, `com.microsoft.bmp image/bmp` and
  `com.microsoft.ico image/vnd.microsoft.icon`, each conforming to `.image`.

The web JS target's table and the `pdf` difference were read, not run.

## Direction

Add `svg`, `bmp`, `ico` and `pdf` to `mime_for`, and the three image types to the
table in `files.js`.

## Related

`QUEUE.md`, "A tap on a file `input` holds no picker under the agent" is about how the
hold starts. No issue names `mime_for`.
