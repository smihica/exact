# A document in `ja-JP`, `ko-KR` or `zh-Hant` gets the Simplified Chinese face for its Han characters

**Status:** Closed
**Resolution:** Han fallback selects Japanese, Korean and Chinese regional faces from language, script and region subtags on every painting host.
**Systems:** host/linux/src/text/fallback.rs (the painting host: Linux, Windows, Android)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

Han characters are drawn differently in Japanese, Korean, Simplified and Traditional
Chinese faces, so the fallback picks the Han face by the document's language
(`host/linux/src/text/fallback.rs:11` to `:13`). The choice is `han(locale)`, which compares
the whole tag (`fallback.rs:278` to `:286` for Linux and Android, `:96` to `:104` for
Windows, `:151` to `:159` for the Mac build):

```rust
match locale {
    "ja" => &["Noto Sans CJK JP"],
    "ko" => &["Noto Sans CJK KR"],
    "zh-HK" => &["Noto Sans CJK HK"],
    "zh-TW" => &["Noto Sans CJK TC"],
    _ => &["Noto Sans CJK SC"],
}
```

The tag is the name of the app's string table (`Runner::resolved_locale`,
`runner/src/runner/time.rs:16`), passed on unchanged, and a table may be named with any
subtags (`contract/cli/src/strings.rs:160`; `strings/en-GB.json` in
`contract/cli/tests/it/strings.rs:63`). So `ja-JP`, `ko-KR`, `zh-Hant`, `zh-Hant-TW`,
`zh-Hant-HK` and `zh-MO` all fall to the last arm. A Japanese app whose table is
`ja-JP.json` shows its kanji in the Simplified Chinese face (直, 骨 and 角 have visibly
different forms), and a Traditional Chinese app named `zh-Hant` likewise. `han_language` in
the same file already splits the tag (`fallback.rs:272` to `:275`), so on Android a `ja-JP`
document puts a CJK face first and it is the Simplified one.

## The failure

| tag | `han(tag)` on Linux and Android | expected |
| --- | --- | --- |
| `ja` | Noto Sans CJK JP | |
| `ja-JP` | Noto Sans CJK SC | JP |
| `ko-KR` | Noto Sans CJK SC | KR |
| `zh-TW` | Noto Sans CJK TC | |
| `zh-Hant`, `zh-Hant-TW` | Noto Sans CJK SC | TC |
| `zh-Hant-HK`, `zh-MO` | Noto Sans CJK SC | HK |

A language range matches a tag it is a prefix of up to a hyphen (RFC 4647 §3.3.1; CSS's
`:lang(ja)` matches `ja-JP` the same way), and the script and region subtags say which
Chinese is written (RFC 5646 §2.2). Chromium 156 on a Mac draws 直骨角 identically under `lang="ja"` and `ja-JP`,
under `ko` and `ko-KR`, under `zh-TW`, `zh-Hant` and `zh-Hant-TW`, and under `zh-HK`,
`zh-Hant-HK` and `zh-MO`, and differently between those groups and `zh-CN`.

## Verification

On a Mac, where only the macOS list compiles, a temporary test printed `han("ja")` as
Hiragino Sans and `han("ja-JP")`, `han("ko-KR")`, `han("zh-Hant")`, `han("zh-Hant-TW")`,
`han("zh-Hant-HK")` and `han("zh-MO")` as PingFang SC; `Measurer::set_language` then shaped
直 with Hiragino Sans W4 under `ja` and another face under `ja-JP`. The Linux and Android
and the Windows `han` and `unix_common`, copied verbatim into a standalone program, gave
the table above (Microsoft YaHei UI on Windows for each wrong row), and
`unix_common(true, "ja-JP")` begins with Noto Sans CJK SC where `"ja"` begins with JP. The
Chromium comparison hashed a screenshot for each tag. Not run: a Linux, Android or Windows
machine with those faces installed.

## Direction

Match on the tag's subtags in one function the three platform lists share: language `ja`
or `ko`; for `zh`, Traditional when the script is `Hant` or the region is `TW`, `HK` or
`MO` (Hong Kong's face for `HK` and `MO`), else Simplified.

## Related

`issues/20261110-inline-run-drops-font-optical-sizing.md` and
`issues/20261110-hosted-field-face-cache-outlives-its-plan.md` are other parts of the text
engine. QUEUE's "Parley on Linux, what is left" calls the macOS and Windows lists
unmeasured; it does not name the tag match. No issue or QUEUE entry names the Han face.
