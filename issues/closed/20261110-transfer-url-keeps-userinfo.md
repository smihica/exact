# A background transfer to a URL with a user name and password keeps them in its record and its refusal

**Status:** Closed
**Resolution:** Admission refuses a record URL with a user name or password, every such refusal names the URL through `grants::redacted` (no userinfo, query or fragment), and the Apple carrier records the URL without user and password.
**Systems:** runner/src/runner/modules/admit.rs, host/apple/Sources/ExactKit/Transfers.swift
**Severity:** P2
**Author:** Claude (Opus 5.5), from GPT-6 Astra's whole-project review (2026-11-10)
**Date:** 2026-11-10

Line references are at `46f3d608`.

## What is wrong

A request URL's userinfo (`https://alice:secret@example.com/file`) passes admission and is
kept where a query is not:

- Admission (`runner/src/runner/modules/admit.rs:1009-1012`) matches the URL's origin
  with `crate::grants::admitting`, whose `exact_grants::origin_of` parses it with
  `url::Url` and keeps only scheme, host and port (`grants/src/lib.rs:597-617`), so the
  credentials do not stop it. A grant line spelt with userinfo is refused
  (`grants/src/lib.rs:863-871`); a request URL is not.
- A refusal is meant to name the URL "without its query or fragment"
  (`admit.rs:994`); it cuts at `?` and `#` only (`admit.rs:1012`), so a
  `grant-missing (requests[0].url=https://alice:secret@… needs net.fetch)` carries the
  password.
- The carrier clears the query and fragment of the URL it records
  (`host/apple/Sources/ExactKit/Transfers.swift:174-176`) and keeps the rest, user and
  password included (`:188`). That URL is written to the registration's `record.json`
  (`:133`) and returned in every `backgroundFetchRegistration` answer (`:209`), although
  `shown` says a package reads "never a header or a query" (`:201`).

`issues/closed/20261006-auth-userinfo-disclosure.md` fixed the same class for auth
URLs by refusing userinfo before grants are checked.

## To see it

Confirmed by reading the code only. On iOS or macOS, with `net.fetch https://example.com`
granted, `backgroundFetch({ id: 'x', requests: [{ url: 'https://alice:secret@example.com/file' }] })`
starts; `backgroundFetchRegistration({ id: 'x' })` answers a record whose `url` is
`https://alice:secret@example.com/file`, and the same string is in the record's file on
disk. Without the grant, the refusal's text holds it.

## What a fix is

Refuse a request URL with a user name or password at admission, without echoing it (as
the auth module does), and redact the refusal's URL through one function that drops
userinfo, query and fragment. The carrier's `shownURL` should also clear `user` and
`password`, so a record never holds them whatever reaches it. A browser's `Request`
refuses such a URL too (Fetch, <https://fetch.spec.whatwg.org/#dom-request>, step "If
parsedURL includes credentials, then throw a TypeError").
