# The Contract reference, graded

## With LLP 1161 (2026-11-11)

On the branch that implements LLP 1161.000 to 1161.002, the reference with `when`, `current` and `parallel` writes
grades visible 30/30, hidden 77/77, overlap 38/38, and the fuzzer finds no hole in 20 seeds (`ref3: 0/20 seeds found a
hole`): **0 defects**. With `when` and `current` and its writes still `queue`, it graded 30/30, 77/77 and 35/38, with 6
of 20 seeds finding a hole (9 defects).

## Before LLP 1161

`eval/reference/desk9/contract` (in `harness.patch`), graded by upstream's `harness/grade.sh` on exact2 `46f3d608`
and harness `afb81cd` (upstream `08255de` with the patch). Visible tests: 30/30.

**10 defects**: 0 hidden, 3 overlap, 7 fuzz seeds. Each comes from a mutation keeping one request in flight (README,
"The reference"). The fuzz holes are deterministic: two gradings found the same seven.

```
== hidden
77/77 passed
== overlap
✗ base: an earlier rename declined after a later one was accepted keeps the later title
✗ base: settings: an earlier save failing after a later one succeeded leaves the later one
✗ base: settings: a later save failing while an earlier one is out shows its error and then the earlier result
35/38 passed
== fuzz
desk9-contract-1: 7/20 seeds found a hole
  seed 1 step 6: row assignee: #3 shows bob, server null
    user bob ; focus ; land fetchIssues:open ; row-assign 2 ; row-close 0 ; row-assign 0
  seed 7 step 4: open row missing: #1
    land fetchIssues:open ; row-close 1 ; row-assign 0 ; row-close 0
  seed 8 step 14: row assignee: #3 shows ann, server null
    land fetchIssues:open ; row-close 1 ; filter closed ; fail fetchIssues:closed ; user bob ; land closeIssue:3 ; user ann ; land fetchIssues:open ; focus ; land fetchIssues:closed ; row-assign 0 ; select #3 ; land fetchComments:3 ; row-assign 1
  seed 9 step 21: row assignee: #3 shows guest, server null
    land fetchIssues:open ; filter closed ; tab help ; land fetchIssues:closed ; tab issues ; tab help ; land fetchIssues:open ; tab issues ; filter closed ; focus ; filter open ; user guest ; land fetchIssues:open ; land fetchIssues:closed ; select #4 ; assign ; land fetchComments:4 ; rename Bravo ; row-assign 0 ; rename Bravo ; row-assign 1
  seed 12 step 27: detail title: "#4 Bravofooter says 2019retryassign to guestrenamecloseguest", server "Typo in footer"
    land fetchIssues:open ; focus ; select #1 ; row-close 1 ; select #4 ; land closeIssue:3 ; row-assign 1 ; rename Alpha ; close ; rename Alpha ; land fetchIssue:1 ; land fetchIssue:4 ; row-close 0 ; rename Alpha ; fail closeIssue:4 ; land assign:4,guest ; land fetchComments:1 ; land fetchComments:4 ; post me too ; close ; fail renameIssue:4,Alpha ; assign ; land addComment:4,guest,me too ; close
  seed 14 step 14: row assignee: #3 shows guest, server null
    filter closed ; filter open ; land fetchIssues:open ; land fetchIssues:closed ; row-assign 0 ; select #1 ; land fetchIssue:1 ; land fetchLabels:1 ; close ; row-close 1 ; fail fetchComments:1 ; rename Alpha ; rename Alpha ; row-assign 0
  seed 18 step 6: open row missing: #3
    land fetchIssues:open ; select #4 ; row-close 0 ; land fetchLabels:4 ; assign ; row-close 0
```

## The same app with unsent writes left out of the overlay

A diagnostic copy whose `overlay` shows a queued write only once its request has started (it counts the requests its
sources start, which an overlay is not meant to read): visible 30/30, hidden 77/77, overlap 29/38, fuzz 0/20, **9
defects**. The six overlap tests it adds to the three above expect a write to show as soon as it is clicked while
another of its kind is out (two assignments, two closes, two posts, two label adds, two saves).
