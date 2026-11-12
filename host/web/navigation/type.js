import { viewParent } from "./lists.js";
import { root } from "./routes.js";
// @ref LLP 1069.000 D3, LLP 1121 D6 — the app's `setRootFontSize(px)`: `:root { font-size: <px> !important }`, an author rule over the root element's own size (the browser's setting, or the agent's
// `prefer root-font-size`), which can change beneath it; a percentage is a share of that size which follows it (CSS's `125%` of `medium`, or of the agent's substitute); `"medium"` removes the rule. The wasm runner checked the value; the JS target checks it here, by the runner's grammar (exact_num's `css_percent`) and its share, `host * ratio` as an f32 above 0.
const APP_ROOT = "exact-root-font-size", PERCENT = /^[+-]?(\d+|\d*\.\d+)(e[+-]?\d+)?%$/i;
export function appRootFontSize(value, say, platform = globalThis) {
  const doc = platform.document, host = platform.getComputedStyle ? beneathApp(platform, () => parseFloat(platform.getComputedStyle(doc.documentElement).fontSize) || 16) : 16;
  let rule = doc.getElementById(APP_ROOT);
  if (value === "medium") {
    return void rule?.remove();
  }
  const ratio = typeof value === "string" && PERCENT.test(value) ? parseFloat(value) / 100 : NaN, share = Math.fround(host * ratio), px = typeof value === "number" && Math.fround(value) > 0 && Number.isFinite(Math.fround(value));
  if (!px && !(share > 0 && Number.isFinite(share))) {
    return say?.(`setRootFontSize(${JSON.stringify(value) ?? ""}) refused: the root font size is a number of px above 0, a percentage of the host's above 0 ("125%"), or "medium"`);
  }
  if (!rule) {
    rule = doc.createElement("style");
    rule.id = APP_ROOT;
    doc.head.append(rule);
  }
  rule.textContent = px ? `:root{font-size:${value}px!important}` : `:root{font-size:calc(var(--exact-host-font-size, 100%) * ${ratio})!important}`;
  refitTexts();
}
// LLP 1116.001 D1.4: fit a text's min-content width to its room, down to half size (or the root size for text above twice the root), as kernel/src/layout/fit.rs does.
// CSS zoom scales text, runs, leading and tracking together; authored rows stay. Restyling clears the scale, then the next pass refits.
export function fitText(el) {
  el.style.zoom = "";
  const cs = getComputedStyle(el), px = (v) => parseFloat(v) || 0;
  if (cs.getPropertyValue("--exact-fit").trim() !== "shrink" || cs.display === "none" || !el.offsetWidth) {
    return;
  }
  // Layout pixels, through any transform above it (a route sliding in).
  const k = el.getBoundingClientRect().width / el.offsetWidth || 1;
  const inset = px(cs.paddingLeft) + px(cs.paddingRight) + px(cs.borderLeftWidth) + px(cs.borderRightWidth);
  const content = () => el.getBoundingClientRect().width / k - inset, box = content(), { width, flexShrink } = el.style;
  // A flex item that may shrink (`min-width: 0`) would shrink its min-content width back to the room it has.
  el.style.width = "min-content";
  el.style.flexShrink = "0";
  const natural = content();
  el.style.width = width;
  el.style.flexShrink = flexShrink;
  // Sized by its text, it has its container's room.
  const up = viewParent(el), ps = up && getComputedStyle(up);
  const room = box + .5 < natural || !up ? box : up.clientWidth - px(ps.paddingLeft) - px(ps.paddingRight) - Math.max(0, px(cs.marginLeft)) - Math.max(0, px(cs.marginRight));
  if (!(natural > room + .01)) {
    return;
  }
  const size = px(cs.fontSize), root = px(getComputedStyle(document.documentElement).fontSize);
  el.style.zoom = String(Math.max(size > 0 && root > 0 ? Math.min(.5, root / size) : .5, Math.max(0, room) / natural));
  zoomed.add(el);
}
// What `fitText` scaled: one whose row went (a bound `-exact-fit`) is let go.
let fitSeen = false, zoomed = new Set();
/** Whether a style the page writes fits a text: then every pass refits. */
export const noteFit = (css) => {
  if (!fitSeen && css?.includes("--exact-fit")) {
    fitSeen = true;
  }
};
/** Every fitted text again: after a tree update, a resize, the fonts' arrival. */
export function refitTexts() {
  if (!fitSeen) {
    return;
  }
  const fitted = new Set(document.querySelectorAll("[style*=\"--exact-fit\"]"));
  for (const el of zoomed) {
    if (!fitted.has(el)) {
      el.style.zoom = "";
      zoomed.delete(el);
    }
  }
  for (const el of fitted) {
    fitText(el);
  }
}
/** The JS target's: after each tree update (rt.js `After`), on resize and once the fonts are in. */
export function installFits(after) {
  if (fitSeen || typeof document === "undefined" || typeof addEventListener !== "function" || globalThis.__exactRender) {
    return;
  }
  fitSeen = true;
  after.push(refitTexts);
  addEventListener("resize", refitTexts);
  document.fonts?.ready.then(refitTexts);
}
// What `read` gives with the app's rule set aside: the host's own reading.
function beneathApp(platform, read) {
  const rule = platform.document.getElementById?.(APP_ROOT);
  if (!rule) {
    return read();
  }
  rule.disabled = true;
  try {
    return read();
  } finally {
    rule.disabled = false;
  }
}
export { beneathApp };
