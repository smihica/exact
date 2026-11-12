# The LLP reader links a citation of a three-level document to its parent

**Status:** Closed
**Resolution:** A citation's number takes every dotted part and links the longest prefix the corpus knows; the index's depth counts every dot.
**Systems:** apps/llp/data (refs.rs), apps/llp
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

`apps/llp/data/src/refs.rs:91-109` (at `dd6f5dcd`) reads a citation's number as "the digits (and one dotted part)": `number()` takes `1012`, then at most one `.NNN`. The corpus numbers documents three and four levels deep (`1012.001.000-trustworthy-agent-drives.rfc.md`, `1053.000.000.001-glass-morph.rfc.md`; 38 such files), and `Llp::read` resolves a number by exact match against the index (`apps/llp/data/src/lib.rs:480-485`). So `LLP 1012.001.000` in prose is read as `1012.001`, which is a different document, and the link goes there. The link's text is `LLP 1012.001`; `.000` stays plain text after it.

Where the two-level prefix names no document, the citation is not linked at all: the scan moves on without trying the longer number. The corpus has no such case today.

The same runs feed the terminal reader's link list (`apps/llp/data/src/page.rs:103-105`), so "open link" there opens the parent too.

## The failure

```rust
let out = llp_data::link_references(
    vec![Run::text("The driver refuses a stale build (LLP 1012.001.000).")],
    &|n| match n {
        "1012.001" => Some("1012.001-agentic-usability-review.research.md".into()),
        "1012.001.000" => Some("1012.001.000-trustworthy-agent-drives.rfc.md".into()),
        _ => None,
    },
);
```

Observed runs: `"The driver refuses a stale build ("`, then `"LLP 1012.001"` linked to `1012.001-agentic-usability-review.research.md`, then `".000)."`. Expected: one link `"LLP 1012.001.000"` to `1012.001.000-trustworthy-agent-drives.rfc.md`.

## Verification

Ran the snippet as an integration test of `llp-data`; it printed the three runs above. A second test with `LLP 1053.000.000.001` and a finder without `1053.000` produced no link.

Ran the reader's own pipeline (`markdown_parse::parse`, then `link_references` with a finder over the file names) across the corpus's 408 documents: 297 citations in 131 documents are linked to the parent of the document they name, none is left unlinked. `LLP 1035.005.000` is the most cited, 37 times, each a link to `1035.005`. Not run: the reader on a host.

## Direction

Have `number()` take every `.digits` part, and on a miss retry with the last part dropped, so the longest number the corpus knows wins and a sentence's final dot is never taken. The index's `depth` (`apps/llp/data/src/index.rs:232`) is 1 for every dotted number; count the dots while there.

## Related

No issue or QUEUE entry names the reader's citations. QUEUE's "The LLP terminal reader's leftovers" lists find across a run boundary, not this.
