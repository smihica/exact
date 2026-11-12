# `state` is not JSON after a `scrollIntoView` is refused for a key that holds a backslash or a line break

**Status:** Closed
**Resolution:** Refused scrollIntoView reasons use JSON escaping, preserving key text and keeping state replies valid.
**Systems:** runner/src/instance/collection/into_view.rs, runner/src/runner/into_view.rs, runner/src/agent.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

A refused request's status is written into `state.scrollIntoView` by
`IntoViewStatus::json` (`runner/src/instance/collection/into_view.rs:96-101`):

```rust
IntoViewStatus::Refused(why) => {
    out.push_str("\"refused: ");
    out.push_str(&why.replace('"', "'"));
    out.push('"');
```

Only the double quote is handled. The reason carries the key the app asked for (`no row
keyed {text} in the list`, `into_view.rs:844`; `the list holds {text} more than once`,
`into_view.rs:851`), so a backslash or a control character in the key goes into the JSON
string as it is. `record_refusal` (`runner/src/runner/into_view.rs:87-98`) keeps the entry
in a queue of eight, and `agent::state` appends the queue (`runner/src/agent.rs:967`), so
every `state` reply is unreadable until eight later refusals push the entry out or the app
restarts. The key of a request that began is written with `agent::quote`
(`into_view.rs:1008`); only the refusal is not.

The Apple hosts, the Linux host and the wasm web host answer the driver's `state` with
this text (`host/apple/src/agent.rs:48`, `host/linux/src/host.rs:798`,
`host/web/src/host.rs:164`), and the driver parses it (`scripts/agent-wire.mjs:63`). So
`scripts/agent.mjs`'s `state` fails, and with it what
reads state through the driver: `clock until state …` (`scripts/agent-held.mjs:66`) and a
`.test.contract`'s `expect state …` (`scripts/agent-test.mjs:945`). `contract verify` does
not read it. The web JS target is unaffected: it builds the same text
(`host/web-js/list.js:967`) and serializes its state with `JSON.stringify`.

## The failure

```
component App
  state files = ["a.txt", "b.txt"]
  action open(path: string)
    scrollIntoView("files", path)
  view
    list id="files" virtualized=true height=100 estimated-item-height=20
      for f in files key=f
        text f height=20
```

Run `open` with `C:\notes\c.txt`, then read `state`.

Observed: `state.scrollIntoView` is
`[{"list":"files","status":"refused: no row keyed s:C:\notes\c.txt in the list"}]`, and
the whole reply fails to parse (`invalid escape`). A key of two lines fails with `control
character (\u0000-\u001F) found while parsing a string`.
Expected: valid JSON, the reason escaped as RFC 8259 §7 requires, as the keys `c.txt` and
`say "hi"` give.

## Verification

A temporary test booted the source above with `contract::compile`, ran `open` with four
keys and parsed `Runner::into_view_json()` and `agent::state(&runner)` with `serde_json`.
Both parse for `c.txt` and `say "hi"`; neither parses for `C:\notes\c.txt` or a key of two
lines.

On the Mac app (`apps/hello` with a list `feed`, a text input `path` bound to a state, and
a button `find` whose action is `scrollIntoView("feed", path)`):

```
$ bun scripts/agent.mjs macos --app hello 'type path C:\dir\c.txt' 'tap find' state
typed #19 "path" · … · value="C:\\dir\\c.txt"
tapped #15 "find" · …
op 3/3 `state`: unreadable reply: {"epoch":5,"incarnation":1,"clock":0,"slots":{…     (exit 1)
```

The same drive with `web` in place of `macos` printed a `state` that parses.

## Direction

Write the status with `agent::quote` (`"refused: "` and the reason as one quoted string)
and drop the quote replacement.

## Related

`issues/20261110-state-names-a-list-once-per-element.md` is another defect of the same
reply: three of its objects repeat a key, and the reply still parses. Nothing in
`QUEUE.md` or `docs/agent-pitfalls.md` names this one.
