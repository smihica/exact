// A `video` on the JS target (LLP 1042): the web host's own media-glue.js,
// fetched after the first frame by a page with one (a loaded capability, as
// on the wasm host), applies its props — `paused`, `volume`, `currentTime`,
// `playbackRate`, `preservesPitch`, the visibility policy — and reports its
// events. This adapter hands it the node's props as the wasm host's
// `syncMedia` does (static ones from the element's attributes, by the
// plan's names) and hears its reports as `exact-media` events. A node the
// tree has ended is retired: its player stops and it reports nothing more,
// so a late `pause`, `timeupdate` or refused play never reaches whatever now
// holds its place (jukebox F1, F5, F6, F20).
import { onEnd, onMount, preparedScope, inflight, journal, data, clock } from "./rt.js"; import { liveById } from "./navigation.js";
import { loadOnce } from '../web/load-once.js';

const loads = {};

// The media session's actions (LLP 1098 D2, D6): the glue sends them as it
// sends the element's events, with `seekOffset seekTime fastSeek`.
const SESSION = ["seekbackward", "seekforward", "seekto", "previoustrack", "nexttrack", "stop"];
const BOOL = new Set(["autoplay", "controls", "loop", "muted", "playsinline", "disablepictureinpicture", "disableremoteplayback"]);
export const MEDIA_EVENTS = new Set(["loadedmetadata", "durationchange", "timeupdate", "play", "playing", "pause", "ended", "waiting", "seeking", "seeked", "ratechange", "volumechange", "error", "canplay", "fullscreenchange", ...SESSION]);
let Install = null;
// Nodes whose props changed: handed to the glue together after the commit
// that built or changed them, when they are in the document.
const Dirty = new Set();
const send = e => text => e.dispatchEvent(new CustomEvent("exact-media", { detail: text }));
function flush() {
  const later = [], batch = [...Dirty];
  Dirty.clear();
  for (const e of batch) if (!e.$media.retired) {
    if (e.isConnected) {
      try { Install(e, send(e)); holdUntilPlayable(e); }
      catch (err) { journal.push(`media: ${err.message}`); }
    } else later.push(e);
  }
  if (later.length) requestAnimationFrame(() => { for (const e of later) install(e); });
}
// A load or a seek in flight holds a settle (media-glue.js `mediaHold`, the wasm host's too), one hold at a time.
function holdUntilPlayable(e) {
  if (e.$media.hold) return;
  const held = globalThis.exact.mediaHold(e, () => { e.$media.hold = false; inflight.n--; });
  if (!held) return;
  e.$media.hold = true;
  inflight.n++;
  e.$media.release = held.release;
}
function enqueue(e) {
  if (e.$media.retired) return;
  if (!Dirty.size && Install) queueMicrotask(flush);
  Dirty.add(e);
  if (loads.Glue) return;
  inflight.n++;
  loadOnce(loads, "Glue", () => new Promise(r => requestAnimationFrame(r)).then(() => { globalThis.exact ??= {}; return import("./media-glue.js"); })
    .then(() => { Install = globalThis.exact.installMedia; flush(); }), err => { Dirty.clear(); journal.push(`media: unavailable: ${err.message}`); }).finally(() => inflight.n--);
}
function install(e) {
  if (e.$media.mounted) return enqueue(e);
  // Inside a prepared scope, defer until mount; an async resolution (e.g.
  // `appSource`) before mount leaves installation to `media()`'s mount hook.
  if (preparedScope()) onMount(() => enqueue(e), e.$media.installKey);
}
/** The glue on its way (the agent waits for it before an operation). */
export const mediaPiece = () => loads.Glue;
/** A `video` built or adopted with its static props (`h`). */
export function media(e, attrs) {
  if (typeof requestAnimationFrame !== "function" || globalThis.__exactRender) return;
  const props = {};
  for (const k in attrs) if (!k.startsWith("data-")) props[k] = BOOL.has(k) ? "true" : attrs[k];
  e.$media = {
    retired: false,
    mounted: false,
    app: null,
    appSeq: 0,
    appPending: false,
    appQueue: [],
    poster: attrs["data-app-poster"] ?? null,
    posterSeq: 0,
    installKey: {},
    appKey: {},
    posterKey: {},
    command: (name, seconds) => command(e, name, seconds),
  };
  e.exactMedia = { props, handlers: [] };
  onMount(() => { e.$media.mounted = true; });
  if (attrs["data-app-src"]) appSource(e, attrs["data-app-src"]);
  if (attrs["data-app-poster"]) appPoster(e, attrs["data-app-poster"]);
  const exitFloating = () => {
    if (document.pictureInPictureElement === e) document.exitPictureInPicture().catch(() => {});
  };
  // A route or document can leave while the browser still owns its floating window.
  const entered = () => e.$media.retired ? exitFloating() : window.addEventListener("pagehide", exitFloating);
  const left = () => window.removeEventListener("pagehide", exitFloating);
  e.addEventListener("enterpictureinpicture", entered);
  e.addEventListener("leavepictureinpicture", left);
  onEnd(() => {
    left();
    Dirty.delete(e);
    e.$media.retired = true;
    e.$media.appPending = false;
    e.$media.appQueue.length = 0;
    exitFloating();
    e.$media.release?.();
    globalThis.exact?.removeMedia?.(e);
    // As the wasm host retires a video: stopped, its source let go.
    e.pause(); e.removeAttribute("src"); e.load();
  });
  install(e);
}
/** A dynamic prop's value (`P`); true when it took the source itself (an `app:/` file). */
export function mediaProp(e, name, v) {
  if (!e.$media) return false;
  if (name === "src") {
    if (v?.startsWith("app:/")) { appSource(e, v); return true; }
    const changed = e.$media.app != null || e.exactMedia.props.src !== v;
    e.$media.app = null;
    e.$media.appPending = false;
    e.$media.appSeq++;
    e.$media.appQueue.length = 0;
    // Drop any queued `load` left from a previous source so it cannot reload the replacement URL.
    if (changed) {
      e.exactMedia.srcGen = (e.exactMedia.srcGen ?? 0) + 1;
      if (e.exactMedia.commands?.length) {
        e.exactMedia.commands = e.exactMedia.commands.filter(([c]) => c !== "load");
      }
    }
  }
  if (name === "poster") {
    e.$media.poster = v?.startsWith("app:/") ? v : null;
    if (e.$media.poster) { appPoster(e, v); return true; }
    e.$media.posterSeq++;
  }
  if (v == null) delete e.exactMedia.props[name]; else e.exactMedia.props[name] = v;
  install(e);
  return false;
}
// An `app:/` source (LLP 1069.002 D7): the app's own file, one its data
// module wrote to `app:/data` (a downloaded episode, podcast F19), as an
// object URL from the web host's picker glue (`appURL`), as an `image` shows
// one; counted in flight, so `clock settle` waits for it. A path with no
// file is handed to the element as written, which refuses it
// (`src-not-supported`), as HTML refuses a source it cannot fetch.

