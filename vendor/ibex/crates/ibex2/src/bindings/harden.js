// The LLP 0067 R4 intrinsic freeze (measured in LLP 0062 §3), run after the standard library is
// installed and before any module code.
//
// Three properties this walk must have, all of which the retired native
// __exactDeepFreeze also had: it reads property DESCRIPTORS so getters are not
// invoked, it is ITERATIVE so a deep graph cannot hit a native stack cap, and
// it tracks VISITED so cycles terminate.
//
// The global object itself is NOT frozen. Its existing bindings are locked —
// `Array` cannot be pointed at something else, and everything reachable from
// them is frozen — but the object stays extensible, so application code can
// add to it. That is the SES lockdown shape: shared intrinsics are immutable,
// the global object is a compartment's own. It is also what running Exact
// requires: its runtime anchors shared state on `globalThis` under 193
// distinct `__exact*` names, plus `process`, `window`, `self`, `global`, and
// `navigator`, and with a frozen global every one of those writes silently
// did nothing — the first symptom was a `TypeError` three modules later,
// reading a registry that had never been created. R1 is unaffected: nothing
// capability-bearing is on the global object to begin with, and a property
// an application adds is state, not authority.
(function () {
  "use strict";
  // Everything the walk calls is captured here, and every descriptor it builds
  // or reads is classified by OWN fields only. Bootstrap code may pollute
  // Object.prototype (a planted `value` would make an accessor look like data,
  // and would leak into an attributes literal as a value to write) or delete
  // globals such as Function and Reflect, as Snapback 2 effects do.
  const O = Object;
  const getOwn = O.getOwnPropertyDescriptor;
  const names = O.getOwnPropertyNames;
  const symbols = O.getOwnPropertySymbols;
  const define = O.defineProperty;
  const freeze = O.freeze;
  const protoOf = O.getPrototypeOf;
  // Descriptor records inherit from Object.prototype. Only when bootstrap has
  // planted a `value` there does an `in` test lie; decide that once, since no
  // other code runs during the walk, and pay for the own-field lookup only then.
  const isData = ("value" in {})
    ? function (d) { return getOwn(d, "value") !== undefined; }
    : function (d) { return "value" in d; };
  const DATA_LOCK = { __proto__: null, writable: false, configurable: false };
  const ACCESSOR_LOCK = { __proto__: null, configurable: false };
  const keysOf = function (o) {
    const out = names(o);
    const syms = symbols(o);
    for (let i = 0; i < syms.length; i++) out[out.length] = syms[i];
    return out;
  };
  // Freezing writable data properties would also forbid an inheritor's own
  // assignment (ECMA-262 OrdinarySetWithOwnDescriptor). Keep the intrinsic
  // fixed while letting a receiver define the property it would have made.
  const allowOverride = function (obj, key, value, enumerable) {
    const accessor = {
      __proto__: null,
      configurable: false,
      enumerable: enumerable,
      get() { return value; },
      set(next) {
        if (this === obj) throw new TypeError("Cannot assign to a frozen intrinsic");
        const own = getOwn(this, key);
        if (own) {
          if (!isData(own) || !own.writable) throw new TypeError("Cannot assign to a read-only property");
          define(this, key, { __proto__: null, value: next });
        } else {
          define(this, key, { __proto__: null, value: next, writable: true, enumerable: true, configurable: true });
        }
      },
    };
    freeze(accessor.get);
    freeze(accessor.set);
    define(obj, key, accessor);
  };
  const seen = new Set();
  const queue = [];
  const objects = [];
  const prototypes = new Set();

  seen.add(globalThis);
  const globals = keysOf(globalThis);
  for (let i = 0; i < globals.length; i++) {
    let d;
    try { d = getOwn(globalThis, globals[i]); } catch (e) { continue; }
    if (!d) continue;
    const data = isData(d);
    if (d.configurable) {
      try { define(globalThis, globals[i], data ? DATA_LOCK : ACCESSOR_LOCK); } catch (e) {}
    }
    if (data) queue[queue.length] = d.value;
    else { queue[queue.length] = d.get; queue[queue.length] = d.set; }
  }
  try { queue[queue.length] = protoOf(globalThis); } catch (e) {}

  while (queue.length) {
    const obj = queue.pop();
    if (obj === null || (typeof obj !== "object" && typeof obj !== "function")) continue;
    if (seen.has(obj)) continue;
    seen.add(obj);
    objects[objects.length] = obj;
    // Names AND symbols: `Date.prototype[Symbol.toPrimitive]` and the RegExp
    // `Symbol.match`/`split`/... functions are reachable only by symbol, and a
    // walk by name left every one of them extensible.
    let keys;
    try { keys = keysOf(obj); } catch (e) { continue; }
    for (let i = 0; i < keys.length; i++) {
      let d;
      try { d = getOwn(obj, keys[i]); } catch (e) { continue; }
      if (!d) continue;
      if (isData(d)) {
        queue[queue.length] = d.value;
        if (typeof obj === "function" && keys[i] === "prototype") prototypes.add(d.value);
      } else { queue[queue.length] = d.get; queue[queue.length] = d.set; }
    }
    try {
      const proto = protoOf(obj);
      prototypes.add(proto);
      queue[queue.length] = proto;
    } catch (e) {}
  }
  // Discover prototypes before locking anything: a constructor may be visited
  // after its prototype. Original values stay in objects even when a getter
  // replaces their descriptor, so the freeze still reaches them.
  for (let i = 0; i < objects.length; i++) {
    const obj = objects[i];
    if (prototypes.has(obj)) {
      const keys = keysOf(obj);
      for (let j = 0; j < keys.length; j++) {
        const d = getOwn(obj, keys[j]);
        if (d && isData(d) && d.configurable && d.writable) {
          allowOverride(obj, keys[j], d.value, d.enumerable);
        }
      }
    }
    try { freeze(obj); } catch (e) {}
  }
})();
