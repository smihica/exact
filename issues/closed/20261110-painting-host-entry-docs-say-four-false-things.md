# Three sentences in the painting host's entry and dev-URL docs are false

**Status:** Closed
**Resolution:** The entry docs name loopback VNC, live dev-URL polling and the shared update store; redirect errors explain the refusal and include Location when supplied.
**Systems:** host/linux/src/app.rs, host/linux/src/fetch.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`. Three statements in the rustdoc of
`app.rs` and `fetch.rs` contradict the code beside them. `app.rs`'s header is
where the host's environment variables are documented, and `Config` is public.

1. `host/linux/src/app.rs:119`, on `Config::vnc`: "Serve the screen over VNC
   at this address (`1` is `0.0.0.0:5900`)". `1` is `127.0.0.1:5900`
   (`host/linux/src/vnc.rs:104`), as the same file's header (`app.rs:27`) and
   `README.md:283` say. The field's doc tells a reader the screen is served
   on every interface.

2. `host/linux/src/fetch.rs:4-7`: "One shot, at process start … the display
   loop's live SSE half waits for a Linux display to verify it on (QUEUE)."
   The same file's `Poller` (`fetch.rs:117-321`) polls the URL every 500 ms
   after first pixel, on the Linux display (`display.rs:611`), the Android
   reader (`canvas.rs:1211`) and the stdio agent (`agent/stdio.rs:42`), as
   `app.rs:15-16` describes. QUEUE has no such entry.

3. `host/linux/src/fetch.rs:12-13`: "Redirects are not followed in v1: the
   error names the limitation and the direct URL fixes it." The error for a
   redirect is the one for any status but 200 (`fetch.rs:377-379`). It names
   neither the limitation nor the `Location` to use.

`app.rs:35` also cites `update.rs` for the agent's temporary update store.
The behaviour is as it says, but this crate has no such file: the code is
`store_dir` in `update/src/client.rs:354`.

## The failure

For 3, a page URL that answers `301`, `302` or `308` with a `Location` (an
origin that redirects `http` to `https`, or a path without its trailing
slash) prints:

```
exact url: http://host/app: HTTP 301; booting the baked plan
```

## Verification

1 and 2 by reading; each line is quoted above with the line that contradicts
it. For 3, a test appended to `fetch.rs` served `301 Moved Permanently`,
`302 Found` and `308 Permanent Redirect` with `Location: /app/` and called
`fetch_app`; each returned `http://<host>/app: HTTP <status>`. That ran on
macOS, where the transport is `NSURLSession`. Not run: the rustls transport
Linux uses, which is built with `max_redirects(0)`
(`vendor/ibex/crates/ibex2/src/transport/rustls_http.rs:157`).

## Direction

Correct 1 and 2 and the `update.rs` pointer in place. For 3, either say in
the error that redirects are not followed and print the `Location` header,
or drop the claim.

## Related

`issues/closed/20261102-production-vnc-authenticates-nobody.md` moved the
`EXACT_VNC=1` default to loopback; sentence 1 is the one place its fix did
not reach. Nothing else in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md`
names these lines.
