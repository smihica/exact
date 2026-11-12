# Two sends pass `analyze-send-twice` when a `match` binder is spelled like the state the first send tested

**Status:** Closed
**Resolution:** Guard facts distinguish match bindings from outer names, detecting overlapping effects while preserving exclusive paths across shadowing scopes.
**Systems:** contract/analyze/src/sends.rs, contract/analyze/src/calls.rs, contract/analyze/src/rereads.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at `dd6f5dcd`. LLP 1088 D8 refuses two sends to one
mutation on one path, and its as-built note admits one exception: "sequential
`if`s that test one name against different literals, with no assignment or
send of that name between them, are exclusive".

The walk keeps each fact by the tested name's spelling (`facts`,
`contract/analyze/src/sends.rs:37-83`; `exclusive`, `:29-34`), and
`Stmt::Match` carries the enclosing guard into its `some` arm unchanged
(`:262-271`). A `match` statement's binder may shadow a state (LLP 1089 D3: "a
`match` binding may share a slot's spelling"; only a `let` is refused,
`type-let-shadow`). Inside the arm the same spelling names another value, and
a test of the binder is read as a second test of the state.

## The failure

```
shape R
  ok: bool

component App
  state k = "1"
  state o = some("2")
  state n = 0
  mutation m as shape R
  action go()
    if k == "1"
      send m = save("a")
    match o
      case some(k)
        if k == "2"
          send m = save("b")
      case none
        n = 1
  view
    button press=go testId="go"
      text "go"
```

Observed: it compiles. On the runner, `go` makes two requests to `m` in one
commit, and the first reply is dropped:

```
requests: 2
  ticket 1 target m body {"source":"save","arg":"a"}
  ticket 2 target m body {"source":"save","arg":"b"}
pending: [("m", 2)]
  fulfill 1: landed Ok(false)
  fulfill 2: landed Ok(true)
```

Expected: `analyze-send-twice`, as the same program with the binder named `v`
(`case some(v)`, `if v == "2"`) gets: "`go` sends `m` twice on one path, at
lines 11 and 15".

The warning `analyze-read-after-assign` has the same gap. `rereads.rs:98` and
`calls.rs:131` push the binder onto `bound` for slot reads but keep its facts
by spelling. With `n = n + 1` in place of the first send and `seen = n` in
place of the second, the `v` form is warned of at `seen = n` and the `k` form
is not.

## Verification

Ran `contract build` on both forms of both programs: the `k` forms build with
no diagnostic, the `v` forms get the refusal and the warning. Booted the `k`
form of the first on the runner with a data source whose sends answer later,
ran `go`, and printed `take_requests()`, `pending()` and each `fulfill`: the
output above. `calls.rs` was read, not run.

## Direction

Entering a `match` statement's `some` arm, forget what the guard and the
recorded sends say of the binder's name (as `changed` does for an assigned
name), and forget what the arm learned of it on leaving. The three walks share
`facts` and `exclusive`, so one helper serves them.

## Related

No issue names `analyze-send-twice`, and `QUEUE.md` and
`docs/agent-pitfalls.md` do not mention shadowing in these walks. A parameter
that shadows a state is consistent through the whole action and is not
affected.
