# On native hosts a module cannot assign over an inherited built-in property: `this.name` in an Error subclass throws, and nothing tells the author

**Status:** Closed
**Resolution:** Inherited writable, configurable built-in prototype properties can be shadowed while the intrinsic remains frozen.
**Systems:** js/src/lib.rs, vendor/ibex/crates/ibex2/src/bindings/harden.js, docs/reference.md, docs/agent-pitfalls.md
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd` the native executor freezes every intrinsic before it loads a module (`js/src/lib.rs:726-728`, which runs `vendor/ibex/crates/ibex2/src/bindings/harden.js`). The freeze is chosen: LLP 1027 records "universal hardening" and that "Ibex2's baked hardening locks its captured intrinsics". The walk freezes each object and does nothing more (`harden.js:67-86`), so `Object.prototype`, `Error.prototype` and the rest hold their data properties non-writable. By the language's own rule (ECMA-262 §10.1.9.2, OrdinarySetWithOwnDescriptor), an assignment to a property the object inherits as non-writable data fails: a `TypeError` in strict code (every class body), nothing at all in sloppy code, which is how Hermes runs the rest of the bundled module.

The web's module realm is not hardened (`js/src/binary.js:451-452`), so the same module works in the dev loop and in the browser and fails on every native host. `docs/reference.md:960` says "The language is the same everywhere", and no guide or entry in `docs/agent-pitfalls.md` mentions the difference. The repository works around it in its own code (`packages/oauth/index.ts:134-136`, `js/src/prelude.js:524-530`, `js/src/binary.js:104-109`).

## The failure

```ts
class MyError extends Error {
  constructor(m: string) { super(m); this.name = "MyError"; }
}
```

| | Hermes executor | unhardened realm |
| --- | --- | --- |
| `new MyError("x")` | throws `TypeError: Cannot assign to read-only property 'name'` | a `MyError` |
| `money.toString = () => "$2.50"; String(money)` | `[object Object]` | `$2.50` |
| `Point.prototype.toString = …; String(new Point(3))` | `[object Object]` | `Point(3)` |
| `Refused.prototype = Object.create(Error.prototype); Refused.prototype.name = "Refused"` | name stays `Error` | `Refused` |
| `for (const w of ["a", "constructor", "valueOf"]) seen[w] = true; Object.keys(seen)` | `a` | `a,constructor,valueOf` |

The first is how TypeScript usually names an error class. It throws where the app meant to throw its own error, so a `catch` that tests `instanceof MyError` sees a `TypeError`. The last loses entries of any object used as a map with keys from data. The sloppy cases also apply to a bundled dependency written in the ES5 style.

## Verification

Run on macOS: a fixture module with these cases under `js/tests/fixtures`, bundled and compiled as the other fixtures are, loaded with `Module::loaded` and asked through `answer` in a test under `js/tests/it`; it printed the left column. The same file imported in Bun, whose realm is unhardened as a browser's is, printed the right one. A class field (`override name = "MyError"`) works in both, since it defines the property. Not run: the other native hosts, which harden through the same call.

## Direction

Either make these assignments work as SES does after freezing: replace each data property of the frozen prototypes with an accessor whose setter defines an own property on the receiver, in `harden.js`, an Exact patch to the vendored Ibex. Or keep the behaviour and document it in `docs/reference.md`, "What a data module can use", and `docs/agent-pitfalls.md`, with a class field, `Object.defineProperty` and `Map` as the answers. The first keeps the web and native executors agreeing.

## Related

Nothing in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` names frozen prototypes or a refused assignment.
