# The source graph omits `strings/<locale>.json`, so a strings edit rebuilds neither a Rust-source app's plan nor the resident dev session's

**Status:** Closed
**Resolution:** The source graph lists the app's `strings/*.json` tables, so `rerun_if_changed` and the dev session rebuild on a strings edit; a locale added later is still not looked for.
**Systems:** contract/cli/src/lib.rs (`source_graph`, `rerun_if_changed`), contract/cli/src/sources/tail.rs, contract/cli/src/strings.rs, host/web/src/dev.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`source_graph` is "every source compiling `path` reads … what a capture copies, a watcher
watches, and a deploy freezes" (`contract/cli/src/lib.rs:441-446`, LLP 1091 D10). It holds the
Contract files, the designs, their icon SVGs and the package manifests
(`contract/cli/src/sources/tail.rs:81`).

Every compile also reads the app's strings tables: `compile_loaded` calls `strings::load`
(`lib.rs:963`, `contract/cli/src/strings.rs:53-105`), and what `t` names is written into the
plan. LLP 1060 D3: "The plan remains a pure function of the sources, now including
`strings/*.json`." The tables are in no list of the graph, and two of its readers rebuild on
nothing else:

- `rerun_if_changed` (`lib.rs:551-591`), which a Rust-source app's build script calls, adding
  `../app.json` and `../data` by hand (`apps/metronome/linux/build.rs:7-20`). The revision the
  scripts give Cargo as `EXACT_BAKE_INPUTS` changes only when one of those printed paths does
  (`scripts/cargo-dep-info.mjs:58-87`), so a script build is no different.
- The resident dev session, `Session::poll` (`host/web/src/dev.rs:184`, its list from `uses`,
  `:109-128`).

A TypeScript app's bake is not affected: it names every `.json` under the app to Cargo itself
(`js/bake/src/lib.rs:428`, `:829`).

## The failure

```
app.contract          component App
                        view
                          text t("greeting")
strings/en.json       { "greeting": "Hello" }
```

`contract sources app.contract` lists `app.contract` alone. With a build script that calls
`contract::rerun_if_changed` and `contract::compile_path` on this app and embeds the plan:

```
$ cargo build && ./app            # the plan holds "Hello"
$ echo '{ "greeting": "Goodbye" }' > strings/en.json
$ cargo build -v                  # Fresh app v0.1.0
$ ./app                           # the plan holds "Hello"
$ touch app.contract && cargo build && ./app       # the plan holds "Goodbye"
```

`Session::poll` after the same edit returns `None` and the plan on disk keeps "Hello".
Expected: both rebuild.

## Verification

All run. The Cargo case used a crate outside the workspace with that build script and an
`app.json`; `cargo build -v` printed `Fresh` and the binary kept the old text. Driving the same
crate as `buildBake` does (`cargoBakeInputs` before the build, its record after,
`scripts/app.mjs:1312-1315`) kept one revision across the strings edit and the old text, and
gave a new revision and the new text after an edit to `app.contract`; a whole `build.mjs` run
was not made. The session case was a temporary test in `host/web/tests/it/dev.rs`. No
`.contract` file in the tree calls `t()`, so no build here shows it.

## Direction

Add the tables `strings::load` read to the graph's sources, and the `strings` directory to its
consulted paths so a new locale is seen, as a design and a package's manifest already are.

## Related

`issues/closed/20260927-swift-build-staleness.md` is the Swift archive's staleness, another
input list. No issue or QUEUE entry names the strings tables.
