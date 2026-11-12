# On Hermes, `fetch` gives a string or `URLSearchParams` body no `Content-Type`, so the server sees none on Linux and a form type for every POST on Apple

**Status:** Closed
**Resolution:** Hermes fetch supplies Fetch's string and form Content-Type defaults without replacing explicit headers; Apple's untyped byte POST default is documented.
**Systems:** js/src/prelude.js (`request`), docs/reference.md
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd` the prelude's `fetch` sends only the headers the caller named (`js/src/prelude.js:593`) and the body's text or bytes (`prelude.js:597-602`, `667`). Fetch's Request constructor also appends the type the body brings when `init.headers` names none (https://fetch.spec.whatwg.org/#concept-bodyinit-extract): `text/plain;charset=UTF-8` for a string, `application/x-www-form-urlencoded;charset=UTF-8` for a `URLSearchParams`, none for bytes. The same prelude does this for `new Response(body)` (`js/src/binary.js:701`, `716`), and the web JS target hands the body to the browser's `fetch` (`host/web-js/ts-fetch.js:83` keeps a `URLSearchParams` as one).

What the transport under the executor then does differs by host. The rustls transport the Linux host uses adds nothing. URLSession adds `Content-Type: application/x-www-form-urlencoded` to any POST that has a body and no type, and nothing to a PUT or PATCH.

## The failure

```ts
await fetch("https://fixture.exact.test/token", {
  method: "POST",
  body: new URLSearchParams({ grant_type: "authorization_code", code: "a b" }),
});
```

The executor hands the runner `POST … headers=[] body="grant_type=authorization_code&code=a+b"`. The `Content-Type` a server receives when the app names none:

| Request | Chrome 156 | Apple host | rustls transport (Linux) |
| --- | --- | --- | --- |
| POST, `URLSearchParams` | `application/x-www-form-urlencoded;charset=UTF-8` | `application/x-www-form-urlencoded` | none |
| POST, string | `text/plain;charset=UTF-8` | `application/x-www-form-urlencoded` | none |
| POST, bytes | none | `application/x-www-form-urlencoded` | none |
| PUT, string | `text/plain;charset=UTF-8` | none | none |

So a token endpoint that reads the body by its type (`packages/oauth/test/fixture.ts:264` refuses one "not a form") answers 400 to a Linux build and 200 to the web and Apple builds, and a server that parses by type reads a string or bytes POST from an Apple build as a form.

## Verification

Ran the snippet, and one with `body: "hello"`, as data sources in a temporary `exact-js` test: both requests reached the runner with `headers=[]`. For the hosts, ran temporary tests that send a `Request` with a body and no headers to a loopback listener and print the bytes received: through `exact-apple`'s executor core on the platform transport (macOS 27, CFNetwork 3896), and through `RustlsHttpTransport::send` in ibex2 built with `portable-http`. The Chrome column is the pinned Chromium (156.0.8078.4) against a local server. Not run: the Windows and Android hosts.

## Direction

In `request`, when the body is a string or a `URLSearchParams` and the headers hold no `content-type`, append the type Fetch gives it, as `binary.js` `initialized` does for a `Response`. That makes the first two rows and the last agree on every host. A bytes POST on Apple still gets URLSession's form type, where Fetch sends none; say so in the `fetch` row of `docs/reference.md`, which lists a string body with no word on its type.

## Related

`docs/reference.md` states "no Content-Type is added" only for `exactBodyFrom`. `packages/oauth/index.ts:764` sets the header itself, so the package does not show it. No issue or QUEUE entry names it.
