# `push` labels a plan with the installed binary's format digest, not its own

**Status:** Closed
**Resolution:** `plan_graph` takes the plan's format version, format digest and kernel schema from the plan, so `classifyArtifacts` refuses a push to a cohort whose binary reads another format.
**Systems:** scripts/push.mjs, bake/src/receipt.rs (`plan_graph`), scripts/deploy.mjs (`classifyArtifacts`)
**Severity:** P2
**Author:** Claude (Opus 5.5), from GPT-6 Astra's review of LLP 1156.005 r3 (2026-11-10)
**Date:** 2026-11-10

Line references are at `46f3d608`.

## What is wrong

`push` compiles the plan with the checkout's compiler, then describes it with the installed
cohort's compatibility record. `planGraph` hands the cohort's `compat.json` to the graph tool
(`scripts/push.mjs:148`), and `plan_graph` copies `app`, `kernelSchema`, `formatVersion` and
`formatDigest` from those inputs into the plan's `requires` (`bake/src/receipt.rs:524`).
It does not read them from the plan it was given. The bundle's own `formatVersion` and
`kernelSchema` come from the cohort too (`scripts/push.mjs:230`). `classifyArtifacts` then
compares the copied values with the cohort and finds them equal.

So after a change to `plan/tables/format.json` (whose digest every host's decoder checks, LLP
1030 D3a) or to the kernel schema, `push` publishes a plan that no installed binary can decode,
under a requirement record that says it can. The refusal it should give, "It needs a new
binary" (`scripts/push.mjs:235`), is never reached.

## To see it

Read from the code; not run. Deploy an app, change `plan/tables/format.json` (LLP 1156.005 D4
added a resource field), then `bun scripts/push.mjs <app>`: the stream's head is replaced by a
bundle whose plan header carries the new digest.

## What a fix is

Take `formatVersion`, `formatDigest` and `kernelSchema` from the candidate plan (its header,
or the graph tool's own constants) rather than from the cohort. Let `classifyArtifacts`
refuse the push when they differ from the cohort's, naming `deploy` and a new binary. A test
in `scripts/push.test.mjs` (or the deploy smoke) with a cohort whose `formatDigest` differs
from `exact_plan::FORMAT_DIGEST` holds it.
