// One activation button per click; authored writes and reentrant click() keep their order.
let Invokers = new Map();
const invocationAttrs = new Set(["popovertarget", "commandfor"]);

export const suppressedInvocation = (e, name) => Invokers.has(e) && invocationAttrs.has(name);

function muteInvoker(e) {
  if (Invokers.has(e)) return;
  const values = new Map([...invocationAttrs].map(k => [k, e.getAttribute(k)]));
  const set = e.setAttribute, remove = e.removeAttribute;
  const setOwn = Object.getOwnPropertyDescriptor(e, "setAttribute");
  const removeOwn = Object.getOwnPropertyDescriptor(e, "removeAttribute");
  for (const k of invocationAttrs) remove.call(e, k);
  e.setAttribute = function(k, v) {
    k = String(k).toLowerCase();
    if (invocationAttrs.has(k)) values.set(k, String(v));
    else set.call(this, k, v);
  };
  e.removeAttribute = function(k) {
    k = String(k).toLowerCase();
    if (invocationAttrs.has(k)) values.set(k, null);
    else remove.call(this, k);
  };
  Invokers.set(e, () => {
    if (setOwn) Object.defineProperty(e, "setAttribute", setOwn);
    else delete e.setAttribute;
    if (removeOwn) Object.defineProperty(e, "removeAttribute", removeOwn);
    else delete e.removeAttribute;
    for (const [k, v] of values) {
      if (v == null) remove.call(e, k);
      else set.call(e, k, v);
    }
  });
}

export function oneActivation(e) {
  const click = e.click;
  e.click = function() {
    const outer = Invokers, nodes = [...outer.keys()];
    for (const restore of outer.values()) restore();
    Invokers = new Map();
    try { return click.call(this); }
    finally {
      for (const restore of Invokers.values()) restore();
      outer.clear();
      Invokers = outer;
      for (const node of nodes) muteInvoker(node);
    }
  };
  e.addEventListener("click", ev => {
    const target = ev.composedPath().find(n => n.localName === "button" || n.localName === "a" && n.hasAttribute("href"));
    if (target === e) return;
    muteInvoker(e);
    const scope = Invokers;
    setTimeout(() => {
      if (Invokers !== scope) return;
      for (const restore of scope.values()) restore();
      Invokers = new Map();
    });
  }, true);
}

// Root-relative app files belong to the page's published release (LLP 1038 D7), or to the build generation it
// booted (host/web/generations.mjs: a dev or drive server's `.exact/gen/<id>/` base).
let Release;
export const releaseAsset = (k, v) => (k === "src" || k === "poster") && /^\/(assets|deck|shaders)\//.test(v ?? "")
  && (Release ??= (() => {
    try { return /^\/\.exact\/root\/web\/releases\/[0-9a-f]{64}\/$|^\/(?:[^/]+\/)*\.exact\/gen\/[0-9a-f]{32}\/$/.test(new URL(document.baseURI).pathname); }
    catch { return false; }
  })()) ? "." + v : v;

// An image's `load` and `error` (LLP 1011 §2) as HTML `<img>` fires them, once per source, the error with its message (glue.js `imageEvents`); a symbol fires neither. A tinted raster paints
// through its CSS mask (element.rs `host_css`), a CORS fetch: from an origin that sends no CORS headers it paints nothing, so a CORS probe of the source decides. An adopted page's image may have settled before this attached.
const IMAGE_ERROR = "the image did not load", CORS_ERROR = "a tinted image from another origin needs CORS (Access-Control-Allow-Origin)", Probes = new Map(), srcOf = e => e.$why != null ? `why:${e.$why}` : e.currentSrc; // `$why`: a package picture's refusal (pictures.js), which an image's `error` says
export function imageEvent(e, kind, f, inflight) {
  const probe = src => Probes.get(src) ?? (inflight.n++, Probes.set(src, new Promise(ok => { Object.assign(new Image(), { crossOrigin: "anonymous", onload: () => ok(true), onerror: () => ok(false) }).src = src; }).finally(() => inflight.n--)).get(src));
  const settle = failed => { const src = srcOf(e); if (!e.isConnected || e.hasAttribute("data-symbol-path") || e.$settled?.[kind] === src) return; (failed || !getComputedStyle(e).maskImage?.includes("url(") || /^(data|blob):/.test(src) || new URL(src, location.href).origin === location.origin ? Promise.resolve(!failed) : probe(src)).then(ok => { if (srcOf(e) !== src || !e.isConnected || (e.$settled ??= {})[kind] === src) return; e.$settled[kind] = src; if (ok ? kind === "load" : kind === "error") ok ? f() : f(failed ? (e.$why ?? IMAGE_ERROR) : CORS_ERROR); }); };
  e.addEventListener("load", () => settle(false)); e.addEventListener("error", () => settle(true)); if (e.complete && e.getAttribute("src")) setTimeout(() => e.complete && settle(!e.naturalWidth));
}
