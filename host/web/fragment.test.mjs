// fragment.js, the web's scroll to a fragment (LLP 1153 D5), over stand-in elements and real browser scrollers.
// A scroll by the person since navigation cancels it though the module arms later; a hold while pictures load ends at the app's own scroll, a
// newer navigation, or an entry that no longer shows the fragment; a fragment is all after the first `#`; and Back
// puts back the scrollers above the route as well as those in it, again after the browser's own scroll to the id.
import { test, expect, afterAll } from 'bun:test';
import { hostFixtureFile } from './tests/glue-fixture.mjs';
import { suiteBrowser } from '../../scripts/agent-launch.mjs';

class El {
  constructor(attrs = {}, parent = null) {
    this.attrs = new Map(Object.entries(attrs)); this.parentElement = parent; this.children = []; parent?.children.push(this);
    this.scrollTop = 0; this.scrollLeft = 0; this.scrollHeight = 0; this.clientHeight = 0; this.scrollWidth = 0; this.clientWidth = 0;
    this.listeners = []; this.aligned = 0; this.isConnected = true;
  }
  get id() { return this.attrs.get('id') ?? ''; }
  getAttribute(k) { return this.attrs.get(k) ?? null; }
  hasAttribute(k) { return this.attrs.has(k); }
  setAttribute(k, v) { this.attrs.set(k, v); }
  removeAttribute(k) { this.attrs.delete(k); }
  all() { return this.children.flatMap(c => [c, ...c.all()]); }
  querySelectorAll(sel) {
    if (sel === '*') return this.all();
    if (sel === 'img') return this.all().filter(e => e.attrs.has('img'));
    const name = sel.slice(1, -1);
    return this.all().filter(e => e.attrs.has(name));
  }
  contains(e) { for (let x = e; x; x = x.parentElement) if (x === this) return true; return false; }
  closest() { return null; }
  matches() { return false; }
  addEventListener(type, f, o) { this.listeners.push({ type, f, signal: o?.signal }); }
  fire(type) { for (const l of this.listeners) if (l.type === type && !l.signal?.aborted) l.f({}); }
  scrollIntoView() { this.aligned++; }
  scrollTo({ top, left }) { this.scrollTop = top; this.scrollLeft = left; }
  focus() {}
}
const page = new El(), root = new El({}, page), port = new El({}, root);
Object.assign(port, { scrollHeight: 4000, clientHeight: 400 });
const route = new El({ navigationKey: '1' }, port), target = new El({ id: 'moves' }, route), picture = new El({ img: '' }, route);
picture.complete = false;
// Test files share one process: the stand-in document and history are put back after this file.
const had = Object.fromEntries(['document', 'history', 'requestAnimationFrame'].map(k => [k, Object.getOwnPropertyDescriptor(globalThis, k)]));
afterAll(() => { for (const [k, d] of Object.entries(had)) if (d) Object.defineProperty(globalThis, k, d); else delete globalThis[k]; });
globalThis.document = { scrollingElement: page, activeElement: null };
globalThis.history = { state: { exact: 0, id: 1, url: '/item/7#moves' } };
const frames = [], frame = () => { for (const f of frames.splice(0)) f(); };
globalThis.requestAnimationFrame = f => frames.push(f);
const f = await import('./fragment.js');
const lines = [], log = l => lines.push(l), person = { personScrolls: 0 };

test('a fragment is everything after the first #', () => {
  expect(f.shows(1, 'a#b', { id: 1, url: '/item/7#a#b' })).toBe(true);
  expect(f.shows(1, 'a', { id: 1, url: '/item/7#a#b' })).toBe(false);
  expect(f.shows(1, '', { id: 1, url: '/item/7' })).toBe(true);
});

test("a scroll by the person since the navigation cancels its scroll, though the module armed after it", () => {
  person.personScrolls = 1;
  f.arm(root, { top: 1, fragment: 'moves', activation: 1, since: 0 }, person);
  f.scroll(root, 1, 'moves', log, 1);
  expect([target.aligned, lines.at(-1)]).toEqual([0, 'fragment #moves: the person scrolled first; the route stays where it is']);
});

test("the hold while pictures load ends at the app's own scroll, a newer navigation, or another entry shown", () => {
  const held = ends => {
    f.arm(root, { top: 1, fragment: 'moves', activation: 2, since: person.personScrolls }, person);
    f.scroll(root, 1, 'moves', log, 2);
    route.fire('load');
    expect(target.aligned).toBe(2); // the scroll, then the hold's own
    ends();
    route.fire('load');
    expect(target.aligned).toBe(2);
    target.aligned = 0; route.listeners = [];
  };
  held(() => f.cancel());
  held(() => f.arm(root, { top: 1, fragment: 'top', activation: 3, since: person.personScrolls }, person));
  held(() => { globalThis.history.state = { exact: 1, id: 1, url: '/item/7#top' }; });
  globalThis.history.state = { exact: 0, id: 1, url: '/item/7#moves' };
  held(() => { person.personScrolls++; });
});

test('Back puts back the scrollers above the route, as those in it', () => {
  picture.complete = true;
  port.scrollTop = 120;
  globalThis.history.state = { exact: 1, id: 1, url: '/item/7#other' };
  f.arm(root, { top: 1, fragment: 'other', activation: 4, pushed: true, since: person.personScrolls }, person); // entry 0 kept as it was
  port.scrollTop = 900;
  globalThis.history.state = { exact: 0, id: 1, url: '/item/7#moves' };
  f.arm(root, { top: 1, fragment: 'moves', activation: 5, traversal: true, since: person.personScrolls }, person);
  f.scroll(root, 1, 'moves', log, 5);
  expect(port.scrollTop).toBe(120);
  port.scrollTop = 900; // the browser's own, at its next frame's layout
  frame(); frame();
  expect(port.scrollTop).toBe(120);
});

