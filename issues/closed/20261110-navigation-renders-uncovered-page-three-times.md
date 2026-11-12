# A navigation to a page the kernel-free writer does not cover renders it three times

**Status:** Closed
**Resolution:** A navigation whose tree needs a kernel renders twice, as a buffered request does, without retrying the direct writer.
**Systems:** host/render/src/serve.rs, host/render/src/direct.rs, host/render/src/lib.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

A JavaScript page is written from the runner's instance tree, with no kernel,
where the fold covers the tree. A tree holding a relative length (`rem`, `em`)
or a virtualized list is not covered, and "the render then renders again with
a kernel … That costs double, and only in those cases" (LLP 1048.004 D4).

On the server's navigation path it costs triple:

1. `document` calls `direct::render_js` (`host/render/src/serve.rs:1224-1239`),
   which settles the page, finds the tree uncovered and returns `Ok(None)`
   (`host/render/src/direct.rs:105-112`).
2. `document` then calls `render_as` (`serve.rs:1243-1251`), which is
   `render_with(…, Projection::Auto)` (`host/render/src/lib.rs:199-218`). It
   settles the page for the kernel-free writer again and fails the same way.
3. Only then does it render with a kernel (`lib.rs:314-327`).

Each settle boots a runner and asks every source the page reads, so the
reader waits three times as long and the sources answer three times.

## The failure

```
routes nav
  tab rem "/rem" render=request
  tab px "/px" render=request
  notfound render=build

component Site
  resource cards = cards() as shape string[]
  view
    column
      for e in stack(nav) key = e.id
        column
          if e.name == "rem"
            for c in cards key = c
              text c font-size="1.5rem"
          if e.name == "px"
            for c in cards key = c
              text c font-size="24px"
```

with a `cards` source that answers after 300 ms, served by the render host
over a JS-target build:

| Request | Time | "with a kernel" log lines |
|---|---|---|
| `GET /px` | 0.31 s | 0 |
| `GET /px`, `Sec-Fetch-Dest: document` | 0.31 s | 0 |
| `GET /rem` | 0.63 s | 1 |
| `GET /rem`, `Sec-Fetch-Dest: document` | 0.94 s | 2 |

Expected: the navigation to `/rem` takes the two renders the buffered request
takes.

## Verification

A temporary test in `host/render/tests/it` served the build with
`exact_render::Server` and a data source whose answer sleeps 300 ms. `curl -w
'%{time_total}'` printed the times above (the plan held other routes too), and
the server logged `render /rem: with a kernel (not written without a kernel: a
relative length (rem, em))` once for the buffered request and twice for the
navigation, then `render /rem 200 934.8ms … flushed`.

## Direction

When `render_js` has settled a page and found its tree uncovered, have the
caller render with `Projection::Kernel`: return that case apart from "the plan
is never written without a kernel", for which `Auto` already goes straight to
the kernel.

## Related

On a `paint=boot` route the same `Ok(None)` also leaves the boot document in
the page; that is a separate issue. Nothing in `issues/` or `QUEUE.md` names
the third render; `issues/closed/20261006-render-corpus-refuses-rem-outside-gate.md`
is about the corpus.
