# On iOS a write that finishes while a scan is live leaves the reader session no longer polling

**Status:** Closed
**Resolution:** A write or lock that settles while other claims live now restarts polling once its tag leaves the field, as a read does.
**Systems:** packages/nfc/apple/NfcRegistry.swift
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

A Core NFC session whose delegate takes tags (`didDetect tags`, which the registry implements for
both session kinds) does not look for another tag by itself. Core NFC's header says that
`restartPolling` has no effect on a session whose delegate lacks that method, "as RF polling
restart is done automatically"; for a delegate that has it, new tags arrive only after the call
([`restartPolling()`](https://developer.apple.com/documentation/corenfc/nfcndefreadersession/restartpolling())).

At `dd6f5dcd` the registry restarts polling on four paths: several tags at once (`several`,
`packages/nfc/apple/NfcRegistry.swift:188-191`), a failed connect or read (`retry`, `:229-231`),
a tag type it does not know (`:354`), and a read, once the tag has left the field (`whenGone`,
`:199-208`, called at `:223` and `:318`).

The write path calls none of them. `arrived` hands the tag to `write` and returns (`:215-217`).
When the write ends, `succeed` and `fail` (`:253-266`) end the session if no claim is left;
otherwise they only set the sheet's sentence:

```swift
if self.claims.isEmpty { self.finishIfIdle(c.words.done) } else if let done = c.words.done { self.session?.alertMessage = done }
```

So when a scan claim is still live, the session stays open and detects nothing more. The scan
reads no further tag. A later `write` or `makeReadOnly` joins that session (`claim`, `:50`,
begins one only when none exists) and waits for a tag that is never reported, until Core NFC's
60 second limit ends every claim with `timeout`.

## The failure

On an iPhone:

```
send s = scan()                       // a continuous scan; the sheet is up
send w = write(message=…)             // hold a tag: it is written, w answers
// hold another tag: no `reading` arrives
send w2 = write(message=…)            // hold a tag: nothing happens; AbortError "timeout" later
```

Expected (LLP 1133.035 D6: claims share one reader, and after a claim ends while others live "the
next tag detected" sets the sheet's message): the scan goes on reading after the write, and the
second write is written. A write that fails with a scan live (`overwrite`, `read-only`,
`too-small`) leaves the session the same way.

## Verification

Read, not run. Core NFC needs an iPhone, and QUEUE "@exact/nfc's Apple and Android stages"
records that the registries take no injected tags yet, so no test reaches this path. Every call
of `restartPolling` in the file was traced; the quoted sentences are from
`CoreNFC.framework/Headers/NFCNDEFReaderSession.h` in the iOS 27.2 SDK, and
`NFCTagReaderSession.restartPolling` reads the same.

The device check: run the four lines above on an iPhone and hold a second tag after the write.

## Direction

After a write or lock settles with claims remaining, wait for the tag to leave and restart
polling, as the read path does (`whenGone { tag.isAvailable }`).

## Related

The QUEUE entry above owes the tests; it does not name this.
