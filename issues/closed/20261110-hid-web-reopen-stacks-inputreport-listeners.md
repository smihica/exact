# On the web, each reopen of a HID device adds another input-report listener that is never removed

**Status:** Closed
**Resolution:** Web HID removes its listener when the shared open ends, preserves final report batches, orders device operations, and releases disposed handles.
**Systems:** packages/hid/web/index.js
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, `lease` (`packages/hid/web/index.js:50`) adds an `inputreport` listener to the `HIDDevice` whenever the device has no entry in `shared` (line 55). `close` drops that entry with the last lease and closes the device (line 130), and `forget` drops it too (line 138), but neither removes the listener: it is an anonymous closure nothing keeps a reference to. The next `open` of the same device finds no entry, so it adds a second listener beside the first.

Each stale listener keeps its closed open's log alive (up to 1,024 reports, `KEPT`), base64-encodes every later report into it, and calls `changed('hid.inputReports')`. The page runtime turns each call into a commit and a re-ask of every reader (`host/web-js/modules.js:322`). So after N opens of one device, one report costs N encodings, N commits and N re-asks, and the logs of the N−1 closed opens are never freed.

`connections` also keeps every closed connection's record, and through it that open's log, for the page's life (`close`, line 128, sets `open` false and never deletes).

## The failure

An app that opens the device on a screen and closes it on leaving, visited four times. With a stand-in `navigator.hid` whose one granted device is an `EventTarget` that counts its listeners:

```js
for (let round = 1; round <= 4; round++) {
  const { connection } = await impl.open({ device: id });
  device.report(round);                       // dispatches one `inputreport`
  await impl.close({ connection });
}
```

Observed:

```
open 1: 1 inputreport listeners on the device; one report announced hid.inputReports 1 time(s)
open 2: 2 inputreport listeners on the device; one report announced hid.inputReports 2 time(s)
open 3: 3 inputreport listeners on the device; one report announced hid.inputReports 3 time(s)
open 4: 4 inputreport listeners on the device; one report announced hid.inputReports 4 time(s)
after 4 closes: 4 listeners still on the device; device.opened = false
```

Expected: one listener while the device is open, none after its last close, and one announcement a report.

## Verification

Ran the unmodified module under Bun with that stand-in, which has WebHID's `open`, `close` and `inputreport` event (https://wicg.github.io/webhid/#dom-hiddevice-oninputreport); the output above is its. Not run against a real device in Chrome.

## Direction

Keep the listener in the shared entry and remove it where the entry is deleted (`close`'s last lease and `forget`).

## Related

QUEUE "USB, HID and serial (LLP 1133.037): what is left" lists unbuilt targets and drives, not this. `issues/20261110-window-handlers-outlive-their-scope.md` is the Contract runtime's window handlers, another file.