async function inBrowser(run) {
  const server = Bun.serve({ hostname: '127.0.0.1', port: 0, fetch(request) {
    const path = new URL(request.url).pathname;
    if (path === '/fragment.js' || path === '/navigation.js') {
      return new Response(hostFixtureFile(new URL(`.${path}`, import.meta.url)), { headers: { 'content-type': 'text/javascript' } });
    }
    return new Response('<!doctype html><main id="root"></main>', { headers: { 'content-type': 'text/html' } });
  } });
  let browser;
  try {
    browser = await suiteBrowser();
    const page = await browser.newPage();
    await page.goto(server.url.href);
    await run(page);
  } finally { await browser?.close(); server.stop(true); }
}

for (const mode of ['without fragments', 'scroll events', 'fragment snapshots']) {
  test(`removed routes can be collected after ${mode}`, () => inBrowser(async page => {
    expect(await page.evaluate(async mode => {
      const fragment = mode === 'without fragments' ? null : await import('./fragment.js');
      const root = document.getElementById('root');
      history.replaceState({ exact: 1, id: 1, url: '/item#moves' }, '');
      window.removedRoutes = [];
      for (let i = 0; i < 40; i++) {
        const route = document.createElement('section'), port = document.createElement('div');
        route.setAttribute('navigationKey', '1');
        port.style.cssText = 'height:100px;overflow:auto';
        port.innerHTML = '<div style="height:20px">Row</div>'.repeat(200);
        route.append(port); root.append(route);
        if (mode === 'fragment snapshots') {
          history.replaceState({ exact: i + 2, id: 1, url: '/item#moves' }, '');
          fragment.arm(root, { top: 1, fragment: 'moves', pushed: true }, { personScrolls: 0 });
        } else {
          await new Promise(resolve => {
            port.addEventListener('scroll', () => resolve(), { once: true });
            port.scrollTop = 100;
          });
        }
        window.removedRoutes.push(new WeakRef(route));
        route.remove();
      }
      return window.removedRoutes.length;
    }, mode)).toBe(40);
    const cdp = await page.context().newCDPSession(page);
    for (let i = 0; i < 3; i++) await cdp.send('HeapProfiler.collectGarbage');
    expect(await page.evaluate(() => window.removedRoutes.filter(ref => ref.deref()).length)).toBe(0);
    if (mode !== 'without fragments') await page.evaluate(async () => {
      const fragment = await import('./fragment.js'), root = document.getElementById('root');
      root.innerHTML = '<section navigationKey="1"></section>';
      history.replaceState({ exact: 1, id: 1, url: '/item#moves' }, '');
      fragment.arm(root, { top: 1, fragment: 'moves', traversal: true }, { personScrolls: 0 });
      fragment.scroll(root, 1, 'moves', () => {});
    });
  }), 30_000);
}

test('an ordinary history push discards the scroll places of the forward entries it replaces', () => inBrowser(async page => {
  expect(await page.evaluate(async () => {
    const fragment = await import('./fragment.js'), { navigation } = await import('./navigation.js');
    const root = document.getElementById('root');
    root.innerHTML = '<div style="height:100px;overflow:auto"><section navigationKey="1" style="height:4000px"></section></div>';
    const port = root.firstElementChild;
    const apply = (url, top = 1) => navigation.apply({ top, url, removed: [] });
    navigation.connect(root, url => apply(url), () => {});
    const move = top => new Promise(resolve => {
      port.addEventListener('scroll', () => resolve(), { once: true });
      port.scrollTop = top;
    });
    apply('/item');
    apply('/item#old'); await move(100);
    apply('/item#later'); await move(200);
    await navigation.travel(null, -2);
    const back = history.state.exact;
    await move(300);
    apply('/other', 2);
    port.firstElementChild.setAttribute('navigationKey', '2');
    fragment.arm(root, { top: 2, fragment: '', traversal: true }, navigation);
    fragment.scroll(root, 2, '', () => {});
    return [back, history.state.exact, port.scrollTop];
  })).toEqual([0, 1, 300]);
}), 30_000);

test('traversal restores the latest scroll event on both axes while the route is retained', () => inBrowser(async page => {
  expect(await page.evaluate(async () => {
    const fragment = await import('./fragment.js'), root = document.getElementById('root');
    root.innerHTML = '<section navigationKey="1" style="height:100px;width:100px;overflow:auto"><div style="height:4000px;width:4000px"></div></section>';
    const port = root.firstElementChild;
    const move = (top, left) => new Promise(resolve => {
      port.addEventListener('scroll', () => resolve(), { once: true });
      port.scrollTo({ top, left, behavior: 'instant' });
    });
    history.replaceState({ exact: 1, id: 1, url: '/item#moves' }, '');
    await move(100, 50); await move(200, 80);
    history.replaceState({ exact: 2, id: 1, url: '/item#other' }, '');
    await move(500, 160);
    history.replaceState({ exact: 1, id: 1, url: '/item#moves' }, '');
    fragment.arm(root, { top: 1, fragment: 'moves', traversal: true }, { personScrolls: 0 });
    fragment.scroll(root, 1, 'moves', () => {});
    return [port.scrollTop, port.scrollLeft];
  })).toEqual([200, 80]);
}), 30_000);
