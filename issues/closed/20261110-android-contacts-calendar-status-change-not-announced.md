# On Android, a contacts or calendar access change made in Settings is announced only while a list is being read

**Status:** Closed
**Resolution:** Android announces contacts and calendar access changes on foreground return even when no list has been read.
**Systems:** packages/calendar/android/Calendar.kt, packages/contacts/android/Contacts.kt
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

`calendarStatus` and `contactsStatus` are live sources: a reader is asked again only when the module announces them. Both Android modules route every announcement through `changedStore`, which returns at once when no list reader is registered:

```kotlin
private fun changedStore() {
    if (readers.isEmpty()) return      // Calendar.kt:131, Contacts.kt:151 at dd6f5dcd
```

`readers` holds only the list names (`calendars`, `calendarEvents`; `queryContacts`), never the status. When the person changes the app's access in Settings and comes back, the runner tells the module (`foreground`, `Calendar.kt:121`), which calls `real.refresh()`. That finds the difference (`if (status().toString() != before) changed()`, `packages/calendar/android/EventStore.kt:109`; `packages/contacts/android/ContactStore.kt:93` to `98`) and calls `changedStore`, which drops it. `refresh` has by then replaced the status it compares with, so no later return announces it either. The runner asks nothing again at a return by itself.

The Apple modules announce the status in that case: `guard !readers.isEmpty else { context.changed("calendar.calendarStatus"); return }` (`packages/calendar/apple/Calendar.swift:132`), and the same for contacts (`packages/contacts/apple/Contacts.swift:152`).

## The failure

A screen that asks for access before anything reads a list:

```
resource access = calendarStatus()
text "{access.state}"
```

On Android, refuse the calendar permission twice, so the state reads `denied` and only Settings can change it. Open the app's page in Settings, allow Calendar, and return to the app. Granting a permission does not end the process.

Observed, by reading: the text stays `denied` until the app is restarted or a list is read. Expected: `granted`. LLP 1133.018 D7: "`contactsStatus` and `calendarStatus` watch it too, since a change of access arrives the same way", and "A grant changed in Settings is read again on return to the foreground".

The defect needs no list of the module to have been read (`readers` empty), which is how a screen that asks for access first is built.

## Verification

Read, not run: Kotlin does not run on the review machine. The path is three lines, with no branch that announces the status: `foreground` (`Calendar.kt:121` to `126`), `refresh` (`EventStore.kt:107` to `110`), `changedStore` (`Calendar.kt:130` to `131`); contacts is the same (`Contacts.kt:140` to `151`). A request is not affected: `allowed` announces `calendar.calendarStatus` itself (`EventStore.kt:126`).

The device check that settles it: the steps above on an emulator, with the two-line screen.

## Direction

Announce the status from `changedStore` when no list is read, as the Apple modules do.

## Related

QUEUE "Contacts and calendar: what the four stages left" says Android is built and not run; it names no defect. No issue covers it.
