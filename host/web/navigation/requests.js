// Defer refusal replies until the batch finishes to prevent reentrant commits; count them in flight through delivery
export function deferredFulfill(fulfill, inflight) {
  return (...args) => {
    const p = Promise.resolve().then(() => fulfill(...args));
    inflight.add(p);
    p.finally(() => inflight.delete(p));
  };
}
// An admission refusal answers its ticket Refused (kind 2) with the reason, in its incarnation.
export const refusal = (op, incarnation) => [
  incarnation,
  op.ticket,
  2,
  0,
  "",
  new TextEncoder().encode(op.message)
];
