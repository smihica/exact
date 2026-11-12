# A harness reply that streams for more than 120 seconds fails with "stream broke: timeout: receive response"

**Status:** Closed
**Resolution:** Streaming replies may exceed 120 seconds; idle reads fail, and leaving a request stops its reader even when a partial line keeps arriving.
**Systems:** apps/harness/data (providers/mod.rs)
**Severity:** P1
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

`post` builds its agent with `timeout_recv_response(Some(IDLE))`, `IDLE` being 120 s (`apps/harness/data/src/providers/mod.rs:119,373-377` at `dd6f5dcd`). The module's header gives the intent: "a stream silent for two minutes fails".

ureq 3.4.0 documents that setting as "Max duration for receiving the response headers, but not the body" (`ureq-3.4.0/src/config.rs:734`). Its code does more. Each body read asks `next_timeout(Timeout::RecvBody)`, which checks the phase before it too: the instant the headers arrived plus `recv_response` (`ureq-3.4.0/src/timings.rs:47-68,153-173`). With no `timeout_recv_body` set that is the only deadline, so the whole body must arrive within 120 s of the headers however steadily it streams. `CancelTransport::await_input` passes the deadline through (`mod.rs:86-108`), the reader thread's `read_line` fails (`mod.rs:403-407`), and the turn ends.

Both providers go through `post_sse` (`anthropic.rs:72`, `openai.rs:81`), so this bounds every reply from Anthropic, OpenAI, OpenRouter and Ollama. The Anthropic request allows 8,192 output tokens (`anthropic.rs:60`), which is 136 s at 60 tokens a second; the OpenAI-compatible request sets no limit, and a local model is slower.

## The failure

A loopback server answers `/v1/chat/completions` (the Ollama route, `OLLAMA_HOST=127.0.0.1:<port>`) with one content token a second for 135 s, then `finish_reason: "stop"` and `[DONE]`.

- Observed: at 120 s the turn ends. The transcript keeps the 119 tokens streamed so far and adds the error entry `stream broke: timeout: receive response`. A tool call whose arguments were still streaming, a long `write_file` for one, is dropped and never runs (`agent.rs:279-286`).
- Expected: the reply completes. Only 120 s without a line fails.

## Verification

Ran that server against `Harness::with_options(Options::offline(None))` as an integration test beside `tests/http.rs` (`setModel("ollama:fake")`, `submit`, then wait for `busy` to clear). It printed

```
turn ended after 120 s; last entry: error Some("stream broke: timeout: receive response")
tokens shown: 119 of 135; HTTP_READERS=0
```

A second server sent one event and went silent with the socket open. The turn ended at 120 s with the same message, not with `post`'s own "no data from the server for 120 s" (`mod.rs:431-435`): ureq's deadline is never later than that branch's, so the branch does not run today.

## Direction

Drop `timeout_recv_response` and have `CancelTransport::await_input` bound each wait itself: when ureq gives no deadline, fail after `IDLE` without input. That is the idle rule for the headers and for every body read, and it ends the reader thread, which the channel loop's timeout alone would leave blocked on a silent socket.

## Related

The closed harness issues (`issues/closed/20261006-harness-*`) concern tools and approvals, not the transport. Nothing in `QUEUE.md` names it.
