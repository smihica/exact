# On the web, aborting a language-model response before its input is admitted hangs the session

**Status:** Closed
**Resolution:** `beginBrowser` returns without admitting the input when the response ended while the browser's session was built or the input measured, so an aborted or cancelled response no longer holds the session.
**Systems:** packages/language-model/web/index.js
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, `beginBrowser` (`packages/language-model/web/index.js:312`) awaits twice before it admits the input: `await platform(s)` (line 316), which builds the browser's session when the module holds none or a stale one, and `await measure.call(session, input, options)` (line 324, the browser's `measureInputUsage`). It checks `r.state` before the first await and after neither. If the response ends during one of them, `end` clears `s.responding` and starts the session's next call, and then `beginBrowser` resumes and calls `admitInput` (line 325), which sets `s.responding = r` unconditionally (line 240). The loop at line 329 returns because `r` is no longer `generating`, and nothing clears `s.responding` again: `end(r, …)` returns at once for a response that has already ended (line 177).

From then on `nextIn` (line 197) starts nothing on that session. `prompt`, `appendLanguageModel` and `cloneLanguageModel` never answer, a `promptStreaming` response stays `generating` with no text, and `languageModelSession().responding` reads true, until the session is destroyed.

The agent path admits synchronously after its state check (lines 251 to 252), and Apple's implementation checks after its awaits: `guard r.state == "generating", m.admitInput(r, extra: measured) else { return }` (`packages/language-model/apple/LanguageModelApple.swift:196`).

## The failure

A stand-in `LanguageModel` whose `measureInputUsage` takes 20 ms; then:

```js
const s = await impl.createLanguageModel({}, { origin: 'app' });
const r = await impl.promptStreaming({ session: s.id, input: 'two' }, {});
await impl.abortLanguageModelResponse({ response: r.id });   // a Stop pressed at once
await impl.prompt({ session: s.id, input: 'three' }, {});    // never settles
```

Observed: the response reads `aborted`; 100 ms later `languageModelSession` reads `"responding":true`; the next `prompt` has not settled after 2 s. Expected (LLP 1133.016 D6): "Every end commits the turn before the session's next call is admitted", so the next prompt runs, as a `prompt()` after an aborted one does in the browser's own API.

The window is also open without a measure, while `LanguageModel.create` rebuilds the session (a clone's first prompt, 40 ms in the run below), and the same happens when the runtime cancels a `prompt` whose call was let go (`cancel(request)`, line 530), as when a mutation is superseded.

## Verification

Ran the unmodified module under Bun with stand-ins that supply the Prompt API's async methods. The sequence above printed a completed control prompt, then `session 100 ms after the abort: {"id":"session-1","contextUsage":21,"contextWindow":4096,"responding":true}` and `next prompt: "NOT SETTLED after 2 s"`. A cloned session whose first response was aborted 5 ms into a 40 ms `create`, and a `prompt` cancelled 5 ms into its measure, each left `"responding":true` and the next prompt unsettled; `destroyLanguageModel` still answered. Not run in a browser with the Prompt API.

## Direction

Return from `beginBrowser` when `r.state !== 'generating'` after each await that precedes `admitInput`, so `admitInput` is reached only by a response that still holds the session.

## Related

QUEUE "The web's Prompt API tool loop is held only by a stand-in" is about the tool loop's coverage, and "The language model's web create starts a download for any origin" is about admission; neither covers the queue.
