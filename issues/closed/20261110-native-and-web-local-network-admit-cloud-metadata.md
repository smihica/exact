# The hosts that admit a local-network fetch still admit the cloud metadata addresses

**Status:** Closed
**Resolution:** `exact_grants`, ibex2 and the web refuse 169.254.169.254, its IPv4-mapped spellings and fd00:ec2::254 under a `local-network` grant, and the shared grant fixture holds all three to it.
**Systems:** grants/src/local.rs (`local_v6`), vendor/ibex ibex2 grant (`is_local_network`), host/web/navigation.js (`localV4`), the JS target's lifted admission, grants/tests/parity.rs, host/web/tests/fixtures/grants.json
**Severity:** P2
**Author:** Claude (Opus 5.5), from Grok 4.7's whole-project review (2026-11-10)
**Date:** 2026-11-10

Line references are at `46f3d608`.

## What is wrong

`issues/closed/20261102-local-network-grant-includes-cloud-metadata.md` was closed by
excluding `169.254.169.254` in `exact_grants::is_local_network` (`grants/src/local.rs:54-57`).
No host opens a socket through that function. Its acceptance, "a local-network fetch of
the metadata address is refused", is not met:

- **Native.** Apple, Linux and the other native hosts parse grants with
  `ibex2::grant::GrantSet::parse` (`host/linux/src/executor.rs:52`,
  `host/apple/src/executor_core.rs:1200`). Each fetch hop is admitted by
  `admit_hop_grant` (`vendor/ibex/crates/ibex2/src/stdlib/fetch.rs:495`) through
  `boundary::admit`, which asks `GrantSet::permits`. Its `FetchLocalNetwork` and
  `WebSocketLocalNetwork` arms (`vendor/ibex/crates/ibex2/src/grant.rs:221-228`) call ibex2's
  own copy, `is_local_network` (`grant.rs:613`). Its IPv4 rule (`:615`) is
  `is_private() || is_link_local() || loopback && is_loopback()`, with no exclusion, and
  an IPv4-mapped address is judged by that rule.
- **Web and the JS target.** `localV4` (`host/web/navigation.js:1226`) admits every
  `169.254.x.x`. `admittingGrant` (`:1417-1423`) uses it, and `host/web-js/build.mjs` lifts
  the same section into the JS target's admission. On the web, grants are parity rather
  than a boundary (the page can call the browser's `fetch`), so the web half is a
  disagreement rather than a hole.

No copy excludes the IPv6 metadata address, `fd00:ec2::254`, which AWS serves on Nitro
instances when the instance enables the IPv6 endpoint
(<https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/instancedata-data-retrieval.html>).
It is unique-local (`fc00::/7`), and all three admit `fc00::/7` whole:
`grants/src/local.rs:64`, `vendor/ibex/crates/ibex2/src/grant.rs:624` and
`host/web/navigation.js:1233`. `grants/src/local.rs`'s test admits `[fc00::1]` and
`[fd12:3456::1]` (`:106-107`) and names no exception. So even `exact_grants`, which the closed
issue fixed for IPv4, admits it.

The prompt does not catch it. `Reach::static_risk` puts the metadata address back on the
prompt list (`runner/src/local_network.rs:146-162`), but `Reach::refusal` returns `None` for
a call the app's own code makes (`:133`), so an app's request goes out.

The parity tests did not notice because nothing asks them to. `grants/tests/parity.rs`
(the two Rust parsers) and `host/web/request-refusal.test.mjs` (the web) both read
`host/web/tests/fixtures/grants.json`, whose
local-network case (`:487-489`) admits `169.254.3.4` and never names `169.254.169.254`.

The render host is not affected: it drops `local-network` lines
(`host/render/src/source.rs:41`).

## To see it

Confirmed by reading code, and by running the web predicate:

```
bun -e "const m = await import('./host/web/navigation.js'); console.log(m.isLocalNetwork('169.254.169.254', false), m.isLocalNetwork('[::ffff:a9fe:a9fe]', false))"
true true
bun -e "const m = await import('./host/web/navigation.js'); console.log(m.isLocalNetwork('[fd00:ec2::254]', false))"
true
```

On a native host, an app whose grants include `net.fetch local-network` and that fetches
`http://169.254.169.254/latest/meta-data/` sends the request. On a cloud VM serving IMDSv1
there (a Linux host run on one, for example), the answer is the instance's credentials.
`http://[fd00:ec2::254]/latest/meta-data/` is sent the same way on an instance with the
IPv6 endpoint on.

## What a fix is

Exclude `169.254.169.254` and its IPv4-mapped forms in ibex2's `is_local_network` and in
`localV4`'s caller, as `grants/src/local.rs` does. Exclude `fd00:ec2::254` in all three
copies, `grants/src/local.rs` included, and keep the rest of `fc00::/7`. Better, make ibex2
call one predicate rather than keep a mirror; whether the vendored crate may depend on
`exact-grants` is the decision. Add `http://169.254.169.254/`,
`http://[::ffff:169.254.169.254]/`, `ws://169.254.169.254/` and `http://[fd00:ec2::254]/` as
refused to the fixture's local-network case, beside its admitted `http://[fd00::5]:81/`
(`:488`), so the parity test holds every copy to the rule.
