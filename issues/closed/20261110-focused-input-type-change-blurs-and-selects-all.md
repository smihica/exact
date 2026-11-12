# Changing a focused Mac input's `type` blurs it, refocuses it and selects its whole value

**Status:** Closed
**Resolution:** Changing a focused Mac input's field class preserves its focus, selection and pending edit without blur or focus events.
**Systems:** host/apple/Sources/ExactKit/Mac/NativeFieldsMac.swift, FieldEditingMac.swift, FocusEvents
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

An `input` is a different AppKit class by `type`: `NSSecureTextField` for `password`, `NSSearchField` for a native `search`, a plain field otherwise. `NodeView.remakeFieldIfNeeded` (`host/apple/Sources/ExactKit/Mac/NativeFieldsMac.swift:82-96`, at `dd6f5dcd`) replaces the field when the class changes:

```swift
let focused = window != nil && editingEditor != nil
f.removeFromSuperview()
addSubview(n)
field = n
if focused { window?.makeFirstResponder(n) }
```

Removing the field that is being edited ends its editing, which the node hears as `controlTextDidEndEditing` (`FieldEditingMac.swift:37-51`): an edit not yet committed commits (`change`) and the node blurs. Making the new field first responder then sends `focus`, and the field editor selects the whole value, as it does whenever it takes a field (`TextAreaMac.swift:854-856`). The caret the person had is gone.

## The failure

A show-password button:

```
input type=(shown ? "text" : "password") value=pw input=edit blur=validate
button press=toggle
  text "Show"
```

Type `hunter2`, put the caret after the third character, press Show. A click on a button leaves the field its focus and its caret (`FocusMac.swift:99-110`), so the field is focused when the batch changes `type`. Observed: the app hears `blur`, so `validate` runs, then `focus`; the whole value is selected, and the next key typed replaces it. Pressing Show again does the same. Expected: the field keeps its focus and its selection and no event fires. Chrome does that, and the HTML Standard's steps for a changed `type` reset the selection only when the old type had none.

## Verification

A temporary XCTest booted that page in a session (`SheetFixture`) with `focus` and `change` handlers too, focused the field, typed in its editor and pressed the buttons with `Presenter.press`. The field's class, its editor's selection and what the app had heard, after each step:

```
typed, caret at 3                     SecureField  {3, 0}  focus input
a press that changes no type          SecureField  {3, 0}  focus input change
password to text                      Field        {0, 7}  ... blur focus
caret moved to 2, text to password    SecureField  {0, 7}  ... blur focus
one more character typed, then `type`
set by a batch, with no press         Field        {0, 8}  ... input change blur focus
```

Chromium 156, the same steps on an `<input>`: focus kept, `selectionStart` and `selectionEnd` 3, no `blur`, `focus` or `change`.

## Direction

Carry the editing across the swap: read the editor's selection first, keep the end of editing from reaching `editingEnded` as `keepingEditing` does for a placeholder write (`FieldEditingMac.swift:76-83`), and show the saved selection in the new field's editor.

## Related

`issues/closed/20260904-macos-offscreen-capture-can-be-transparent.md` records an app whose password field is `type=showPassword ? "text" : "password"`. No issue, `QUEUE.md` entry or pitfall covers a `type` change on a focused input; `issues/closed/20261009-field-selection-beforeinput-range-edit.md` is about selection events on caret moves.
