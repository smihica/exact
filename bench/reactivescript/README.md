# ReactiveScript's eval, with Exact added

[ReactiveScript](https://github.com/solidjs/reactivescript)'s benchmark of AI-written apps, with a `contract` arm.
A fresh agent builds an issue desk (`eval/tasks/desk9`: issues with assigning, renaming, closing, comments and labels;
a help search; a settings form) from a product brief. The backend is fake: every API call stays in flight until the
test lands or fails it, so tests choose the order answers arrive in. Each app is graded by 77 hidden tests, 38 hidden
"overlap" tests (writes that overlap each other or a reload), and 20 seeds of a timing fuzzer whose oracles check
every frame against the server's truth. A failed test or a fuzz seed that finds a hole is one defect.

Upstream's round 11 ([results](https://github.com/solidjs/reactivescript/tree/main/eval/results/desk-2026-11-11))
compared ReactiveScript, React, plain TypeScript and Foldkit. Mean defects per app:

| | Opus (3 apps) | Haiku (6 apps) |
|---|---:|---:|
| ReactiveScript | 2.0 | 3.8 |
| React | 2.0 | 9.7 |
| plain TypeScript | 3.0 | 14.5 |
| Foldkit | 2.7 | 13.2 |

Its tasks and harness are unchanged from that round's freeze (`b8b9e0b`) to the commit `setup.sh` pins.

| File | |
|---|---|
| `harness.patch` | One commit on upstream `08255de`: the `contract` arm (`eval/harness/contract.mjs`, hooked into build, check, scaffold, prompt and score) and a reference app in Contract (`eval/reference/desk9/contract`) |
| `setup.sh [dir]` | Clones upstream at that commit into `dir` (default `~/bench-rx/harness`), applies the patch, installs the harness's packages and Chromium |
| `run.sh` | One round: scaffolds the desks, runs an agent in each, grades them and writes `report.md` |
| `report.mjs <round>` | The round's table: defects per desk, then the mean per arm and tier |
| `results/` | Rounds and the reference's grading |

## The contract arm

A desk is the app `exact new` makes, with the brief, the given `src/api.js` and `src/flights.js`, the visible tests,
and `./check` and `./test` beside it. `app.ts` imports the backend from `./src/api.js`. `./check` is
`bun exact.mjs contract build app.contract`, which also type-checks `app.ts`. `./test` makes a release web build
(`exact.mjs web-build --out dist`) and runs the visible tests on it, as the other arms do. The data module runs in the
page's realm on the web JS target, so `flights.js` puts its control on the page and the upstream driver needs no
change. The agent's prompt is upstream's, with the language rule naming Exact. With `--inline-docs`, it ends with
`docs/start-here.md`, as the `.rx` arm's ends with ReactiveScript's guide. The prompt allows `contract vocab` and
`docs` lookups and forbids the verbs that build, drive or serve the app, so `./check` and `./test` (15 calls between
them) are the only feedback, as in every arm.

## Running a round

```sh
bench/reactivescript/setup.sh                     # → ~/bench-rx/harness
bench/reactivescript/run.sh                       # → ~/bench-rx/runs/<timestamp>/report.md
ARMS=contract FRONTIER=1 SMALL=0 bench/reactivescript/run.sh    # one Opus desk
```

`run.sh` makes contract desks from a `git archive` of `EXACT2_REF` (default `origin/main`) without `bench/`, so an
agent can read Exact's docs but not this reference or its history. Agents run as `claude -p` in their desk, with MCP
servers and web tools off and `EXACT_FEEDBACK=0`. Upstream ran its agents as Claude Code subagents and notes that
numbers are comparable only within one harness: a round here therefore runs a React control (`ARMS="contract
react"`, the default) alongside Exact. Isolation is by instruction, as upstream's is: the prompt forbids reading
outside the desk and Exact's docs, so check the transcripts.

## The reference

`eval/reference/desk9/contract` is the brief written in Contract by hand. Every resource is the root's, so each
page's state lives in the root and opening a page resets it. A resource asks only while its page is open and it has
something to ask (`when`, LLP 1161.002), and every "loading" is `not current(…)` (LLP 1161.001), so a refresh keeps
what shows. Writes show at once through `app.ts`'s `overlay`, and each kind of write is a `parallel` mutation
(LLP 1161.000). It is 372 non-blank lines of Contract and 104 of TypeScript, 476 in all, where upstream's frontier
agents wrote 414 in ReactiveScript and 784 in React.

| Exact | visible | hidden | overlap | fuzz seeds with a hole | defects |
|---|---:|---:|---:|---:|---:|
| `46f3d608`, before LLP 1161 (writes `queue`, sentinel arguments, echoed requests) | 30/30 | 77/77 | 35/38 | 7 of 20 | 10 |
| with LLP 1161.001 and 1161.002, writes still `queue` | 30/30 | 77/77 | 35/38 | 6 of 20 | 9 |
| with LLP 1161.000 too | 30/30 | 77/77 | 38/38 | 0 of 20 | 0 |

The defects before LLP 1161.000 all came from a mutation keeping one request in flight
([results/reference.md](results/reference.md)): three overlap tests land a later write while an earlier one of its
kind is still out, and the fuzzer flags a queued write's guess shown before its request starts. LLP 1161 records the
finding and the three changes.
