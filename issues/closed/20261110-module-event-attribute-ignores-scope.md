# A module event imported under an alias cannot be heard: its root attribute is matched by name, not through the file's scope

**Status:** Closed
**Resolution:** Module event attributes resolve through their file's imports, including aliases, even when local bindings force an internal rename.
**Systems:** contract/cli/src/module_events.rs, contract/cli/src/sources.rs, contract/syntax/src/scope.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

An app hears a module's event with an attribute on its root's outermost node, `column notificationclick=opened` (LLP 1133.004 D6: "The app imports the event with its shape and handles it with an attribute").

`module_events::bind` finds that attribute by comparing its name with each event's `m.name` (`contract/cli/src/module_events.rs:16-24`, `:125`), which after scoping is the event's program-unique name (`contract/syntax/src/module_decl.rs:24-26`). Scoping rewrites calls, types and `class=` through the file's scope (`rescope`, `contract/syntax/src/scope.rs:165`), but not this attribute, so what the file's `use` says never enters the comparison:

1. An alias is ignored. After `use reading as tagEvent`, `tagEvent=` matches nothing, and `reading=`, a name the file does not have, binds.
2. The event binds when no `use` names it, as long as the package is loaded.
3. A local binding of the event's name makes `unique_names` rename the event (`contract/cli/src/sources.rs:1023-1025`: `reading` becomes `reading__exact`), and `reading=` matches nothing. `docs/agent-pitfalls.md` ("A module event's attribute refused as unknown on the root") records this for a derive, a state or an action; a `for` binder does it too. An alias, the usual way out of a name clash, does not help: both names are then refused.

An attribute that fails to match stays on the node, and lowering refuses it as an unknown attribute, which names neither the event nor the alias.

## The failure

```
use reading as tagEvent, NDEFReading from "@exact/nfc"
component App
  state heard = ""
  action tagRead(r: NDEFReading)
    heard = r.id
  view
    column tagEvent=tagRead
      text heard
```

`contract build app.contract -o app.plan`, with `@exact/nfc` in `node_modules`:

```
app.contract:7:12 [lower-unknown-attr] `column` has no attribute `tagEvent`
```

With `column reading=tagRead` the same file compiles. With `state reading = false` added, `tagEvent=` and `reading=` are both refused. `use notificationclick as tapped from "@exact/notifications"` behaves as the first two cases do, and with only `use showNotification from "@exact/notifications"`, `column notificationclick=clicked` compiles and the plan binds the event.

Expected: the attribute is resolved through the file's scope, as a call is. The alias binds, the name the file does not have is refused, and a local binding of the event's own name does not hide an event imported under another.

## Verification

Ran `contract build` on each file above against the repository's packages: it printed the refusals quoted and exited 1, and exited 0 for the files said to compile. A state, an action or a `for` binder named `reading` was refused as the pitfall says. Which events a compiled plan binds was read from `plan.module_events` in a temporary test, and the renamed name from a temporary trace line in `bind`. No host was run.

## Direction

In `bind`, match the attribute through the root file's scope (the local name its `use` brought, to the unique name), or have `rescope` rename the root's event attributes as it renames calls. The pitfall's entry can then go, or shrink to the case of an event and a local binding under one name.

## Related

`docs/agent-pitfalls.md`, "A module event's attribute refused as unknown on the root" (LLP 1133.035, Shelf's `derive reading`), records the clash and its workaround, renaming the local name. No issue or QUEUE entry names the cause or the alias.
