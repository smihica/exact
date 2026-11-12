# On macOS a USB control transfer whose `request`, `value` or `index` is out of range ends the app

**Status:** Closed
**Resolution:** The macOS setup packet takes `request`, `value` and `index` modulo 2^8 and 2^16, as WebIDL and WebUSB do, instead of trapping in a checked initializer.
**Systems:** packages/usb/apple/USB.swift
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, `control` in `packages/usb/apple/USB.swift:273-278` builds the setup packet with
Swift's checked integer initialisers:

```swift
request.bRequest = UInt8(setup["request"] as? Int ?? 0)
request.wValue = UInt16(setup["value"] as? Int ?? 0)
request.wIndex = UInt16(index)
```

`UInt8(_:)` and `UInt16(_:)` stop the process when the value is not representable. The interface
declares the three fields as plain `number` (`packages/usb/exact.json:99-105`,
`USBControlTransferParameters`), so nothing before this refuses `request: 256`, `value: 65536` or
a negative. The same function does check the transfer's length against `0...65535` and answers
`TypeError` (`:261-270`).

The other implementations do not crash on the same call. WebUSB takes `request` as `octet` and
`value`, `index` as `unsigned short` without `[EnforceRange]`
([USBControlTransferParameters](https://wicg.github.io/webusb/#dictdef-usbcontroltransferparameters)),
so the browser reduces them modulo 256 and 65536. The Linux implementation reads each with
`as_u64()` and casts with `as u8` or `as u16` (`packages/usb/linux/lib.rs:238`, `:247-248`), which
does the same for a non-negative integer and reads anything else as 0.

## The failure

On macOS, with a device open:

```ts
await controlTransferIn({ connection, setup: { requestType: "vendor", recipient: "device", request: 1, value: 0x10000, index: 0 }, length: 8 });
```

Expected: the transfer runs with `wValue` 0 as on the web and Linux, or the call answers
`TypeError`. Observed: the app's process traps in `UInt16(_:)`.

## Verification

Run, without a device: the conversions come before the transfer is sent. The `control` method
(`:251-286`, unchanged) was compiled over stand-ins for its device and request structure, its
arguments decoded by `JSONSerialization` as the seam's are, one case a process:

| `request`, `value`, `index` | Result |
|---|---|
| 1, 0, 0 | sent, `status: ok` |
| 255, 65535, 65535 | sent, `status: ok` |
| 256, 0, 0 | exit by SIGTRAP: "Fatal error: Not enough bits to represent the passed value" |
| 1, 65536, 0 | the same |
| 1, 0, 65536 | the same |
| 1, −1, 0 | exit by SIGTRAP: "Fatal error: Negative value is not representable" |

Not run: a transfer to a real device.

## Direction

Answer `TypeError` for a value outside its field's range, as the length does, or build the fields
with `truncatingIfNeeded:` to match the web; pick one and make Linux agree.

## Related

`issues/closed/20261102-usb-control-length-traps-native.md` fixed the transfer's length in this
function and left the setup fields: "This report concerns the measured length boundary, not other
unchecked numeric conversions in the device bridges." QUEUE "USB, HID and serial (LLP 1133.037):
what is left" lists macOS's bulk and interrupt transfers as not built; control transfers are
built.
