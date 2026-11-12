# A dev generation the painting host refuses is downloaded and tried again twice a second

**Status:** Closed
**Resolution:** The poller skips a generation the host holds or refused, as it skips the one running, and chooses the Rust variant before it downloads the plan.
**Systems:** host/linux/src/fetch.rs, host/linux/src/presenter/delivery.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

The URL poller skips a download only when the envelope's identity equals the
last generation the host accepted: `Poller::poll` passes `self.identity` as
`previous` (`host/linux/src/fetch.rs:304`, `313`), and `fetch_changed` returns
early only on that match (`fetch.rs:418`). The presenter sets `dev.identity`
only when `reload_module` succeeds
(`host/linux/src/presenter/delivery.rs:117`). A refused candidate prints a
line and is forgotten (`delivery.rs:140`).

So the next tick, 500 ms after the last answer (`fetch.rs:163`), finds the
same envelope "changed", downloads the plan and the Rust pair again (up to
64 MB and 32 MB), and runs `reload_module` again on the presentation thread,
until the dev server publishes something else. A URL plan refused at boot
starts the poller the same way, with no identity
(`host/linux/src/app.rs:671`).

Three more paths share the cause:

- `fetch_changed` downloads the plan (`fetch.rs:450`) before it looks for a
  Rust variant this binary can run (`fetch.rs:451-462`), so an envelope it
  refuses there costs a plan download on every poll.
- A generation held in `pending_dev` while its module prepares
  (`delivery.rs:131`) is downloaded again each tick for as long as it is held.
- One held for requests in flight (`delivery.rs:132-139`) is downloaded once
  more before it is let go after a second.

## The failure

The Linux display, the Android reader or the stdio agent on a dev URL
(`EXACT_DEV_PLAN=http://…`), and a generation this binary refuses: a plan of
a newer format or schema, a plan whose boot fails, or a Rust pair the binary
cannot take, which `host/android/README.md` ("In a dev loop") says every Rust
edit is on Android.

Observed over 3.2 s against a local server with one fixed generation:

```
bytes that are no plan     plan downloaded 6 times
                           exact url: candidate refused: Plan(BadMagic)                       (six lines)
valid plan, pair refused   plan downloaded 6 times, six lines
envelope refused by fetch  plan downloaded 6 times
                           exact url: invalid Rust executor manifest; keeping the running app (six lines)
module still preparing     plan downloaded 6 times
requests in flight         installed after 1.07 s; plan downloaded 2 times by then, none after
```

Expected: one download and one line for a refused generation; the envelope is
polled, and a payload is fetched again only when its identity changes. The
Apple loader reports a refused generation once and waits for the stream's
next event (`host/apple/Sources/ExactKit/PlanURL.swift:760`).

## Verification

Five tests appended to `host/linux/src/presenter/delivery_tests.rs`. Each
boots a presenter, calls `set_development` with the server's URL and
`poll_development` every 20 ms, and counts the server's requests; they
printed the lines above. Not run: a live `dev.mjs`, or Android.

## Direction

Keep in the `Poller` the identity of the last generation it handed over,
refused or held, and treat an envelope with that identity as unchanged, as
`said` already does for a status. Choose the Rust variant before fetching the
plan.

## Related

"The painting host refuses every dev envelope that offers a `tiered` Rust
variant" is one refusal that repeats this way. Neither fix covers the other.
Nothing in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` names this.
