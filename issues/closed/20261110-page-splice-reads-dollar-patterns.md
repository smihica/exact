# Prepared Markdown and the dev checkpoint are spliced into the page as a replacement string, so a `$` pattern in them corrupts the page

**Status:** Closed
**Resolution:** Prepared Markdown and dev checkpoints preserve dollar patterns literally when inserted into HTML.
**Systems:** host/web-js/build.mjs, host/web-js/dev.mjs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at `dd6f5dcd`.

Two places put JSON text into a page with `String.prototype.replace(pattern, replacementString)`:

- `host/web-js/build.mjs:973`, where `tag` holds the build's prepared Markdown sources and their pieces (`:969-970`);
- `host/web-js/dev.mjs:1054`, where `checkpoint` holds the state a reload carries (`:1053`).

In a replacement string `$$` is `$`, `$&` is the matched text, `` $` `` is the text before the match and `$'` is the text after it (ECMA-262, GetSubstitution). The JSON is escaped for `<` only. A Markdown code span that ends in a dollar sign holds the third pattern, and a quoted `'$'` holds the fourth.

## The failure

```contract
component App
  state body = "A shell variable starts with `$`, as in `$HOME`."
  view
    column
      text body markup="markdown" testId="notes"
```

`bun host/web/build.mjs <app> --out <dir>` exits 0 and prints `1 Markdown sources compiled`. `index.html` is 195,012 bytes, twice the 97,517 of the same app with a body that holds no pattern: its `<script type="application/vnd.exact.markdown">` holds a second copy of the document from `<!doctype html>` to the root `div`, and its text is no longer JSON. In the pinned Chromium the page never boots: `boot refused while building the view: SyntaxError: Bad control character in string literal in JSON` (`host/web-js/markdown.js:12`).

The same build with other bodies:

| The body holds | The page's script | In Chromium |
|---|---|---|
| `` `$` `` | a second copy of the head | never boots |
| `'$'` | cut short at an inserted `</script>` | never boots |
| `` `s/x/[$&]/` `` | `<script type="module"` where `$&` was | never boots |
| `` `$$` `` | the source with one `$` | boots; the entry no longer matches the text |
| `$5` and `` `$HOME` `` | the source as written | boots |

A literal Markdown text is spliced the same way and the page doubles, but its pieces are also compiled into `app.js`, so the page boots.

The dev loop: with `bun host/web/dev.mjs` serving the app, a checkpoint whose state held each body was posted to `/__dev/checkpoint` and the next document fetched with `?__exactDev=<id>`. The checkpoint script failed `JSON.parse` for the first three bodies and read one `$` for the fourth.

## Verification

Each build above was run, its `index.html` read, and the page opened in the pinned Chromium by `bun exact.mjs agent web tree`. With a function as the replacement at `build.mjs:973`, a body holding all four patterns built to a page whose script parsed back to the source, and the page booted. The change was reverted.

No app in `apps/` has one of the four patterns in a Markdown source today.

## Direction

Pass a function as the replacement at both sites (`.replace('<script type="module"', () => tag + '<script type="module"')`): a function's result is inserted as it is.

## Related

Nothing in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` names these splices.
