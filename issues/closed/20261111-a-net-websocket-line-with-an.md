# A net.websocket line with an http origin is accepted and admits nothing

**Status:** Closed
**Resolution:** Refused at build (Charlie, 2026-11-10): a plain net.websocket line whose scheme is not ws or wss fails to parse in exact-grants and in ibex2 (local patch 28), as its subdomain and scheme-wide forms did; the shared corpus (host/web/tests/fixtures/grants.json) holds the lines. An app writes the socket's origin itself.
**Systems:** grants, vendor/ibex ibex2 grant, host/web-js/build.mjs, bake
**Author:** Claude (Fable 5.2), from expo/exact#54's thread
**Date:** 2026-11-10

## What is wrong

`export const grants = 'net.websocket http://127.0.0.1:4791'` builds on every host, and a
socket to `ws://127.0.0.1:4791` is refused: the line is parsed as a socket grant for an
`http:` origin (`grants/src/lib.rs`, the plain `net.fetch` | `net.websocket` arm), which no
socket has. The subdomain form of the same line is refused at build ("a subdomain
`net.websocket` line needs `ws` or `wss`"); the plain form says nothing.

It is why an app names a deployment twice. A backend gives one URL (`CONVEX_URL=http://…`,
which `npx convex dev` writes to `.env.local`), a grant may be written from that setting
(`const URL = import.meta.env.CONVEX_URL ?? '…'`, then `` `net.websocket ${URL}` `` builds),
and the socket line needs the same host under another scheme. expo/exact#54's reporter keeps
"a second setting next to the `http://` one"; `apps/mosaic` writes both literals.

## To see it

The repro of expo/exact#80 with its grants line's `ws:` changed to `http:`:
`bun exact.mjs agent web "clock data" "state feed"` shows `status "error"`, and the build
printed no warning.

## What a fix is

One of two, and it is a decision about the grant grammar (LLP 1016 D6):

- A `net.websocket` line whose origin is `http:` or `https:` grants the socket at that host as
  `ws:` or `wss:`, as the WebSocket constructor takes such a URL
  (<https://websockets.spec.whatwg.org/#dom-websocket-websocket>, which turns an `http` scheme
  into `ws` and `https` into `wss`). One setting
  then names a deployment.
- Or the line is refused at build, as its subdomain form is.

The grammar is read in four places that must agree: `grants/src/lib.rs`, ibex2's parser under
`vendor/ibex`, the web JS build's `normalizeGrants` with `host/web-js/admission.js`, and the
bake's reach (`bake/src/reach.rs`).
