export function releaseAssets(assets) {
  for (const card of assets?.values() ?? []) {
    if (card.objectURL) {
      URL.revokeObjectURL(card.objectURL);
    }
  }
}
// LLP 1007 §6: boot the dev generation when named; use the baked plan if none arrives within five seconds
export function devFirst() {
  if (!document.querySelector("meta[name=\"exact-dev-generation\"]")) {
    return null;
  }
  const slot = globalThis.exactDevFirst ??= {};
  return new Promise((resolve) => {
    const timer = setTimeout(() => resolve(null), 5e3);
    const ask = () => {
      clearTimeout(timer);
      slot.provide().then(resolve, () => resolve(null));
    };
    if (slot.provide) {
      ask();
    } else {
      slot.ready = ask;
    }
  });
}
