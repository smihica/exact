# A rendered page shows a `select`'s first option when its options have no `value`

**Status:** Closed
**Resolution:** Both document renderers select the first matching option by explicit value or label, including empty values and labels with nonbreaking spaces.
**Systems:** host/web/src/document.rs, host/web-js/dom.js
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

An option's value is its `value`, else its label. HTML says so (§4.10.10: "the
value of the value content attribute, if there is one, or, if there is not, the
result of get HTML-aware text content"), and so does the kernel
(`kernel/src/control.rs:437-446`, `select_choices`). The live page sets
`select.value`, which the browser matches against the label.

Both writers of a rendered page mark the shown option by its `value` alone:

- the Rust document writer, inside the arm that handles an option's `value` prop
  (`host/web/src/document.rs:792-798`), which an option with no `value` never
  reaches;
- the JavaScript render server's DOM, which compares `o.getAttribute('value')`
  with the select's value (`host/web-js/dom.js:103`).

So no option is `selected`, and the browser shows the first.

## The failure

```
routes nav
  home "/" render=build

component App
  state pick = "b"
  action choose(v: string)
    pick = v
  view
    column
      select value=pick change=choose testId="plain"
        option "a"
        option "b"
      select value=pick change=choose testId="valued"
        option "A" value="a"
        option "B" value="b"
```

Built with `bun host/web-js/build.mjs caltrain --contract app.contract --render
rust` and again with `--render js`, both pages hold (paint attributes left out):

```
<select data-testid="plain"><option>a</option><option>b</option></select>
<select data-testid="valued">…<option selected value="b">B</option></select>
```

Expected `<option selected>b</option>` in the first.

The page is `data-activate="interaction"`, so it waits for the reader (LLP
1048.007 D1). Until then the first select shows `a` while the app's state is
`b`, and choosing `a` fires no `change`. A reader without JavaScript always
sees `a`.

## Verification

Ran the two builds above and read `index.html`. Loaded a page holding the
first select, from each renderer, in the pinned Chromium (156): `select.value`
was `a` with JavaScript off and with it on before any input, and `b` after a
press loaded the runtime and it logged `adopted the document`. The same app
built by the runtime from `shell.html` reported `b`.

## Direction

Decide `selected` for every `option` after its props are read: its `value` prop,
else its label as `select_choices` computes it, compared with the open select's
value. Give `dom.js` the same rule.

## Related

Nothing in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` names a select's
shown option in a rendered page. Every `option` in `apps/` writes `value=`,
which is why no app shows it.
