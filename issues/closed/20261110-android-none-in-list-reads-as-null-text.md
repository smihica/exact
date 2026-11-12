# An option left `none` inside a list reaches Android's modules as the text "null"

**Status:** Closed
**Resolution:** `written` drops `null` fields of records inside lists too, so a platform never sees an option the call left out; a list's own `null` elements stay.
**Systems:** runner/src/runner/modules/bound.rs, packages/nfc/android/Nfc.kt, packages/quick-actions/android/QuickActions.kt
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

A call's arguments are encoded with `none` as JSON `null`, and `forward_body` removes the nulls
before an implementation sees them: "An option the call omits is left out, so it stays the
platform's" (`runner/src/runner/modules/bound.rs:134-136` at `dd6f5dcd`). The helper says "at
every depth" but only walks objects (`:460-466`):

```rust
fn written(value: &mut Json) {
    if let Json::Object(map) = value {
        map.retain(|_, v| !v.is_null());
        map.values_mut().for_each(written);
    }
}
```

A record inside a list keeps its nulls. Swift's `as? String` and the web's `!= null` read such
a field as absent. Android's `JSONObject.optString` does not: it "Returns the value mapped by
name if it exists, coercing it if necessary", and "the sentinel value `JSONObject.NULL` is
coerced to the string "null""
([JSONObject](https://developer.android.com/reference/org/json/JSONObject)). Two Android
modules read option fields of list items that way:

- `packages/quick-actions/android/QuickActions.kt:71`:
  `.setShortLabel(item.optString("short_name").ifEmpty { name })`.
- `packages/nfc/android/Nfc.kt:444`, `:449`, `:458`: a record's `id`, a text record's `lang`
  and a MIME record's `mediaType`.

**The failure.**

```
setAppShortcuts(shortcuts=[ShortcutItem(name="Open", short_name=none, description=none, url=url, icons=none, apple=none, android=none)])
write(message=NDEFMessageInit(records=[NDEFRecordInit(recordType="url", mediaType=none, id=none, encoding=none, lang=none, data=some("https://example.com/fixture"))]))
```

The second is `packages/nfc/test/fixture/app.contract:30`. Expected on Android: a shortcut whose
short label is "Open"; a URL record with no identifier. By the code: the launcher's short label
is "null", and the tag's record carries the four-byte identifier `null` (a text record also
gets the language code `null`).

**Verification.** The runner half by running: a temporary unit test beside `written` encoded
the first call's arguments with `exact_js_value::to_json`, as `args_json` does, and passed them
through `written` (`cargo test -p exact-runner --lib`):

```
encoded, in a list:     {"shortcuts":[{"android":null,"name":"Open","short_name":null,"url":"/open"}]}
forwarded, in a list:   {"shortcuts":[{"android":null,"name":"Open","short_name":null,"url":"/open"}]}
forwarded, in a record: {"shortcut":{"name":"Open","url":"/open"}}
```

The Android half by reading and Android's documentation: no JDK or device here, and QUEUE
records that the Android NFC registry has not been driven with a tag. The host hands a module
the parsed request's `args` as they came (`host/android/canvas/src/main/java/dev/exact/canvas/Extensions.kt:496`).

**Direction.** Walk arrays in `written`, so the rule its comment states holds for every host.
The Android reads could also test `isNull` first, as `Motion.kt` does for `after`.

**Related.** Shelf passes `short_name=some(…)` (`apps/shelf/app.contract:195`), which is why its
Android drive does not show this.
