# The Rust render host writes a determinate `progress` with no `value`, so it is indeterminate

**Status:** Closed
**Resolution:** The document writer keeps a `progress` element's `value`, as it keeps an input's.
**Systems:** host/web/src/document.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

A `progress` with a `value` is HTML's `<progress value max>` on the web
(`host/web/src/element.rs:355-361`, `:515`; LLP 1116 D8). The live host sends
`value` as a prop and the page sets `el.value`. The document writer keeps a
`value` prop for an input, a button, an option and a textarea, and drops it for
every other element (`host/web/src/document.rs:792-801`):

```rust
"value" => match element {
    "input" | "button" | "option" => { … attrs.push((name.clone(), Some(value.clone()))); }
    "textarea" => content = Some(value.clone()),
    _ => {}
},
```

`progress` falls in `_`. Its `max` goes through the default arm and is written.

## The failure

```
routes nav
  home "/" render=build

component App
  view
    column
      progress value=0.4 testId="p1"
      progress value=3 max=8 testId="p2"
```

Built with `bun host/web-js/build.mjs caltrain --contract app.contract --render
rust`, the page holds:

```
<progress data-testid="p1" class="c2"></progress><progress data-testid="p2" max="8" class="c2"></progress>
```

Expected `value="0.4"` and `max="8" value="3"`, which `--render js` writes for
the same plan.

HTML §4.10.13: "If the value attribute is omitted, then the progress bar is an
indeterminate progress bar." An undeclared route with nothing for its runtime
to do waits for the reader (LLP 1048.007 D1), so every determinate bar on such
a page is drawn indeterminate until the reader's first press or key, and
always for a reader without JavaScript.

## Verification

Ran both builds and read `index.html`. Loaded the Rust-rendered page in the
pinned Chromium (156): with JavaScript off, and with it on before any input,
both bars reported `position` −1 and matched `:indeterminate`. After a press
loaded the runtime and it logged `adopted the document`, they reported 0.4 and
0.375. The same app built by the runtime from `shell.html` reported 0.4 and
0.375 at once.

A test over `Host::boot` printed the same document, beside a boot batch that
carries `"props":{"value":"0.4"}` and `"props":{"max":"8","value":"3"}`.

## Direction

Add `"progress"` to the arm that writes `value` as an attribute.

## Related

`issues/closed/20261009-aria-range-accessibility-values.md` added the
determinate `progress`; the document writer was not given the case. Nothing in
`QUEUE.md` names it.