const appURL = v => (loadOnce(loads, "Files", () => import(new URL("./picker-glue.js", import.meta.url).href).then(() => globalThis.exact.appURL))).then(f => f(v, data.appId));
function appSource(e, v) {
  if (e.$media.app !== v) {
    e.$media.appQueue.length = 0;
    e.exactMedia.srcGen = (e.exactMedia.srcGen ?? 0) + 1;
    if (e.exactMedia.commands?.length) {
      e.exactMedia.commands = e.exactMedia.commands.filter(([c]) => c !== "load");
    }
  }
  e.$media.app = v;
  e.$media.appPending = true;
  const seq = ++e.$media.appSeq;
  onMount(() => {
    if (e.$media.appSeq !== seq || e.$media.retired) return;
    inflight.n++;
    appURL(v)
      .then(url => {
        if (e.$media.appSeq !== seq || e.$media.retired) return;
        e.$media.appPending = false;
        if (e.getAttribute("src") !== (url || v)) e.setAttribute("src", url || v);
        e.exactMedia.props.src = url || v;
        if (e.$media.appQueue.length) (e.exactMedia.commands ??= []).push(...e.$media.appQueue.splice(0));
        install(e);
      })
      .catch(err => {
        if (e.$media.appSeq === seq) {
          e.$media.appPending = false;
          e.$media.appQueue.length = 0;
        }
        journal.push(`media: ${v}: ${err.message}`);
      }).finally(() => inflight.n--);
  }, e.$media.appKey);
}
// A poster that is the app's own file: shown once it resolves, none if there is none.
function appPoster(e, v) {
  const seq = ++e.$media.posterSeq;
  onMount(() => {
    if (e.$media.posterSeq !== seq || e.$media.retired) return;
    inflight.n++;
    appURL(v).then(url => { if (e.$media.posterSeq === seq && !e.$media.retired) { if (url) e.setAttribute("poster", url); else e.removeAttribute("poster"); } })
      .catch(() => {}).finally(() => inflight.n--);
  }, e.$media.posterKey);
}
// `fastSeek(id, seconds)` and `load(id)` (commands.js): queued for the glue,
// which runs them in order once it has the element. A `load` of an `app:/`
// source resolves the file again first: one written since shows; any later
// commands issued while the lookup is in flight wait behind it in FIFO order.
function command(e, name, seconds) {
  if (e.$media.retired) return;
  if (name === "load" && e.$media.app) {
    e.$media.appQueue.push([name, seconds]);
    appSource(e, e.$media.app);
    return;
  }
  if (e.$media.appPending) {
    e.$media.appQueue.push([name, seconds]);
    return;
  }
  (e.exactMedia.commands ??= []).push([name, seconds]);
  install(e);
}
/** A media event's handler (`on`): its payload, a number for the two that
 * carry one; a session action's `MediaSessionActionDetails` as the trailing
 * record (`fastSeek` the token "1", the times numbers), as `scroll`'s. */
export function mediaOn(e, kind, f) {
  e.exactMedia.handlers.push(kind);
  e.addEventListener("exact-media", ev => {
    const at = ev.detail.indexOf("\n"), name = ev.detail.slice(0, at), payload = ev.detail.slice(at + 1);
    if (name !== kind || e.$media.retired) return;
    if (SESSION.includes(kind)) { const [offset, time, fast] = payload.split(" "); f([kind, Number(offset), Number(time), fast === "1"]); }
    else if (kind === "timeupdate" || kind === "durationchange") f(Number(payload)); else if (kind === "error") f(payload); else if (kind === "fullscreenchange") f(payload === "true"); else f();
  });
}

// `requestFullscreen("id")` (rt.js's Hosts): the video with that HTML id takes
// the screen, as HTML's Element.requestFullscreen(); the glue reports
// `fullscreenchange`.
export const requestFullscreen = id => {
  const el = liveById(String(id ?? ""));
  if (!el?.$media) return journal.push(`t=${clock.now} requestFullscreen: refused: no video with id "${id}"`);
  el.requestFullscreen().catch(e => journal.push(`t=${clock.now} requestFullscreen: refused: ${e.name}`));
};
// What rt.js reaches through `useMedia`, installed by the generated module only where its plan has media.
export const mediaUse = () => ({ media, mediaProp, mediaOn, mediaPiece, MEDIA_EVENTS, requestFullscreen });
