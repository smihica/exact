// Mount hooks inside owned-task prepared regions wait until their fragment
// enters the document (#2105): when a commit's tree update is deferred into
// `document.startViewTransition` (route transitions, `sharedElement`, or
// `startViewTransition()`), warm observers, pictures, GPU bindings and
// followers must not abort or measure against the detached fragment.
import { expect, test, beforeAll, afterAll } from 'bun:test';
import { copyFileSync, mkdirSync, mkdtempSync, readdirSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { suiteBrowser } from '../../scripts/agent-launch.mjs';
import { webCompiler } from './module.mjs';
import { engineTests } from '../../scripts/coverage.mjs';
import { scrollFollowers } from '../web/navigation.js';

// Stage an isolated copy of host/web-js with navigation.js and names.js so
// process-global `mock.module('./rt.js', ...)` in sibling test files cannot
// collide with these runtime unit tests.
const root = resolve(import.meta.dir, '../..');
const rtStage = mkdtempSync(resolve(tmpdir(), 'exact-prepared-rt-'));
const rtWebJs = resolve(rtStage, 'web-js');
mkdirSync(rtWebJs);
symlinkSync(resolve(root, 'host/web'), resolve(rtStage, 'web'));
for (const f of readdirSync(resolve(root, 'host/web-js'))) {
  if (f.endsWith('.js') && !f.includes('.test.')) {
    copyFileSync(resolve(root, 'host/web-js', f), resolve(rtWebJs, f));
  }
}
writeFileSync(resolve(rtWebJs, 'navigation.js'), "export * from '../web/navigation.js';\n");
writeFileSync(resolve(rtWebJs, 'names.js'), 'export const sourceTypes = {};\n');
writeFileSync(resolve(rtWebJs, 'symbol-roles.js'), 'export default { star: ["M0 0h10v10H0z", 1] };\n');
writeFileSync(resolve(rtWebJs, 'picker-glue.js'), 'globalThis.exact ??= {}; globalThis.exact.appURL = (v) => globalThis.__mockAppURL(v);\n');

const stagedImport = name => import(pathToFileURL(resolve(rtWebJs, name)).href);
const {
  scope,
  onMount,
  onEnd,
  mounted,
  end,
  sig,
  effect,
  commit,
  W,
  After,
  PropHooks,
  inflight,
  Views,
  viewId,
  gs,
  clock,
  Scrolls,
  journal,
} = await stagedImport('rt.js');
const { P } = await stagedImport('tree.js');
const { packagePictures } = await stagedImport('pictures.js');
const { endFollow } = await stagedImport('follow.js');
const { media, mediaProp } = await stagedImport('media.js');
const { symbols } = await stagedImport('symbols.js');
const { imageEvent } = await stagedImport('elements.js');
const { gsc } = await stagedImport('pieces.js');

const engineTest = await engineTests(['chromium']);

const TINY_SVG = 'data:image/svg+xml,' + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="16" height="16" fill="red"/></svg>');

const SOURCE = `routes nav
  tab home "/"
  tab tabA "/a"
  tab tabB "/b"
  item "/item"

keyframes enter
  from opacity=0
  to opacity=1
keyframes leave
  from opacity=1
  to opacity=0

style Slides
  ::view-transition-new
    animation="enter 80ms linear both"
  ::view-transition-old
    animation="leave 80ms linear both"

component App
  state shared = false
  state tickOn = true
  state popSeen = false
  state imgLoaded = false
  state showList = false
  state rows = ["r1"]
  derive current = top(nav)
  action goItem
    nav = push(nav, path("item"))
  action goBack
    popSeen = true
    nav = back(nav)
  action toggleShared
    shared = not shared
  action fadeToggle
    startViewTransition()
    shared = not shared
  action fadeShowList
    startViewTransition()
    showList = true
  action addRow
    rows = rows.concat(["r2"])
  action fadeTab(name: string)
    startViewTransition()
    nav = select(nav, name)
  action pickTab(name: string)
    nav = select(nav, name)
  action onImgLoad
    imgLoaded = true
  view
    main navigationKey=\`\${current.id}\` navigationBack="goBack" width="100%" height="100%" position="relative" overflow="hidden"
      column position="absolute" top=160 bottom=0 left=0 right=0
        for t in nav.tabs key=t.name
          column role="tabpanel" id=\`panel-\${t.name}\` position="absolute" inset=0
            for e in t.stack key=e.id
              if e.name == "item"
                column navigationKey=\`\${e.id}\` class=Slides testId="route-item" position="absolute" inset=0 background-color="-exact-background"
                  Probe(name="in-route")
                  image "symbol:star" width=20 height=20 testId="route-icon"
                  image "${TINY_SVG}" width=16 height=16 load=onImgLoad testId="route-img"
              else if e.name == "tabA"
                column navigationKey=\`\${e.id}\` testId="route-tabA" position="absolute" inset=0 background-color="-exact-background"
                  Probe(name="tab-a")
              else if e.name == "tabB"
                column navigationKey=\`\${e.id}\` testId="route-tabB" position="absolute" inset=0 background-color="-exact-background"
                  Probe(name="tab-b")
              else
                column navigationKey=\`\${e.id}\` class=Slides testId="route-home" position="absolute" inset=0 background-color="-exact-background"
                  text "home"
                  if popSeen
                    Probe(name="after-pop")
      column position="absolute" top=0 left=0 right=0 height=160
        if tickOn
          Tick()
        row gap=4
          button "Go Item" press=goItem testId="go-item"
          button id="goBack" "Back" press=goBack testId="go-back"
          button "Share" press=toggleShared testId="toggle-share"
          button "Fade" press=fadeToggle testId="fade-toggle"
          button "Fade A" press=fadeTab("tabA") testId="fade-tab-a"
          text \`\${imgLoaded ? "loaded" : "pending"}\` testId="img-state"
          image "${TINY_SVG}" width=16 height=16 testId="warm-img"
          Probe(name="warm")
        row gap=4
          button "Fade List" press=fadeShowList testId="fade-list"
          button "Add Row" press=addRow testId="add-row"
          if showList
            for item in rows key=item
              row gap=4
                text item testId=\`row-\${item}\`
                Probe(name=\`row-\${item}\`)
          if current.name == "item"
            Probe(name="sidebar-on-route")
          if shared
            row gap=8
              box sharedElement="pill" -exact-layout-transition="80ms ease" width=40 height=20
              image "symbol:star" width=20 height=20 testId="shared-icon"
              Probe(name="after-shared")
          else
            box sharedElement="pill" -exact-layout-transition="80ms ease" width=20 height=20
        row role="tablist" gap=8
          button role="tab" aria-controls="panel-home" aria-selected=(nav.tab == "home") press=pickTab("home") testId="tab-home" "Home"
          button role="tab" aria-controls="panel-tabA" aria-selected=(nav.tab == "tabA") press=pickTab("tabA") testId="tab-a" "A"
          button role="tab" aria-controls="panel-tabB" aria-selected=(nav.tab == "tabB") press=pickTab("tabB") testId="tab-b" "B"

component Probe
  props
    name: string
  state hit = false
  state sized = false
  action onHit(entry: IntersectionObserverEntry)
    if entry.isIntersecting
      hit = true
  action onSize(w: number, h: number)
    if w > 0 and h > 0
      sized = true
  view
    box testId=\`probe-\${name}\` width=24 height=24 intersect=onHit resize=onSize
      text \`\${hit ? "hit" : "wait"}:\${sized ? "sized" : "unsized"}\` testId=\`state-\${name}\`

component Tick
  state n = 0
  derive ready = n < 0
  action inc
    n = n + 1
  task poll when ready
    every(1000, inc)
  view
    box width=1 height=1
`;

test('nested prepared scopes queue on the nearest scope and discard hooks on refusal', () => {
  const ran = [];
  const ended = [];
  let innerA, innerB, refused;
  const outer = scope(() => {
    onMount(() => {
      ran.push('outer');
      onEnd(() => ended.push('outer'));
    });
    innerA = scope(() => {
      onMount(() => {
        ran.push('innerA');
        onEnd(() => ended.push('innerA'));
      });
    }, undefined, true);
    innerB = scope(() => {
      onMount(() => ran.push('innerB'));
    }, undefined, true);
    refused = scope(() => {
      onMount(() => ran.push('refused'));
    }, undefined, true);
    // Simulate innerB mounting into outer's fragment before outer mounts,
    // while innerA stays detached (superseded tab) and refused is disposed.
    mounted(innerB);
    end(refused);
  }, null, true);

  expect(ran).toEqual([]);
  mounted(outer);
  expect(ran).toEqual(['outer', 'innerB']);
  // Mounting outer must not drain innerA while innerA's own DOM is still detached.
  mounted(innerA);
  expect(ran).toEqual(['outer', 'innerB', 'innerA']);
  end(outer);
  expect(ended).toEqual(['innerA', 'outer']);
});

test('dynamic package pictures wait for mount, survive intervening After passes, and measure real layout', async () => {
  const prev = {
    src: PropHooks.src,
    afterLen: After.length,
    document: globalThis.document,
    getComputedStyle: globalThis.getComputedStyle,
    createObjectURL: URL.createObjectURL,
    revokeObjectURL: URL.revokeObjectURL,
  };
  const requests = [];
  const attrs = new Map();
  let connected = false;
  const img = {
    localName: 'img',
    get isConnected() { return connected; },
    getAttribute: k => attrs.get(k) ?? null,
    setAttribute: (k, v) => attrs.set(k, String(v)),
    removeAttribute: k => attrs.delete(k),
    hasAttribute: k => attrs.has(k),
    dispatchEvent() {},
    getBoundingClientRect: () => (connected ? { width: 48, height: 32 } : { width: 0, height: 0 }),
  };
  try {
    globalThis.document = { querySelectorAll: () => (connected && attrs.has('data-exact-picture') ? [img] : []) };
    globalThis.getComputedStyle = () => ({ objectFit: 'cover' });
    URL.createObjectURL = () => 'blob:mock-picture';
    URL.revokeObjectURL = () => {};
    packagePictures(
      { swatch: 'ext.swatch' },
      () => null,
      () => ({
        async image(source, opts) {
          requests.push({ source, ...opts });
          return new Blob(['ok']);
        },
      }),
      () => null,
    );
    const afterPass = After[After.length - 1];
    const startInflight = inflight.n;

    const s = scope(() => {
      expect(PropHooks.src(img, 'swatch:red')).toBe(true);
    }, null, true);

    // Detached: no fetch started, no 512x512 fallback, and After cannot purge or lock it.
    expect(requests).toEqual([]);
    expect(inflight.n).toBe(startInflight);
    afterPass();
    expect(img.$picture).toBeUndefined();

    // Connect and publish mount hooks: fetches once at real layout dimensions.
    connected = true;
    mounted(s);
    await new Promise(r => setTimeout(r, 0));
    expect(requests).toEqual([{ source: 'swatch:red', width: 48, height: 32, fit: 'cover', owned: false }]);
    expect(attrs.get('src')).toBe('blob:mock-picture');
    expect(inflight.n).toBe(startInflight);
    end(s);
  } finally {
    PropHooks.src = prev.src;
    After.length = prev.afterLen;
    if (prev.document === undefined) delete globalThis.document; else globalThis.document = prev.document;
    if (prev.getComputedStyle === undefined) delete globalThis.getComputedStyle; else globalThis.getComputedStyle = prev.getComputedStyle;
    URL.createObjectURL = prev.createObjectURL;
    URL.revokeObjectURL = prev.revokeObjectURL;
  }
});

test('GPU binding defers cold queueing until mount and delivers every live commit in order', async () => {
  const prevExact = globalThis.exact;
  const delivered = [];
  const makeCanvas = () => ({ isConnected: false });
  try {
    // Cold path: detached prepared canvas must not enter pendingSurfaces before mount.
    globalThis.exact = {};
    const cold = makeCanvas();
    const sCold = scope(() => {
      gs(cold, 'cold-surface', () => [7]);
    }, null, true);
    expect(globalThis.exact.pendingSurfaces).toBeUndefined();
    cold.isConnected = true;
    mounted(sCold);
    expect(globalThis.exact.pendingSurfaces).toEqual([{ id: viewId(cold), name: 'cold-surface', values: [7], generation: 0 }]);
    end(sCold);

    // Warm path: pre-mount updates coalesce to the latest value on first insertion,
    // while live commits in the same turn each deliver in order.
    globalThis.exact = {
      gpu: {
        surface: (_id, _name, values) => delivered.push(values),
        destroy() {},
      },
    };
    const preparedWarm = makeCanvas();
    const prepCount = sig(0);
    const sPrep = scope(() => {
      gs(preparedWarm, 'prep-surface', () => prepCount());
    }, null, true);
    for (let i = 1; i <= 5; i++) commit(() => W(prepCount, i), `prep-${i}`);
    expect(delivered).toEqual([]);
    preparedWarm.isConnected = true;
    mounted(sPrep);
    await Promise.resolve();
    expect(delivered).toEqual([5]);
    delivered.length = 0;
    end(sPrep);
    Views.delete(viewId(preparedWarm));

    const warm = makeCanvas();
    warm.isConnected = true;
    const count = sig(0);
    const sWarm = scope(() => {
      gs(warm, 'warm-surface', () => count());
    }, null, false);
    commit(() => W(count, 1), 'first');
    commit(() => W(count, 2), 'second');
    await Promise.resolve();
    expect(delivered).toEqual([0, 1, 2]);
    end(sWarm);
    Views.delete(viewId(cold));
    Views.delete(viewId(warm));
  } finally {
    if (prevExact === undefined) delete globalThis.exact; else globalThis.exact = prevExact;
  }
});

test('scrollFollowEnd and authored scrollTop wait for mount across intervening settles', () => {
  const prevRO = globalThis.ResizeObserver;
  try {
    globalThis.ResizeObserver = class { observe() {} disconnect() {} };
    const hooks = { After: [], Before: [], Hosts: { scrollIntoView() {} }, PropHooks: {}, clock, onMount };
    const { F, settle } = endFollow(hooks, { liveById: () => null, scrollFollowers }, () => ({ scrollBehavior: 'auto' }));
    let connected = false;
    const el = {
      scrollTop: 0, scrollHeight: 400, clientHeight: 200, children: [], style: { scrollBehavior: '' },
      get isConnected() { return connected; },
      addEventListener() {},
      removeEventListener() {},
    };
    const s = scope(() => {
      expect(hooks.PropHooks.scrollFollowEnd(el, 'true')).toBe(true);
      hooks.PropHooks.scrollTop(el, '70');
    }, null, true);
    expect(F.followedScrolls.has(el)).toBe(false);
    // An intervening settle while detached must not drop the follower or consume the authored scroll ownership.
    settle();
    connected = true;
    mounted(s);
    el.scrollTop = 70;
    expect(F.followedScrolls.has(el)).toBe(true);
    settle();
    expect(el.scrollTop).toBe(70);
    end(s);
  } finally {
    if (prevRO === undefined) delete globalThis.ResizeObserver; else globalThis.ResizeObserver = prevRO;
  }
});

test('repeated prepared scroll updates coalesce in mounts and populate Scrolls on mount', () => {
  const scrollTop = sig(10);
  const scrollLeft = sig(5);
  const el = { localName: 'div', isConnected: false, style: {} };
  const s = scope(() => {
    P(el, 'scrollTop', () => scrollTop());
    P(el, 'scrollLeft', () => scrollLeft());
  }, null, true);

  for (let i = 1; i <= 50; i++) {
    commit(() => { W(scrollTop, i * 10); W(scrollLeft, i * 2); }, `scroll-${i}`);
  }
  // Both tree.js (Scrolls) and, when ResizeObserver is present, follow.js coalesce
  // per element/axis instead of growing with the 50 pre-mount updates.
  expect(s.mounts.size).toBeGreaterThanOrEqual(2);
  expect(s.mounts.size).toBeLessThanOrEqual(4);
  expect(Scrolls.has(el)).toBe(false);

  mounted(s);
  expect(Scrolls.get(el)).toEqual({ scrollTop: 500, scrollLeft: 100 });
  Scrolls.delete(el);
  end(s);
});

test('CSS-colored GPU surface coalesces pre-mount updates and package picture revokes URL on disposal', async () => {
  const prevExact = globalThis.exact;
  const prevCreate = URL.createObjectURL;
  const prevRevoke = URL.revokeObjectURL;
  const prevDoc = globalThis.document;
  const prevGCS = globalThis.getComputedStyle;
  const prevSrc = PropHooks.src;
  const prevAfter = After.length;
  const revoked = [];
  try {
    // 50 pre-mount updates to a CSS-colored GPU surface coalesce to 1 mount hook.
    const count = sig(0);
    const canvas = {
      isConnected: false,
      setAttribute() {},
      style: { setProperty() {} },
      addEventListener() {},
      removeEventListener() {},
    };
    const sGsc = scope(() => {
      gsc(canvas, 'tinted', () => [count()], [[0, 'red']]);
    }, null, true);
    for (let i = 1; i <= 50; i++) commit(() => W(count, i), `gsc-${i}`);
    expect(sGsc.mounts.size).toBe(1);
    end(sGsc);

    // Ending a package picture scope immediately revokes its blob URL without waiting for After.
    URL.createObjectURL = () => 'blob:pic-to-revoke';
    URL.revokeObjectURL = u => revoked.push(u);
    globalThis.document = { querySelectorAll: () => [] };
    globalThis.getComputedStyle = () => ({ objectFit: 'contain' });
    const attrs = new Map();
    const img = {
      localName: 'img',
      isConnected: true,
      getAttribute: k => attrs.get(k) ?? null,
      setAttribute: (k, v) => attrs.set(k, String(v)),
      removeAttribute: k => attrs.delete(k),
      hasAttribute: k => attrs.has(k),
      dispatchEvent() {},
      getBoundingClientRect: () => ({ width: 24, height: 24 }),
    };
    packagePictures({ rev: 'ext.rev' }, () => null, () => ({ async image() { return new Blob(['x']); } }), () => null);
    const sPic = scope(() => { PropHooks.src(img, 'rev:a'); }, null, false);
    await new Promise(r => setTimeout(r, 10));
    expect(attrs.get('src')).toBe('blob:pic-to-revoke');
    img.isConnected = false;
    end(sPic);
    expect(revoked).toEqual(['blob:pic-to-revoke']);
  } finally {
    PropHooks.src = prevSrc;
    After.length = prevAfter;
    URL.createObjectURL = prevCreate;
    URL.revokeObjectURL = prevRevoke;
    if (prevExact === undefined) delete globalThis.exact; else globalThis.exact = prevExact;
    if (prevDoc === undefined) delete globalThis.document; else globalThis.document = prevDoc;
    if (prevGCS === undefined) delete globalThis.getComputedStyle; else globalThis.getComputedStyle = prevGCS;
  }
});

test('bound symbol roles loaded while detached resolve cleanly on mount without unknown-role refusal', async () => {
  const prev = {
    src: PropHooks.src,
    afterLen: After.length,
    document: globalThis.document,
    window: globalThis.window,
    getComputedStyle: globalThis.getComputedStyle,
    addEventListener: globalThis.addEventListener,
    matchMedia: globalThis.matchMedia,
    MutationObserver: globalThis.MutationObserver,
  };
  const attrs = new Map();
  const styleProps = new Map();
  let connected = false;
  const img = {
    localName: 'img',
    get isConnected() { return connected; },
    getAttribute: k => attrs.get(k) ?? null,
    setAttribute: (k, v) => attrs.set(k, String(v)),
    removeAttribute: k => attrs.delete(k),
    hasAttribute: k => attrs.has(k),
    toggleAttribute: (k, on) => on ? attrs.set(k, '') : attrs.delete(k),
    addEventListener() {},
    style: {
      setProperty: (k, v) => styleProps.set(k, v),
      removeProperty: k => styleProps.delete(k),
      getPropertyValue: k => styleProps.get(k) ?? '',
    },
    classList: [],
    parentElement: null,
  };
  const startJournalLen = journal.length;
  try {
    globalThis.window = {};
    globalThis.addEventListener = () => {};
    globalThis.matchMedia = () => ({ addEventListener() {} });
    globalThis.MutationObserver = class { observe() {} };
    globalThis.document = {
      head: { append() {} },
      documentElement: {},
      createElement: () => ({}),
      getElementById: () => ({}),
      querySelectorAll: () => [],
      styleSheets: [],
    };
    globalThis.getComputedStyle = () => ({
      fontSize: '16px',
      fontWeight: '400',
      objectFit: 'contain',
      direction: 'ltr',
      getPropertyValue: () => '',
    });
    symbols({});
    const afterPass = After[After.length - 1];
    const s = scope(() => {
      expect(PropHooks.src(img, 'symbol:star')).toBe(true);
    }, null, true);

    // Wait for symbol-roles.js to finish loading while the img is still detached.
    await new Promise(r => setTimeout(r, 20));
    expect(attrs.get('data-symbol-path')).toBe('M0 0h10v10H0z');

    connected = true;
    mounted(s);
    afterPass();
    await new Promise(r => setTimeout(r, 10));
    expect(decodeURIComponent(styleProps.get('--exact-symbol-mask') ?? '')).toContain('M0 0h10v10H0z');
    expect(journal.slice(startJournalLen).some(l => l.includes('unknown symbol role'))).toBe(false);
    end(s);
  } finally {
    PropHooks.src = prev.src;
    After.length = prev.afterLen;
    if (prev.document === undefined) delete globalThis.document; else globalThis.document = prev.document;
    if (prev.window === undefined) delete globalThis.window; else globalThis.window = prev.window;
    if (prev.getComputedStyle === undefined) delete globalThis.getComputedStyle; else globalThis.getComputedStyle = prev.getComputedStyle;
    if (prev.addEventListener === undefined) delete globalThis.addEventListener; else globalThis.addEventListener = prev.addEventListener;
    if (prev.matchMedia === undefined) delete globalThis.matchMedia; else globalThis.matchMedia = prev.matchMedia;
    if (prev.MutationObserver === undefined) delete globalThis.MutationObserver; else globalThis.MutationObserver = prev.MutationObserver;
  }
});

test('synchronous throwing picture adapter releases inflight and does not drop sibling mount hooks', async () => {
  const prev = { src: PropHooks.src, afterLen: After.length };
  const attrs = new Map();
  const img = {
    localName: 'img',
    isConnected: true,
    getAttribute: k => attrs.get(k) ?? null,
    setAttribute: (k, v) => attrs.set(k, String(v)),
    removeAttribute: k => attrs.delete(k),
    hasAttribute: k => attrs.has(k),
    dispatchEvent() {},
    getBoundingClientRect: () => ({ width: 20, height: 20 }),
  };
  try {
    packagePictures({ boom: 'ext.boom' }, () => null, () => { throw new Error('adapter crashed'); }, () => null);
    const startInflight = inflight.n;
    const ran = [];
    const s = scope(() => {
      PropHooks.src(img, 'boom:one');
      onMount(() => ran.push('sibling'));
    }, null, true);
    mounted(s);
    await new Promise(r => setTimeout(r, 0));
    expect(ran).toEqual(['sibling']);
    expect(inflight.n).toBe(startInflight);
    end(s);
  } finally {
    PropHooks.src = prev.src;
    After.length = prev.afterLen;
  }
});

test('retired image load catch-up timer does not deliver to a ended scope and cold GPU teardown clears pendingSurfaces', async () => {
  const prevExact = globalThis.exact;
  const prevDoc = globalThis.document;
  try {
    // Cold GPU teardown before gpu-glue loads must drop its pendingSurfaces entry.
    globalThis.exact = {};
    const canvas = { isConnected: true };
    const sGpu = scope(() => { gs(canvas, 'temp-surface', () => [1]); }, null, true);
    mounted(sGpu);
    expect(globalThis.exact.pendingSurfaces?.length).toBe(1);
    end(sGpu);
    expect(globalThis.exact.pendingSurfaces?.length).toBe(0);
    Views.delete(viewId(canvas));

    // Retired image on an exit ghost (still connected) must not deliver catch-up load after end().
    let fired = 0;
    const img = {
      isConnected: true,
      complete: true,
      naturalWidth: 16,
      currentSrc: 'data:image/png;base64,AA==',
      getAttribute: k => k === 'src' ? 'data:image/png;base64,AA==' : null,
      hasAttribute: () => false,
      addEventListener() {},
    };
    const sImg = scope(() => {
      imageEvent(img, 'load', () => { fired++; }, inflight, onMount, onEnd);
    }, null, true);
    mounted(sImg);
    end(sImg);
    await new Promise(r => setTimeout(r, 20));
    expect(fired).toBe(0);
  } finally {
    if (prevExact === undefined) delete globalThis.exact; else globalThis.exact = prevExact;
    if (prevDoc === undefined) delete globalThis.document; else globalThis.document = prevDoc;
  }
});

test('media installation and appSource coalesce pre-mount updates and ignore stale lookups', async () => {
  const prevRaf = globalThis.requestAnimationFrame;
  const prevWin = globalThis.window;
  const prevDoc = globalThis.document;
  const prevMock = globalThis.__mockAppURL;
  let rafCount = 0;
  try {
    globalThis.requestAnimationFrame = () => { rafCount++; return rafCount; };
    globalThis.window = { addEventListener() {}, removeEventListener() {} };
    globalThis.document = { pictureInPictureElement: null };
    const makeVideo = () => {
      const attrs = new Map();
      return {
        attrs,
        isConnected: false,
        getAttribute: k => attrs.get(k) ?? null,
        setAttribute: (k, v) => attrs.set(k, String(v)),
        removeAttribute: k => attrs.delete(k),
        addEventListener() {},
        removeEventListener() {},
        pause() {},
        load() {},
      };
    };
    const startInflight = inflight.n;
    const discarded = makeVideo();
    const s1 = scope(() => {
      media(discarded, { 'data-app-src': 'app:/data/clip.mp4' });
      mediaProp(discarded, 'muted', 'true');
    }, null, true);
    expect(rafCount).toBe(0);
    expect(inflight.n).toBe(startInflight);
    end(s1);
    expect(rafCount).toBe(0);
    expect(inflight.n).toBe(startInflight);
    expect(discarded.$media.retired).toBe(true);

    // Cycle app:/a -> app:/b -> app:/a while detached: coalesced to 1 lookup on mount,
    // and an out-of-order stale response after a subsequent live switch never wins.
    const resolvers = [];
    globalThis.__mockAppURL = v => new Promise(res => resolvers.push({ v, res }));
    const live = makeVideo();
    const s2 = scope(() => {
      media(live, { 'data-app-src': 'app:/a' });
      mediaProp(live, 'src', 'app:/b');
      mediaProp(live, 'src', 'app:/a');
    }, null, true);
    expect(resolvers.length).toBe(0);
    live.isConnected = true;
    mounted(s2);
    await new Promise(r => setTimeout(r, 10));
    expect(resolvers.length).toBe(1);
    expect(resolvers[0].v).toBe('app:/a');
    // Switch live to app:/b and back to app:/a before the first lookup settles.
    mediaProp(live, 'src', 'app:/b');
    mediaProp(live, 'src', 'app:/a');
    await new Promise(r => setTimeout(r, 10));
    expect(resolvers.length).toBe(3);
    // Resolve newest first, oldest last: newest URL must stay.
    resolvers[2].res('blob:newest-a');
    await new Promise(r => setTimeout(r, 0));
    resolvers[0].res('blob:stale-a');
    resolvers[1].res('blob:stale-b');
    await new Promise(r => setTimeout(r, 0));
    expect(live.attrs.get('src')).toBe('blob:newest-a');
    // Two explicit load() commands against the same app:/ source while the first lookup
    // is pending must enqueue both load commands when the latest lookup resolves.
    live.$media.command('load');
    live.$media.command('load');
    await new Promise(r => setTimeout(r, 10));
    expect(resolvers.length).toBe(5);
    resolvers[4].res('blob:reloaded-a');
    await new Promise(r => setTimeout(r, 0));
    resolvers[3].res('blob:older-reload-a');
    await new Promise(r => setTimeout(r, 0));
    expect(live.attrs.get('src')).toBe('blob:reloaded-a');
    expect((live.exactMedia.commands ?? []).filter(c => c[0] === 'load').length).toBe(2);

    // Ordering: `load(); fastSeek(42)` while an app:/ lookup is in flight must preserve
    // FIFO order (`load` then `fastSeek`) instead of letting `fastSeek` overtake `load`.
    live.exactMedia.commands = [];
    let resolveLoadOrder;
    globalThis.__mockAppURL = () => new Promise(r => { resolveLoadOrder = r; });
    live.$media.command('load');
    live.$media.command('fastSeek', 42);
    await new Promise(r => setTimeout(r, 0));
    expect(live.exactMedia.commands).toEqual([]);
    resolveLoadOrder('blob:ordered-a');
    await new Promise(r => setTimeout(r, 0));
    expect(live.exactMedia.commands).toEqual([['load', undefined], ['fastSeek', 42]]);

    // Source switch to an HTTP URL after a lookup queued `load` in exactMedia.commands
    // must strip the stale `load` so it cannot reload the replacement URL.
    mediaProp(live, 'src', 'https://example.com/replacement.mp4');
    expect(live.exactMedia.commands.filter(c => c[0] === 'load')).toEqual([]);
    end(s2);
  } finally {
    if (prevRaf === undefined) delete globalThis.requestAnimationFrame; else globalThis.requestAnimationFrame = prevRaf;
    if (prevWin === undefined) delete globalThis.window; else globalThis.window = prevWin;
    if (prevDoc === undefined) delete globalThis.document; else globalThis.document = prevDoc;
    if (prevMock === undefined) delete globalThis.__mockAppURL; else globalThis.__mockAppURL = prevMock;
  }
});

test('reentrant commit during mount drain mounts remaining siblings before nested After and poison blocks queued deferred tails', async () => {
  const stageRtCopy = name => {
    const sub = resolve(rtStage, name);
    mkdirSync(sub, { recursive: true });
    for (const f of readdirSync(rtWebJs)) copyFileSync(resolve(rtWebJs, f), resolve(sub, f));
    return sub;
  };
  // Reentrant commit inside mount A must drain remaining hooks in BOTH the SAME
  // prepared scope and sibling scopes before nested After runs.
  const rtFresh = await import(pathToFileURL(resolve(stageRtCopy('reentrant-rt'), 'rt.js')).href);
  const order = [];
  const trigger = rtFresh.sig(0);
  let nestedDone = false;
  const afterFn = () => order.push(nestedDone ? 'outer-after' : 'nested-after');
  rtFresh.After.push(afterFn);
  try {
    const sA = rtFresh.scope(() => {
      rtFresh.onMount(() => {
        order.push('mount-A1');
        rtFresh.commit(() => {}, 'reentrant');
        nestedDone = true;
      });
      rtFresh.onMount(() => {
        order.push('mount-A2');
      });
    }, null, true);
    const sB = rtFresh.scope(() => {
      rtFresh.onMount(() => { order.push('mount-B'); });
    }, null, true);
    rtFresh.effect(() => {
      if (trigger() > 0) {
        rtFresh.mounted(sA);
        rtFresh.mounted(sB);
      }
    });
    rtFresh.commit(() => rtFresh.W(trigger, 1), 'mount-pair');
    expect(order).toEqual(['mount-A1', 'mount-A2', 'mount-B', 'nested-after', 'outer-after']);
    rtFresh.end(sA);
    rtFresh.end(sB);
  } finally {
    const idx = rtFresh.After.indexOf(afterFn);
    if (idx >= 0) rtFresh.After.splice(idx, 1);
  }

  // Throwing inside an effect in flush() must NOT run queued mount hooks before poisoning,
  // and a nested commit that poisons the runner must prevent outer finish(true) and commands.
  const rtNestedPoison = await import(pathToFileURL(resolve(stageRtCopy('nested-poison-1'), 'rt.js')).href);
  const prevErr1 = console.error;
  console.error = () => {};
  try {
    const ranHookAfterEffectThrow = [];
    const t1 = rtNestedPoison.sig(0);
    const sQueued = rtNestedPoison.scope(() => {
      rtNestedPoison.onMount(() => ranHookAfterEffectThrow.push('should-not-run'));
    }, null, true);
    rtNestedPoison.effect(() => { if (t1() === 1) rtNestedPoison.mounted(sQueued); });
    rtNestedPoison.effect(() => { if (t1() === 1) throw new Error('effect boom'); });
    const res1 = [];
    rtNestedPoison.commit(() => rtNestedPoison.W(t1, 1), 'effect-throws', ok => res1.push(ok));
    expect(res1).toEqual([false]);
    expect(ranHookAfterEffectThrow).toEqual([]);
  } finally {
    console.error = prevErr1;
  }

  const rtNestedPoison2 = await import(pathToFileURL(resolve(stageRtCopy('nested-poison-2'), 'rt.js')).href);
  const prevErr2 = console.error;
  console.error = () => {};
  try {
    const outerCmds = [];
    rtNestedPoison2.Hosts.ping = () => outerCmds.push('ping');
    const tOuter = rtNestedPoison2.sig(0), tInner = rtNestedPoison2.sig(0);
    rtNestedPoison2.effect(() => { if (tInner() === 1) throw new Error('inner poison'); });
    const sOuter = rtNestedPoison2.scope(() => {
      rtNestedPoison2.onMount(() => {
        rtNestedPoison2.commit(() => rtNestedPoison2.W(tInner, 1), 'inner');
      });
    }, null, true);
    rtNestedPoison2.effect(() => { if (tOuter() === 1) rtNestedPoison2.mounted(sOuter); });
    const outerRes = [];
    rtNestedPoison2.commit(() => { rtNestedPoison2.W(tOuter, 1); rtNestedPoison2.C('ping', []); }, 'outer', ok => outerRes.push(ok));
    expect(outerRes).toEqual([false]);
    expect(outerCmds).toEqual([]);
  } finally {
    console.error = prevErr2;
  }

  // Poisoning inside a deferred transition tail refuses queued subsequent commits without running commands.
  const poisonWebJs = stageRtCopy('poison-web-js');
  writeFileSync(resolve(poisonWebJs, 'presence-glue.js'), 'globalThis.exact ??= {}; globalThis.exact.presence = () => ({ before() {}, after() {}, exit() {} });\n');
  const rtPoison = await import(pathToFileURL(resolve(poisonWebJs, 'rt.js')).href);
  const prevDoc = globalThis.document, prevMM = globalThis.matchMedia, prevExact = globalThis.exact, prevErr = console.error, prevRaf = globalThis.requestAnimationFrame;
  const ranCmds = [];
  rtPoison.Hosts.ping = () => ranCmds.push('ping');
  console.error = () => {};
  try {
    let runUpdate;
    const style = { textContent: '' };
    globalThis.requestAnimationFrame = fn => { fn(); return 1; };
    globalThis.document = {
      head: { appendChild: el => el },
      createElement: () => style,
      getElementById: () => null,
      querySelectorAll: () => [],
      startViewTransition(cb) {
        runUpdate = cb;
        return {
          updateCallbackDone: Promise.resolve(),
          ready: Promise.resolve(),
          finished: Promise.resolve(),
          skipTransition() {},
        };
      },
    };
    globalThis.matchMedia = () => ({ matches: false });
    globalThis.exact = {};
    rtPoison.pr();
    await rtPoison.pieces();
    const boom = rtPoison.sig(0);
    const sBad = rtPoison.scope(() => {
      rtPoison.onMount(() => { throw new Error('boom mount'); });
    }, null, true);
    rtPoison.effect(() => { if (boom() === 1) rtPoison.mounted(sBad); });

    const results = [];
    rtPoison.commit(() => { rtPoison.C('startViewTransition', []); rtPoison.W(boom, 1); }, 'first', ok => results.push(['first', ok]));
    rtPoison.commit(() => { rtPoison.C('ping', []); }, 'second', ok => results.push(['second', ok]));
    runUpdate();
    expect(results).toEqual([['first', false], ['second', false]]);
    expect(ranCmds).toEqual([]);
  } finally {
    console.error = prevErr;
    if (prevRaf === undefined) delete globalThis.requestAnimationFrame; else globalThis.requestAnimationFrame = prevRaf;
    if (prevDoc === undefined) delete globalThis.document; else globalThis.document = prevDoc;
    if (prevMM === undefined) delete globalThis.matchMedia; else globalThis.matchMedia = prevMM;
    if (prevExact === undefined) delete globalThis.exact; else globalThis.exact = prevExact;
  }
});

test('retired pictures skip adapters, image switches clear template tint and refusal state, and exiting scroll followers clean up', async () => {
  const prevDoc = globalThis.document, prevStyle = globalThis.getComputedStyle;
  const prevCreate = URL.createObjectURL, prevRevoke = URL.revokeObjectURL;
  const prevRO = globalThis.ResizeObserver;
  const revoked = [];
  let urlSeq = 0;
  URL.createObjectURL = () => `blob:pic-${++urlSeq}`;
  URL.revokeObjectURL = u => revoked.push(u);
  const makeImg = () => {
    const attrs = new Map(), styles = new Map();
    const img = {
      localName: 'img',
      isConnected: true,
      attrs,
      styles,
      classList: [],
      style: {
        setProperty: (k, v) => styles.set(k, String(v)),
        getPropertyValue: k => styles.get(k) ?? '',
        removeProperty: k => styles.delete(k),
      },
      getAttribute: k => attrs.get(k) ?? null,
      setAttribute: (k, v) => attrs.set(k, String(v)),
      removeAttribute: k => attrs.delete(k),
      hasAttribute: k => attrs.has(k),
      closest: sel => (sel === '[data-exiting]' && attrs.has('data-exiting') ? img : null),
      getBoundingClientRect: () => ({ width: 24, height: 24 }),
      addEventListener() {},
      dispatchEvent() {},
    };
    return img;
  };
  try {
    globalThis.getComputedStyle = () => ({ objectFit: 'contain' });
    const imgs = [];
    globalThis.document = {
      head: { append() {} },
      styleSheets: [],
      createElement: () => ({ textContent: '' }),
      querySelectorAll: sel => (sel === '#exact-root img[data-exact-picture]' ? imgs : []),
    };

    // Bound picture retired before promise runs must never invoke instance().
    let instanceCalls = 0;
    let admitResult = null;
    const pics = packagePictures(
      { swatch: 'ext-swatch' },
      () => admitResult,
      () => { instanceCalls++; return { image: () => new Blob(['x']) }; },
      () => admitResult,
    );
    const retiredImg = makeImg();
    const sRet = scope(() => {
      PropHooks.src(retiredImg, 'swatch:red');
    }, null, true);
    mounted(sRet);
    end(sRet);
    await new Promise(r => setTimeout(r, 10));
    expect(instanceCalls).toBe(0);

    // Literal picture on an exiting ghost stays tracked while connected so permission
    // revocation clears its displayed raster without firing error, then forgets on disconnect.
    const literalImg = makeImg();
    let literalErrors = 0;
    literalImg.dispatchEvent = () => { literalErrors++; };
    literalImg.setAttribute('data-exact-picture', 'swatch:blue');
    imgs.push(literalImg);
    // Discover literal image while an unrelated mount scope is active; ending that unrelated
    // scope must NOT revoke the connected literal image!
    const unrelatedScope = scope(() => {
      for (const f of After) f();
    });
    await new Promise(r => setTimeout(r, 10));
    expect(instanceCalls).toBe(1);
    const blueUrl = literalImg.getAttribute('src');
    expect(blueUrl).toMatch(/^blob:pic-/);
    end(unrelatedScope);
    expect(revoked).not.toContain(blueUrl);
    expect(literalImg.getAttribute('src')).toBe(blueUrl);

    // Mark as exiting ghost: pictureChanged ignores it, revokePicture clears its `src`
    // without dispatching error, and disconnect cleans up tracking.
    literalImg.setAttribute('data-exiting', '');
    for (const f of After) f();
    expect(literalImg.getAttribute('src')).toBe(blueUrl);
    pics.pictureChanged('swatch:blue');
    await new Promise(r => setTimeout(r, 10));
    expect(instanceCalls).toBe(1);
    pics.revokePicture('swatch:blue');
    await new Promise(r => setTimeout(r, 10));
    expect(revoked).toContain(blueUrl);
    expect(literalImg.getAttribute('src')).toBeNull();
    expect(literalErrors).toBe(0);
    literalImg.isConnected = false;
    for (const f of After) f();

    // Switching tinted raster -> package picture clears template mask & offscreen object-position,
    // and switching away from a refused package picture clears $why.
    symbols({});
    const switchImg = makeImg();
    switchImg.style.setProperty('--exact-tint', 'red');
    const sSw = scope(() => {
      PropHooks.src(switchImg, 'https://example.com/a.png');
      expect(switchImg.styles.get('--exact-tint-offset')).toBe('-100000px 0');
      admitResult = 'denied';
      PropHooks.src(switchImg, 'swatch:blocked');
    }, null, true);
    mounted(sSw);
    expect(switchImg.styles.has('mask-image')).toBe(false);
    expect(switchImg.styles.has('--exact-tint-offset')).toBe(false);
    expect(switchImg.$why).toBe('denied');
    PropHooks.src(switchImg, 'https://example.com/b.png');
    expect(switchImg.$why).toBeNull();
    end(sSw);

    // Animated exit on a scrollFollowEnd scroller unregisters it on scope end and on disconnect.
    let roCallback;
    let disconnects = 0;
    globalThis.ResizeObserver = class {
      constructor(cb) { roCallback = cb; }
      observe() {}
      disconnect() { disconnects++; }
    };
    const fakeRt = { After: [], Before: [], Hosts: { scrollIntoView() {} }, PropHooks: {}, clock: { agent: false }, onMount: f => f(), onEnd };
    const { F } = endFollow(fakeRt, { liveById: () => null, scrollFollowers });
    const scroller = {
      isConnected: true,
      scrollTop: 0,
      scrollHeight: 200,
      clientHeight: 100,
      children: [],
      style: {},
      closest: () => null,
      addEventListener() {},
      removeEventListener() {},
    };
    const sFollow = scope(() => {
      fakeRt.PropHooks.scrollFollowEnd(scroller, 'true');
    });
    expect(F.followedScrolls.has(scroller)).toBe(true);
    end(sFollow);
    expect(F.followedScrolls.has(scroller)).toBe(false);
    expect(disconnects).toBeGreaterThan(0);

    // Disconnect-triggered ResizeObserver callback also unregisters without a commit.
    F.followScroll(scroller, true);
    expect(F.followedScrolls.has(scroller)).toBe(true);
    scroller.isConnected = false;
    roCallback();
    expect(F.followedScrolls.has(scroller)).toBe(false);

    // Stale CORS verdict from a previous raster must not cross an authored source switch
    // even while `currentSrc` still reports the previous URL.
    const prevImg = globalThis.Image, prevLoc = globalThis.location;
    let probeResolve;
    globalThis.location = { origin: 'https://app.example', href: 'https://app.example/' };
    globalThis.getComputedStyle = () => ({ objectFit: 'contain', maskImage: 'url("https://other.example/a.png")' });
    globalThis.Image = class {
      set src(_) { probeResolve = this; }
    };
    try {
      const corsImg = makeImg();
      const listeners = {};
      corsImg.addEventListener = (k, fn) => { listeners[k] = fn; };
      corsImg.currentSrc = 'https://other.example/a.png';
      corsImg.setAttribute('src', 'https://other.example/a.png');
      const corsEvents = [];
      const sCors = scope(() => {
        imageEvent(corsImg, 'error', msg => corsEvents.push(msg), inflight, onMount, onEnd);
      });
      listeners.load();
      // Author switches src to b.png while currentSrc still holds a.png!
      corsImg.$srcGen = (corsImg.$srcGen ?? 0) + 1;
      corsImg.setAttribute('src', 'https://other.example/b.png');
      probeResolve.onerror();
      await new Promise(r => setTimeout(r, 10));
      expect(corsEvents).toEqual([]);
      end(sCors);
    } finally {
      if (prevImg === undefined) delete globalThis.Image; else globalThis.Image = prevImg;
      if (prevLoc === undefined) delete globalThis.location; else globalThis.location = prevLoc;
    }
  } finally {
    URL.createObjectURL = prevCreate;
    URL.revokeObjectURL = prevRevoke;
    if (prevDoc === undefined) delete globalThis.document; else globalThis.document = prevDoc;
    if (prevStyle === undefined) delete globalThis.getComputedStyle; else globalThis.getComputedStyle = prevStyle;
    if (prevRO === undefined) delete globalThis.ResizeObserver; else globalThis.ResizeObserver = prevRO;
  }
});

test('concurrent picker-glue appURL lookups share the newest result and prepared openTabs survives nested tab mutations', async () => {
  // Concurrent appURL readers on the same path: first completing before second must wait
  // for the newer read rather than returning an empty cached string.
  const pickerDir = mkdtempSync(resolve(tmpdir(), 'exact-picker-appurl-'));
  const prevExact = globalThis.exact, prevCreate = URL.createObjectURL, prevRevoke = URL.revokeObjectURL;
  let createdCount = 0;
  URL.createObjectURL = () => `blob:shared-${++createdCount}`;
  URL.revokeObjectURL = () => {};
  try {
    const resolves = [];
    writeFileSync(resolve(pickerDir, 'storage-environment.js'), 'export const storageKey = () => "k";\n');
    writeFileSync(resolve(pickerDir, 'storage-fs.js'), `
      export const createFileStore = () => ({
        stat: async () => ({}),
        blob: () => new Promise(r => globalThis.__pickerResolves.push(r)),
        close() {},
      });
    `);
    copyFileSync(resolve(root, 'host/web/picker-glue.js'), resolve(pickerDir, 'picker-glue.js'));
    globalThis.__pickerResolves = resolves;
    globalThis.exact = {};
    await import(pathToFileURL(resolve(pickerDir, 'picker-glue.js')).href);
    const p1 = globalThis.exact.appURL('app:/data/shared.png', 'app');
    const p2 = globalThis.exact.appURL('app:/data/shared.png', 'app');
    await new Promise(r => setTimeout(r, 10));
    expect(resolves.length).toBe(2);
    // First lookup resolves BEFORE second lookup completes: must share second lookup's URL!
    resolves[0]({ blob: new Blob(['v1']), version: 1 });
    await new Promise(r => setTimeout(r, 10));
    resolves[1]({ blob: new Blob(['v2']), version: 2 });
    const [u1, u2] = await Promise.all([p1, p2]);
    expect(u1).toBe('blob:shared-1');
    expect(u2).toBe('blob:shared-1');
  } finally {
    delete globalThis.__pickerResolves;
    URL.createObjectURL = prevCreate;
    URL.revokeObjectURL = prevRevoke;
    if (prevExact === undefined) delete globalThis.exact; else globalThis.exact = prevExact;
    rmSync(pickerDir, { recursive: true, force: true });
  }

  // openTabs(): a tab panel's build/mount hook synchronously removing a sibling waiting panel
  // must not throw TypeError on `waiting.prepared` or poison the runner.
  const { tp } = await stagedImport('document.js');
  const rtCore = await stagedImport('rt.js');
  const makePanel = () => ({
    parentElement: null,
    childNodes: [],
    hasAttribute: () => false,
    getAttribute: () => null,
    cloneNode: () => ({ childNodes: [] }),
    append(...nodes) { this.childNodes.push(...nodes); },
  });
  const pA = makePanel(), pB = makePanel();
  const opened = [];
  let sTabB;
  const sTabA = scope(() => {
    tp(pA, () => {
      opened.push('A');
      end(sTabB);
    });
  });
  sTabB = scope(() => {
    tp(pB, () => {
      opened.push('B');
    });
  });
  commit(() => {}, 'open-tabs');
  expect(opened).toEqual(['A']);
  expect(rtCore.Poisoned).toBe(false);
  end(sTabA);

  // Reentrant commit during child teardown inside dispose() while owner's Waiting.delete
  // has not run yet must remove the gone panel instead of spinning openTabs().
  const pGone = makePanel();
  let childDisposed = false;
  const sGone = scope(() => {
    scope(() => {
      onEnd(() => {
        if (!childDisposed) {
          childDisposed = true;
          commit(() => {}, 'reentrant-during-dispose');
        }
      });
    });
    tp(pGone, () => { opened.push('should-not-open'); });
  });
  end(sGone);
  expect(opened).toEqual(['A']);
  expect(rtCore.Poisoned).toBe(false);
});

test('v7 regressions: same-flush GPU order, immediate mount poison, dynamic ghost picture revocation, CORS same-URL/app:/, and media-glue batch filtering', async () => {
  const prev = {
    exact: globalThis.exact, document: globalThis.document, window: globalThis.window,
    addEventListener: globalThis.addEventListener, matchMedia: globalThis.matchMedia,
    requestAnimationFrame: globalThis.requestAnimationFrame,
    getComputedStyle: globalThis.getComputedStyle, Image: globalThis.Image, location: globalThis.location,
    MutationObserver: globalThis.MutationObserver, create: URL.createObjectURL, revoke: URL.revokeObjectURL,
    src: PropHooks.src, afterLen: After.length, mockAppURL: globalThis.__mockAppURL,
  };
  const stage = mkdtempSync(resolve(rtStage, 'v7-'));
  try {
    // 1. Same-flush GPU update after mounted(s) clears deferMount must publish [2], not [2, 0].
    const delivered = [];
    globalThis.exact = { gpu: { surface(_id, _name, v) { delivered.push(v[0]); }, destroy() {} } };
    const show = sig(false), val = sig(0), canvas = { isConnected: true };
    let childScope = null;
    const hostScope = scope(() => {
      childScope = scope(() => { gs(canvas, 'same-flush', () => [val()]); }, null, true);
      effect(() => { if (show()) mounted(childScope); });
    });
    commit(() => { W(show, true); W(val, 2); }, 'gpu-same-flush');
    await new Promise(r => setTimeout(r, 0));
    expect(delivered).toEqual([2]);
    end(hostScope);
    if (childScope && !childScope.gone) end(childScope);
    Views.delete(viewId(canvas));

    // 2. Mount hook A throwing synchronously poisons immediately so sibling hook B cannot commit.
    for (const f of readdirSync(rtWebJs)) copyFileSync(resolve(rtWebJs, f), resolve(stage, f));
    const rtP = await import(pathToFileURL(resolve(stage, 'rt.js')).href);
    let hookBCommitted = false, hookBResult = null;
    const sP = rtP.scope(() => {
      rtP.onMount(() => { throw new Error('hook-a-boom'); });
      rtP.onMount(() => { hookBCommitted = true; hookBResult = rtP.commit(() => {}, 'hook-b'); });
    }, null, true);
    const triggerP = rtP.sig(0);
    const sOuter = rtP.scope(() => { rtP.effect(() => { if (triggerP() > 0) rtP.mounted(sP); }); });
    expect(rtP.commit(() => rtP.W(triggerP, 1), 'outer-mount-throw')).toBe(false);
    expect(hookBCommitted).toBe(false);
    expect(hookBResult).toBeNull();
    expect(rtP.Poisoned).toBe(true);
    rtP.end(sOuter);

    // 3. Dynamically bound picture whose scope ends BEFORE leave() sets [data-exiting]:
    //    revokePicture clears src without firing error, and animation removal revokes URL via MutationObserver.
    let seq = 0;
    const revoked = [], observers = [];
    globalThis.MutationObserver = class {
      constructor(cb) { this.cb = cb; this.disconnected = false; observers.push(this); }
      observe() {}
      disconnect() { this.disconnected = true; }
    };
    URL.createObjectURL = () => `blob:dyn-ghost-${++seq}`;
    URL.revokeObjectURL = u => revoked.push(u);
    globalThis.document = { querySelectorAll: () => [] };
    globalThis.getComputedStyle = () => ({ objectFit: 'contain' });
    const makeDynImg = () => {
      const attrs = new Map();
      const img = {
        localName: 'img', isConnected: true, exiting: false, errors: 0, parentNode: {},
        getAttribute: k => attrs.get(k) ?? null, setAttribute: (k, v) => attrs.set(k, String(v)),
        removeAttribute: k => attrs.delete(k), hasAttribute: k => attrs.has(k),
        closest: sel => (sel === '[data-exiting]' && img.exiting ? img : null),
        dispatchEvent: ev => { if (ev.type === 'error') img.errors++; },
        getBoundingClientRect: () => ({ width: 20, height: 20 }),
      };
      return img;
    };
    const api = packagePictures({ dyn: 'ext.dyn' }, () => null, () => ({ async image() { return new Blob(['x']); } }), () => null);
    const afterPic = After[After.length - 1];
    const imgA = makeDynImg(), sA = scope(() => { PropHooks.src(imgA, 'dyn:a'); }, null, false);
    await new Promise(r => setTimeout(r, 15));
    expect(imgA.getAttribute('src')).toBe('blob:dyn-ghost-1');
    end(sA); imgA.exiting = true; imgA.setAttribute('data-exiting', ''); afterPic();
    expect(revoked).toEqual([]);
    api.revokePicture('dyn:a');
    await new Promise(r => setTimeout(r, 0));
    expect(imgA.getAttribute('src')).toBeNull();
    expect(imgA.errors).toBe(0);
    expect(revoked).toEqual(['blob:dyn-ghost-1']);
    const imgB = makeDynImg(), sB = scope(() => { PropHooks.src(imgB, 'dyn:b'); }, null, false);
    await new Promise(r => setTimeout(r, 15));
    expect(imgB.getAttribute('src')).toBe('blob:dyn-ghost-2');
    end(sB); imgB.exiting = true; imgB.setAttribute('data-exiting', ''); afterPic();
    expect(observers.length).toBeGreaterThan(0);
    imgB.isConnected = false; observers.at(-1).cb();
    expect(revoked).toEqual(['blob:dyn-ghost-1', 'blob:dyn-ghost-2']);
    expect(observers.at(-1).disconnected).toBe(true);

    // 4. P() same-URL reevaluation keeps in-flight CORS verdict, and switching to app:/
    //    clears old src/mask immediately so a late load cannot trigger a stale CORS probe.
    const probes = [];
    globalThis.Image = class { set src(v) { this._src = v; probes.push(this); } get src() { return this._src; } };
    globalThis.location = { href: 'https://app.example/', origin: 'https://app.example' };
    globalThis.window = { addEventListener() {}, removeEventListener() {} };
    globalThis.addEventListener = () => {};
    globalThis.matchMedia = () => ({ addEventListener() {} });
    globalThis.document = {
      head: { append() {} }, documentElement: {}, createElement: () => ({}),
      getElementById: () => ({}), querySelectorAll: () => [], styleSheets: [],
      hidden: false, addEventListener() {}, removeEventListener() {}, pictureInPictureElement: null,
    };
    globalThis.getComputedStyle = el => ({
      maskImage: el.$template ? 'url("https://other.example/x.png")' : 'none',
      objectFit: 'contain', fontSize: '16px', fontWeight: '400', direction: 'ltr', getPropertyValue: () => '',
    });
    symbols({});
    const styleMap = new Map([['--exact-tint', 'red']]), attrs = new Map(), listeners = { load: [], error: [] };
    const imgCors = {
      localName: 'img', isConnected: true, currentSrc: 'https://other.example/keep.png', classList: [],
      style: { getPropertyValue: k => styleMap.get(k) ?? '', setProperty: (k, v) => styleMap.set(k, v), removeProperty: k => styleMap.delete(k) },
      getAttribute: k => attrs.get(k) ?? null, setAttribute: (k, v) => attrs.set(k, String(v)),
      removeAttribute: k => attrs.delete(k), hasAttribute: k => attrs.has(k),
      addEventListener: (k, fn) => { (listeners[k] ??= []).push(fn); },
    };
    let resolveApp;
    globalThis.__mockAppURL = () => new Promise(r => { resolveApp = r; });
    const urlSig = sig('https://other.example/keep.png'), bumpSig = sig(0);
    let corsLoads = 0, corsErrors = 0;
    const sCors = scope(() => {
      P(imgCors, 'src', () => { bumpSig(); return urlSig(); });
      imageEvent(imgCors, 'load', () => corsLoads++, inflight, onMount, onEnd);
      imageEvent(imgCors, 'error', () => corsErrors++, inflight, onMount, onEnd);
    });
    imgCors.$template = true;
    for (const fn of listeners.load) fn();
    expect(probes.length).toBe(1);
    commit(() => W(bumpSig, 1), 'same-url');
    probes[0].onload();
    await new Promise(r => setTimeout(r, 0));
    expect(corsLoads).toBe(1);
    expect(corsErrors).toBe(0);
    commit(() => W(urlSig, 'app:/data/pic.png'), 'to-app-src');
    expect(imgCors.getAttribute('src')).toBeNull();
    expect(imgCors.$template).toBe(false);
    // Catch-up timer firing after src was cleared for app:/ must not report false load failure.
    imgCors.complete = true;
    imgCors.naturalWidth = 0;
    for (const fn of listeners.load) fn();
    await new Promise(r => setTimeout(r, 10));
    expect(probes.length).toBe(1);
    expect(corsErrors).toBe(0);
    resolveApp?.('');
    await new Promise(r => setTimeout(r, 0));
    end(sCors);

    // 5. Real media-glue.js: fastSeek(-1) error handler switching src drops queued load(),
    //    while newly appended commands wait for the next update so their props apply first.
    globalThis.requestAnimationFrame = () => 0;
    globalThis.window = { addEventListener() {}, removeEventListener() {} };
    globalThis.document = { hidden: false, addEventListener() {}, removeEventListener() {}, pictureInPictureElement: null };
    globalThis.exact = {};
    await import(pathToFileURL(resolve(root, 'host/web/media-glue.js')).href);
    let loadCalls = 0, loadedSrc = null;
    const mAttrs = new Map();
    const video = {
      localName: 'video', isConnected: true, readyState: 2, paused: true, muted: false, currentTime: 0,
      played: { length: 0 }, disablePictureInPicture: false, preservesPitch: true,
      pause() {}, play() { return Promise.resolve(); },
      load() { loadCalls++; loadedSrc = this.getAttribute('src'); },
      getAttribute: k => mAttrs.get(k) ?? null, setAttribute: (k, v) => mAttrs.set(k, String(v)),
      removeAttribute: k => mAttrs.delete(k), toggleAttribute: (k, on) => { if (on) mAttrs.set(k, ''); else mAttrs.delete(k); },
      addEventListener() {}, removeEventListener() {},
    };
    const sMed = scope(() => {
      media(video, { src: 'old.mp4' });
      video.exactMedia.handlers.push('error');
      video.exactMedia.commands = [['fastSeek', -1], ['load', undefined]];
    }, null, false);
    globalThis.exact.installMedia(video, msg => {
      if (msg.startsWith('error\n')) {
        mediaProp(video, 'src', 'replacement.mp4');
        video.setAttribute('src', 'replacement.mp4');
        mediaProp(video, 'currentTime', '10');
        video.exactMedia.commands.push(['fastSeek', 42]);
      }
    });
    expect(loadCalls).toBe(0);
    expect(loadedSrc).toBeNull();
    // Newly pushed fastSeek(42) stayed queued for the next update so currentTime=10 applies first, then fastSeek(42).
    expect(video.currentTime).toBe(0);
    globalThis.exact.installMedia(video, () => {});
    expect(video.currentTime).toBe(42);
    end(sMed);
  } finally {
    PropHooks.src = prev.src; After.length = prev.afterLen;
    URL.createObjectURL = prev.create; URL.revokeObjectURL = prev.revoke;
    for (const k of ['exact', 'document', 'window', 'addEventListener', 'matchMedia', 'requestAnimationFrame', 'getComputedStyle', 'Image', 'location', 'MutationObserver']) {
      if (prev[k] === undefined) delete globalThis[k]; else globalThis[k] = prev[k];
    }
    if (prev.mockAppURL === undefined) delete globalThis.__mockAppURL; else globalThis.__mockAppURL = prev.mockAppURL;
    rmSync(stage, { recursive: true, force: true });
  }
});

let dir, server;
beforeAll(async () => {
  if (webCompiler().cmd === 'cargo') throw new Error('build the compiler first: cargo build -p exact-web-js');
  dir = mkdtempSync(resolve(root, 'target/prepared-observers-test-'));
  writeFileSync(resolve(dir, 'app.json'), JSON.stringify({ name: 'Prepared Observers', app: { id: 'test.prepared-observers', name: 'Prepared Observers' }, host: { web: {} } }));
  writeFileSync(resolve(dir, 'app.contract'), SOURCE);
  const dist = resolve(dir, 'dist');
  const build = Bun.spawn([process.execPath, 'host/web-js/build.mjs', 'prepared-observers', '--out', dist, '--render', 'none'], {
    cwd: root, env: { ...process.env, EXACT_APP_DIR: dir }, stdout: 'pipe', stderr: 'pipe',
  });
  const [status, stdout, stderr] = await Promise.all([build.exited, new Response(build.stdout).text(), new Response(build.stderr).text()]);
  expect({ status, error: status ? stdout + stderr : '' }).toEqual({ status: 0, error: '' });
  server = Bun.serve({ hostname: '127.0.0.1', port: 0, async fetch(req) {
    const path = new URL(req.url).pathname, file = Bun.file(resolve(dist, path === '/' || !path.includes('.') ? 'index.html' : path.slice(1)));
    return await file.exists() ? new Response(file) : new Response('not found', { status: 404 });
  } });
}, 120_000);
afterAll(() => {
  server?.stop(true);
  if (dir) rmSync(dir, { recursive: true, force: true });
  rmSync(rtStage, { recursive: true, force: true });
});

engineTest.chromium('warm observers, image load, and superseded tabs work when a view transition holds fragment insertion', async () => {
  const browser = await suiteBrowser('chromium');
  const context = await browser.newContext({ viewport: { width: 800, height: 600 } });
  const page = await context.newPage();
  const errors = [];
  page.on('pageerror', e => errors.push(e.message));
  await page.addInitScript(() => {
    const orig = document.startViewTransition?.bind(document);
    if (!orig) return;
    window.__holdVT = false;
    window.__vtPending = [];
    document.startViewTransition = cb => {
      if (!window.__holdVT) return orig(cb);
      let release;
      const gate = new Promise(r => { release = r; });
      window.__vtPending.push(release);
      return orig(async () => {
        await gate;
        return cb();
      });
    };
    window.__releaseVT = () => {
      for (const r of window.__vtPending.splice(0)) r();
    };
  });
  try {
    await page.goto(server.url.href);
    const waitState = (name, want) =>
      page.waitForFunction(({ name, want }) => document.querySelector(`[data-testid="state-${name}"]`)?.textContent === want, { name, want }, { timeout: 5000 });
    const flushMicrotasksWhileHeld = async () => {
      await page.waitForFunction(() => window.__vtPending.length > 0, { timeout: 5000 });
      await page.evaluate(() => new Promise(r => setTimeout(r, 20)));
    };
    await page.waitForFunction(() => typeof globalThis.exact?.afterCommit === 'function');
    // Warm the intersection and resize observer modules before the transitions so their
    // attachment microtasks would run before a held startViewTransition inserts the DOM.
    await waitState('warm', 'hit:sized');

    await page.evaluate(() => { window.__holdVT = true; });

    await page.click('[data-testid="go-item"]');
    await flushMicrotasksWhileHeld();
    expect(await page.evaluate(() => document.querySelector('[data-testid="state-in-route"]'))).toBeNull();
    await page.evaluate(() => window.__releaseVT());
    await waitState('in-route', 'hit:sized');
    await waitState('sidebar-on-route', 'hit:sized');
    await page.waitForFunction(() => document.querySelector('[data-testid="img-state"]')?.textContent === 'loaded', { timeout: 5000 });
    await page.waitForFunction(() => {
      const icon = document.querySelector('[data-testid="route-icon"]');
      return icon?.hasAttribute('data-symbol-path') && icon.style.getPropertyValue('--exact-symbol-mask').includes('url(');
    }, { timeout: 5000 });

    await page.click('[data-testid="go-back"]');
    await flushMicrotasksWhileHeld();
    await page.evaluate(() => window.__releaseVT());
    await waitState('after-pop', 'hit:sized');

    await page.click('[data-testid="toggle-share"]');
    await flushMicrotasksWhileHeld();
    await page.evaluate(() => window.__releaseVT());
    await waitState('after-shared', 'hit:sized');
    await page.waitForFunction(() => {
      const icon = document.querySelector('[data-testid="shared-icon"]');
      return icon?.hasAttribute('data-symbol-path') && icon.style.getPropertyValue('--exact-symbol-mask').includes('url(');
    }, { timeout: 5000 });

    await page.evaluate(() => { window.__holdVT = false; });
    await page.click('[data-testid="toggle-share"]');
    await page.waitForFunction(() => !document.querySelector('[data-testid="state-after-shared"]'));
    await page.evaluate(() => { window.__holdVT = true; });
    await page.click('[data-testid="fade-toggle"]');
    await flushMicrotasksWhileHeld();
    await page.evaluate(() => window.__releaseVT());
    await waitState('after-shared', 'hit:sized');

    // Prepare tab A inside a held transition, switch to tab B before flush, then open tab A later.
    await page.click('[data-testid="fade-tab-a"]');
    await flushMicrotasksWhileHeld();
    await page.evaluate(() => document.querySelector('[data-testid="tab-b"]').click());
    await page.evaluate(() => new Promise(r => setTimeout(r, 20)));
    await page.evaluate(() => window.__releaseVT());
    await waitState('tab-b', 'hit:sized');
    expect(await page.evaluate(() => document.querySelector('[data-testid="state-tab-a"]'))).toBeNull();

    await page.evaluate(() => { window.__holdVT = false; });
    await page.click('[data-testid="tab-a"]');
    await waitState('tab-a', 'hit:sized');

    // Open a prepared `if` arm containing a top-level `for` list under a held transition,
    // add a second row before the transition releases, and verify both rows mount cleanly without poisoning.
    await page.evaluate(() => { window.__holdVT = true; });
    await page.click('[data-testid="fade-list"]');
    await flushMicrotasksWhileHeld();
    await page.evaluate(() => document.querySelector('[data-testid="add-row"]').click());
    await page.evaluate(() => new Promise(r => setTimeout(r, 20)));
    await page.evaluate(() => window.__releaseVT());
    await page.waitForFunction(() => document.querySelector('[data-testid="row-r1"]')?.textContent === 'r1'
      && document.querySelector('[data-testid="row-r2"]')?.textContent === 'r2', { timeout: 5000 });
    await waitState('row-r1', 'hit:sized');
    await waitState('row-r2', 'hit:sized');

    expect(errors).toEqual([]);
  } finally {
    await context.close();
  }
}, 60_000);
