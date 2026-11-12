# Two Linux tests expect plaintext passwords in agent state after state masking

**Status:** Closed
**Resolution:** Fixed by e59408c0: both tests expect the masked slot in state and the value under reveal, and pass.
**Systems:** host/linux/src/agent/tests.rs, host/linux/src/presenter/hatch_tests.rs
**Severity:** P3
**Author:** Codex (GPT-6 Astra), from the fix lanes of the 2026-11-10 project review
**Date:** 2026-11-11

## What is wrong

At `09cb236d64007f51a668e38926fda6f64dfb6fb5`, two Linux-host tests still expect `"hunter2"` in the agent's `state` reply:

- `a_password_value_is_masked_in_every_reply`, at `host/linux/src/agent/tests.rs:248`.
- `acts_are_queued_run_between_turns_and_journaled_an_input_by_its_length`, at `host/linux/src/presenter/hatch_tests.rs:946`.

Agent state intentionally masks every string matching a live password field (`runner/src/agent.rs:446`). Both failures have the same cause: the assertions predate that masking. They do not establish a failure to store or deliver the input.

## The failure

Each fixture types or delivers `hunter2` into a password input bound to `secret`, then reads agent state. Observed: `slots.secret` is `"•••"`. The assertions expect `"hunter2"` and fail. Expected test behavior: accept the masked agent output and inspect the runner directly if the test needs to prove the private value was stored.

## Verification

With `~/.bun/bin` first on PATH and `CARGO_BUILD_JOBS=5`, ran each alone on macOS with the Linux host's CPU painter:

```sh
cargo test -p exact-linux --lib a_password_value_is_masked_in_every_reply -- --nocapture
cargo test -p exact-linux --lib acts_are_queued_run_between_turns_and_journaled_an_input_by_its_length -- --nocapture
```

The first failed at `agent/tests.rs:248`; the printed state included `"slots":{"secret":"•••"}`. Its earlier assertions for the type reply, tree and node reply passed. The second failed at `presenter/hatch_tests.rs:946`:

```
assertion `left == right` failed
  left: String("•••")
 right: "hunter2"
```

Each run reported `0 passed; 1 failed; 1795 filtered out`. No source or test was changed.

## Direction

Change both state-reply assertions to require the mask and absence of the plaintext. Keep any assertion about the actual stored password on the runner's internal state, outside agent serialization.

## Related

`issues/closed/20260901-web-input-type-attribute.md` concerns password fields in the web DOM. QUEUE's password-masking note records the behavior, not these stale tests. No matching issue or QUEUE entry was found for their failures.
