// A resource over a list on the JS target (LLP 1017.000 P4c, amended 2026-11-04; runner/src/runner/each.rs):
// `resource r = source(x) for x in list [key=k] as shape S`. Each element has its own resource (`res`), keyed as a
// row is: an element that joins makes one, which asks; one that leaves lets only its own ticket go, once the commit
// that dropped it stands; the others go on. `r` reads their values as `S[]` in the list's order, pending or failed
// when any element's is.
import { res, memo, scope, end, untracked, sig, write, eq, say, stood, restoring, owner, Resources, Refusal } from "./rt.js";
import { join, forget } from "./overlay.js";

const keyText = v => typeof v === "string" ? "s:" + v : typeof v === "number" ? (Number.isFinite(v) ? "n:" + (v === 0 ? 0 : v) : null) : typeof v === "boolean" ? "b:" + v : null;

/** `list()` the elements, `key(x)` an element's key, `args(x)` its request's arguments; `type` the list's, `elem` an
 * element's, `zero` what an element shows until its answer. */
export function resEach(name, source, list, key, args, type, elem, zero) {
  const members = new Map(), parent = owner(); // key text → { s: its scope, g: its resource, item: its element's slot, left }
  const remove = (k, e) => {
    members.delete(k);
    const r = e.g.r;
    if (r.ticket) { say(`forget ticket ${r.ticket.id} (${name})`); r.ticket = null; }
    end(e.s); Resources.splice(Resources.indexOf(r), 1); forget(r);
  };
  let order = [];
  const g = memo(() => {
    const seen = new Set(), next = [];
    for (const x of list()) {
      const k = keyText(key(x));
      if (k === null) throw new Refusal(`ResourceKey { resource: ${JSON.stringify(name)} }`);
      let e = members.get(k);
      if (!e && restoring()) continue; // a refused commit's settle asks nothing
      if (!e) {
        e = untracked(() => { const item = sig(x), e = { item }; e.s = scope(() => { e.g = res(name, source, () => e.left ? r.ticket?.args ?? r.settled : args(item()), undefined, undefined, elem, zero, false, true); }, parent); const r = e.g.r; return e; }); // a leaving element asks nothing more
        e.g.r.family = g.r;
        join(e.g.r);
        members.set(k, e);
        // A refused join has no checkpoint to restore and no request to keep.
        const joined = () => {};
        joined.refused = () => remove(k, e);
        stood(joined);
      } else if (!seen.has(k) && !eq(e.item.n.v, x)) write(e.item.n, x); // a repeated key reads the first element's answer
      e.left = false;
      seen.add(k); next.push(e);
    }
    for (const [k, e] of members) if (!seen.has(k) && !e.left) {
      e.left = true;
      stood(() => { // let go only once the commit that dropped it stands
        if (!e.left || members.get(k) !== e) return;
        remove(k, e);
      });
    }
    order = next;
    return next.map(e => e.g());
  }, type, name);
  const each = f => { for (const e of members.values()) if (!e.left) f(e.g.r); };
  g.p = () => (g(), order.some(e => e.g.p()));
  g.c = () => (g(), order.every(e => e.g.c())); // `current(x)`: every listed element answers its own arguments (LLP 1161.001)
  g.f = () => (g(), order.some(e => e.g.f()));
  g.e = () => { g(); for (const e of order) { const f = e.g.e(); if (f) return f; } return null; };
  // What a mutation's `refreshes` and the overlay ask of a resource, asked of each element's.
  g.r = { name, source, each: true,
    get value() { g(); return order.map(e => e.g.r.value); },
    force: undo => each(r => r.force(undo)), touch: undo => each(r => r.touch(undo)),
    reconcile: undo => each(r => r.reconcile(undo)), revise: undo => each(r => r.revise(undo)) };
  g.r.members = () => { const out = []; each(r => out.push(r)); return out; };
  g.n.resource = g.r;
  return g;
}
