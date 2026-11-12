# An interactive terminal launch gives every launch the seed 0, reads the app's language from `LC_TIME` and tells no format locale

**Status:** Closed
**Resolution:** Interactive terminal launches draw a secure 32-bit seed and resolve message and format locales by their POSIX categories, shared with Linux.
**Systems:** host/terminal (zone.rs), host/linux (zone.rs)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

An interactive terminal launch builds the app's `Place` at `host/terminal/src/zone.rs:49-63` (`dd6f5dcd`):

```rust
locale: locale(&["LC_ALL", "LC_TIME", "LANG"].iter().find_map(...)),
time_zone: ...,
seed: 0.,
format_locale: String::new(),
```

`Place` (`runner/src/time.rs:188-202`) holds a `seed` "from the platform's secure random source, drawn once per launch" and two locales. `locale` is "the device's first preferred language", which the app's strings follow. `format_locale` is the regional format the platform's date and number formatters use; when it is empty, `formatLocale` answers `locale`.

POSIX resolves each category as `LC_ALL`, then the category's own variable, then `LANG` (IEEE Std 1003.1, Base Definitions, 8.2 "Internationalization Variables"). Message language is `LC_MESSAGES`; date and time formats are `LC_TIME`. So `locale` should read `LC_ALL`, `LC_MESSAGES`, `LANG`, and `format_locale` should read `LC_ALL`, `LC_TIME`, `LANG`. LLP 1101.003 D6 says so of the terminal ("`locale` is the first non-empty of `LC_ALL`, `LC_MESSAGES`, `LANG` ...; the seed is entropy"), and the Linux host does it: `locale` from `LC_MESSAGES`, `format_locale` from `LC_TIME` (empty when none of its three variables is set), the seed from `getrandom` (`host/linux/src/zone.rs:95-113`).

The terminal host leaves the seed at the runner's "not told yet" value, puts the date category where the language goes, and never reads `LC_MESSAGES`.

## The failure

Contract has no ambient randomness (LLP 1027.000 D1): a deal comes from `exactTime()`'s `seed`, which `docs/recipes/random-and-shuffle.md` describes as "new each launch". In a terminal app run by a person it is 0 every time, so a shuffle dealt from it is the same on every launch.

| Environment | Terminal `locale` / `format_locale` | Linux host |
|---|---|---|
| `LANG=en_US.UTF-8 LC_TIME=de_DE.UTF-8` | `de-DE` / `""` | `en-US` / `de-DE` |
| `LANG=en_US.UTF-8 LC_MESSAGES=fr_FR.UTF-8` | `en-US` / `""` | `fr-FR` / `en-US` |

In the first row a person who asked for English with German dates gets an app's German strings. In the second a person who asked for French gets English.

## Verification

An entry that shows `exactTime()`'s `seed` and `locale`, booted three times as the interactive loop boots it (`Host::boot_with_clock(…, true)`) under `LANG=en_US.UTF-8 LC_TIME=de_DE.UTF-8`, painted `seed 0 locale de-DE` each time.

A temporary test in each `zone.rs` called the launch function with the two environments, `launch(true, env)` in the terminal host and `launch_place(env)` in the Linux host:

```
terminal LANG=en_US.UTF-8 LC_TIME=de_DE.UTF-8:     locale="de-DE" format_locale="" seed=0
terminal LANG=en_US.UTF-8 LC_MESSAGES=fr_FR.UTF-8: locale="en-US" format_locale="" seed=0
linux    LANG=en_US.UTF-8 LC_TIME=de_DE.UTF-8:     locale="en-US" format_locale="de-DE" seed=1541881410
linux    LANG=en_US.UTF-8 LC_MESSAGES=fr_FR.UTF-8: locale="fr-FR" format_locale="en-US" seed=2363715891
```

A second Linux launch of each printed a different seed. No app with string tables was run.

## Direction

Draw the seed and read the two categories as the Linux host does. LLP 1101.003 D6 planned to move that function into `exact_runner::time`; one shared function also removes the second copy of the POSIX locale normalisation (`host/terminal/src/zone.rs:8-28` beside `host/linux/src/zone.rs:136-158`). LLP 1027.000.000 records the terminal as telling no format locale; amend that line with the change.

## Related

`issues/closed/20260927-linux-no-place-or-seed.md` set the Linux host's rule. `QUEUE.md`, "`formatLocale` carries a forced hour cycle on Apple only", covers Android and Windows, not the terminal.
