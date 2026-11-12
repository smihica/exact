# The Bun renderer reads each resource's type by position, so a page with a list resource fails to build or boots blank

**Status:** Closed
**Resolution:** The Bun renderer checkpoints declared resources in plan order, including settled list aggregates; members ask independently without seeding aggregates into sources, open resource names appear once, and Markdown and agent reads use declared indices.
**Systems:** host/web-js/checkpoint.js, host/web-js/build.mjs, host/web-js/each.js, host/web-js/markdown.js
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, `__render` writes a page's kept answers with `answers(Resources, types[2], sourceTypes)`
(`host/web-js/build.mjs:648`), and `answers` encodes `Resources[i]` with `types[i]`
(`host/web-js/checkpoint.js:19-26`). `types[2]` is the plan's resources in plan order
(`host/web-js/src/emit.rs:706`). `Resources` is not: a resource over a list (`host/web-js/each.js`) adds no row of
its own, and each of its elements adds one after the plan's rows (`host/web-js/rt.js:543`). Every resource declared
after a list resource, and every element, is encoded with another resource's type or with `undefined`.

`compileReaders` (`host/web-js/markdown.js:20`) and the agent's `r.observation?.(i)` (`host/web-js/agent.js:644`)
index `Resources` by plan position too; neither was run.

## The failure

```
routes nav
  home "/" render=build

shape R
  text: string

component Probe
  state ids: number[] = []
  resource each = item(id) for id in ids as shape R
  resource title = title() as shape string
  view
    column
      text title
      for r in each key=r.text
        text r.text
```

`item` answers `{ text: 'item ' + id }` and `title` answers `'Hello'`, from `app.ts`. An app with no native render
entry renders under Bun, so this is the default build, `bun host/web-js/build.mjs <app>`:

| the plan | result |
|---|---|
| as written (the list empty), or `ids = [1, 2]` | the build fails: `TypeError: v.map is not a function. (In 'v.map((x) => encode(x, t[1]))'` |
| `title` declared first, `ids = [1, 2]` | the build fails: `TypeError: undefined is not an object (evaluating 'Object.keys(t)')` |
| `title` declared first, `ids = [1]` | the build passes; the checkpoint holds `["each","item",[1],[{"r":["i"]}]]`, the element's record encoded as a list |
| `title` alone (control) | the build passes; the checkpoint holds `["title","title",[],"Hello"]` |

In Chrome the third page throws `boot refused while building the view: derive "each": resource "each" (source
"item"): value does not conform to its type`, and `mount` empties the root (`rt.js:1405`): the rendered page goes
blank. Expected: each page builds, and boots showing its title and items.

## Verification

Each row was built at `dd6f5dcd` with the command above (the last three with `--render js`, which is what the
default chooses) and printed the result shown. The third page was opened in Chrome through the agent driver, which
reported `the page never booted` with the exception above. `--render rust` was not run.

## Direction

Give `answers` each resource's own type (a resource can carry its `type_json`), and leave a list resource's
elements out of the kept answers. Read by name, or from the resource, in `compileReaders` and the agent.

## Checkpoint boot

A list's checkpoint holds its aggregate with empty arguments. Each member asks independently at boot and cannot
consume that aggregate as its own answer or pass it to a data module's `kept` hook. The regression tests cover
pending members with and without source arguments, settled aggregates, browser boot with two open element streams,
and a module that caches kept answers for a source with no arguments.
