// Pictures of a package's `img` scheme on the JS target (LLP 1133.019 D2,
// D11 item 1), as the native hosts' raster loaders and the runner's
// `modules/pictures.rs` do: a source such as `swatch:red`, whose scheme an
// interface declares (`images`), is never fetched. An `img` given one (bound,
// through `PropHooks.src`, or literal, as rows.rs writes it:
// `data-exact-picture`) is admitted first (a read, never a ticket); a refusal
// empties it and fires `error` with the reason. Admitted, the bound web
// implementation is asked `image(source, {width, height, fit, owned})` for the
// box's device pixels and `object-fit`, and the Blob it answers is shown
// through an object URL, counted in flight so `clock settle` waits for it.
// The implementation may say a picture changed (each `img` showing it asks
// again) or revoke it (each is emptied and fires `error` "revoked", and a
// reply asked before is dropped).
import { After, PropHooks, inflight, onEnd, onMount, owner, say } from "./rt.js";

/** `schemes`: scheme → extension; `admit(extension, source)`: null to ask, else why not; `instance(extension)`: its
 * implementation; `decide(extension)`: admission's answer now, uncounted. Answers the implementations' context:
 * `revokePicture(source)`, `pictureChanged(source)`. */
export function packagePictures(schemes, admit, instance, decide) {
  const showing = new Map(); // img → { source, url, asked }
  const generations = new Map(); // source → its generation: a revocation moves it
  // Boxes refused admission: asked again after each commit, since an implementation bound or a permission granted
  // since may let one in (D2).
  const refused = new Set();
  const STALE = Symbol();
  const live = e => e.isConnected && !e.closest?.("[data-exiting]");
  const extensionOf = v => { const i = typeof v === "string" ? v.indexOf(":") : -1; return i > 0 && Object.hasOwn(schemes, v.slice(0, i)) ? schemes[v.slice(0, i)] : undefined; };
  const forget = e => {
    const s = showing.get(e);
    if (s) { s.unwatch?.(); if (s.url) URL.revokeObjectURL(s.url); }
    showing.delete(e);
  };
  const watchGhost = e => {
    const s = showing.get(e);
    if (!s || s.unwatch || typeof MutationObserver !== "function") return;
    const hosts = new Set();
    for (let n = e; n; n = n.parentNode) if (n.hasAttribute?.("data-exiting") && n.parentNode) hosts.add(n.parentNode);
    if (!hosts.size) return;
    const obs = new MutationObserver(() => { if (!e.isConnected) forget(e); });
    for (const host of hosts) obs.observe(host, { childList: true });
    s.unwatch = () => { obs.disconnect(); s.unwatch = null; };
  };
  // An `img` with no picture fires `error` with `why`, as an unresolved image does (rt.js `imageEvent` reads `$why`),
  // in a microtask: the commit that makes a box attaches its `error` handler after setting its source.
  const fail = (e, why) => {
    e.removeAttribute("src"); e.$settled = null; e.$why = why;
    const source = e.$picture;
    queueMicrotask(() => { if (e.$picture === source && e.$why === why && live(e)) e.dispatchEvent(new Event("error")); });
  };
  const box = e => {
    const r = e.getBoundingClientRect(), scale = (typeof devicePixelRatio === "number" && devicePixelRatio) || 1;
    // Before layout, the platforms' own thumbnail size.
    const [w, h] = r.width > 0 && r.height > 0 ? [r.width, r.height] : [512, 512];
    return { width: Math.max(1, Math.ceil(w * scale)), height: Math.max(1, Math.ceil(h * scale)), fit: getComputedStyle(e).objectFit || "fill" };
  };
  const draw = (e, source, owned = false) => {
    const extension = extensionOf(source);
    forget(e);
    refused.delete(e);
    e.$picRetired = false;
    if (owned && !e.$picEnd && owner()) {
      e.$picEnd = true;
      onEnd(() => {
        e.$picRetired = true;
        refused.delete(e);
        // `end()`/`endAll()` run before `clear()`/`leave()` either disconnect the DOM
        // or mark `[data-exiting]`. Only forget synchronously if already disconnected;
        // otherwise `After` (or this microtask fallback) forgets non-ghosts and watches
        // exiting ghosts until their exit animation removes them.
        if (!e.isConnected) { forget(e); return; }
        queueMicrotask(() => {
          if (!showing.has(e)) return;
          if (!e.isConnected || !e.closest?.("[data-exiting]")) forget(e);
          else watchGhost(e);
        });
      });
    }
    // The source last asked for: a revoked one is not asked again until it changes.
    e.$picture = source;
    e.setAttribute("data-exact-picture", source);
    const why = admit(extension, source);
    if (why) { say(`image refused: ${source}: ${why} (LLP 1133.019 D2)`); refused.add(e); fail(e, why); return; }
    const s = { source, url: null, asked: generations.get(source) ?? 0 };
    const current = () => live(e) && !e.$picRetired && showing.get(e) === s && (generations.get(source) ?? 0) === s.asked;
    showing.set(e, s);
    e.$srcGen = (e.$srcGen ?? 0) + 1;
    e.$why = null;
    e.$settled = null;
    e.removeAttribute("src");
    inflight.n++;
    Promise.resolve()
      .then(() => (current() ? instance(extension) : STALE))
      .then(impl => {
        if (impl === STALE || !current()) return STALE;
        if (typeof impl?.image !== "function") throw Object.assign(new Error(`${extension}'s web implementation serves no pictures`), { code: "NotSupportedError" });
        return impl.image(source, { ...box(e), owned: false });
      })
      .then(async picture => {
        if (picture === STALE || !current()) {
          if (typeof ImageBitmap === "function" && picture instanceof ImageBitmap) picture.close?.();
          return;
        }
        const blob = picture instanceof Blob ? picture
          : typeof ImageBitmap === "function" && picture instanceof ImageBitmap ? await bitmapBlob(picture)
          : new Blob([picture]);
        if (!current()) return;
        s.url = URL.createObjectURL(blob);
        e.$why = null;
        e.setAttribute("src", s.url);
      })
      .catch(err => { if (current()) fail(e, err?.code ?? err?.name ?? "UnknownError"); })
      .finally(() => inflight.n--);
  };
  const previous = PropHooks.src;
  PropHooks.src = (e, v) => {
    if (e.localName === "img" && extensionOf(v) !== undefined) {
      if (e.$app) { e.$app = null; e.$appSeq = (e.$appSeq ?? 0) + 1; e.removeAttribute("data-app-src"); }
      e.$unsymbol?.();
      e.$untemplate?.();
      if (e.$picture !== v) {
        e.$srcGen = (e.$srcGen ?? 0) + 1;
        e.$why = null;
        e.$settled = null;
        e.$picBound = v;
        e.setAttribute("data-exact-picture", v);
        e.removeAttribute("src");
        onMount(() => {
          if (e.$picBound === v) e.$picBound = undefined;
          if (e.getAttribute("data-exact-picture") === v && e.$picture !== v) draw(e, v, true);
        }, (e.$pictureKey ??= {}));
      }
      return true;
    }
    if (e.$picture !== undefined || e.$picBound !== undefined || e.hasAttribute("data-exact-picture")) {
      forget(e); refused.delete(e); e.$picture = undefined; e.$picBound = undefined; e.$srcGen = (e.$srcGen ?? 0) + 1; e.$why = null; e.$settled = null; e.removeAttribute("data-exact-picture");
    }
    return previous ? previous(e, v) : false;
  };
  // Literal sources (rows.rs), and the boxes that left the page.
  After.push(() => {
    if (typeof document !== "object") return;
    for (const e of document.querySelectorAll("#exact-root img[data-exact-picture]")) {
      if (!live(e) || e.$picRetired) continue;
      const v = e.getAttribute("data-exact-picture");
      if (e.$picture === v || e.$picBound === v) continue;
      if (extensionOf(v) === undefined) { e.$picture = v; say(`image refused: ${v}: no package serves its scheme (LLP 1133.019 D2)`); fail(e, "no package serves its scheme"); continue; }
      draw(e, v, false);
    }
    // Keep exiting ghosts (and scopes currently between `end()` and `leave()`) tracked in
    // `showing` until disconnected; `onEnd`'s microtask forgets non-ghost retired nodes once
    // `leave()` has run, and `watchGhost` revokes URLs when an exit animation removes the ghost.
    for (const e of [...showing.keys()]) {
      if (!e.isConnected) forget(e);
      else if (e.closest?.("[data-exiting]")) watchGhost(e);
    }
    for (const e of [...refused]) {
      if (!live(e) || e.$picRetired || extensionOf(e.$picture) === undefined) refused.delete(e);
      else if (decide(extensionOf(e.$picture)) == null) draw(e, e.$picture, false);
    }
  });
  return {
    revokePicture(source) {
      generations.set(source, (generations.get(source) ?? 0) + 1);
      for (const [e, s] of [...showing]) if (s.source === source) {
        forget(e);
        if (live(e) && !e.$picRetired) fail(e, "revoked");
        else { e.removeAttribute("src"); e.$settled = null; e.$why = null; }
      }
      say(`picture ${source}: revoked`);
    },
    pictureChanged(source) {
      for (const [e, s] of [...showing]) if (s.source === source && live(e) && !e.$picRetired) { forget(e); draw(e, source, false); }
    },
  };
}

async function bitmapBlob(bitmap) {
  const canvas = new OffscreenCanvas(bitmap.width, bitmap.height);
  canvas.getContext("2d").drawImage(bitmap, 0, 0);
  return canvas.convertToBlob();
}
