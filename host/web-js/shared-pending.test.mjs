import { expect, mock, test } from 'bun:test';

let mockShownStacks = () => new Map();
let mockHoldRoutes = () => () => {};
mock.module('./navigation.js', () => ({
  shownStacks: root => mockShownStacks(root),
  holdRoutes: (...args) => mockHoldRoutes(...args),
  inertAncestor: () => false,
}));

let sharedAfterCommit;

// A ViewTransition whose update callback never runs (e.g. compositor/GPU loss skipping the
// transition) must still drain pending commit tails so later UI commits never wedge.
test('a rejected ViewTransition drains pending tails and lets later commits run', async () => {
  const prev = {
    document: globalThis.document,
    matchMedia: globalThis.matchMedia,
    exact: globalThis.exact,
    CSS: globalThis.CSS,
  };
  try {
    globalThis.CSS = { escape: s => s };
    const style = { textContent: '' };
    let rejectUpdate, rejectFinished, rejectReady;
    globalThis.document = {
      head: { appendChild: el => el },
      createElement: () => style,
      getElementById: () => null,
      querySelectorAll: () => [],
      startViewTransition() {
        const updateCallbackDone = new Promise((_, rej) => { rejectUpdate = rej; });
        const ready = new Promise((_, rej) => { rejectReady = rej; });
        const finished = new Promise((_, rej) => { rejectFinished = rej; });
        return { updateCallbackDone, ready, finished, skipTransition() {} };
      },
    };
    globalThis.matchMedia = () => ({ matches: false });
    globalThis.exact = sharedAfterCommit ? { afterCommit: sharedAfterCommit } : {};

    const { rt } = await import('./rt-link.js');
    rt.Invocation ??= { id: 0, pending: new Set() };
    const { commit } = await import('./shared.js');
    sharedAfterCommit ??= globalThis.exact.afterCommit;

    const order = [];
    const inflight = { n: 0 };
    // Start a crossFade transition whose callback will never be invoked.
    expect(commit(() => { order.push('first'); return true; }, [], inflight, [], true)).toBe(true);
    // Queue a second commit and an afterCommit hook behind the stalled transition.
    expect(commit(() => { order.push('second'); return true; }, [], inflight, [], false)).toBe(true);
    globalThis.exact.afterCommit(() => order.push('after'));
    expect(order).toEqual([]);
    expect(inflight.n).toBe(1);

    // Reject the ViewTransition without running its update callback.
    rejectUpdate(new Error('skipped'));
    rejectReady(new Error('skipped'));
    rejectFinished(new Error('skipped'));
    await new Promise(r => setTimeout(r, 0));

    expect(order).toEqual(['first', 'second', 'after']);
    expect(inflight.n).toBe(0);

    // A subsequent commit runs immediately, proving `pending` is cleared.
    expect(commit(() => { order.push('third'); return true; }, [], inflight, [], false)).toBe(true);
    expect(order).toEqual(['first', 'second', 'after', 'third']);

    // A synchronous throw from startViewTransition must also restore inflight and drain pending tails
    // on both the crossFade path and the paired/routed transition path.
    globalThis.document.startViewTransition = () => { throw new Error('sync skip'); };
    expect(commit(() => { order.push('fourth'); return true; }, [], inflight, [], true)).toBe(true);
    expect(inflight.n).toBe(0);
    expect(order).toEqual(['first', 'second', 'after', 'third', 'fourth']);

    const sharedEl = {
      isConnected: true,
      style: { viewTransitionName: '' },
      getAttribute: name => (name === 'data-shared-element' ? 'card' : null),
      closest: () => null,
    };
    const root = { querySelectorAll: sel => (sel === '[data-shared-element]' ? [sharedEl] : []) };
    globalThis.document.getElementById = id => (id === 'exact-root' ? root : null);
    expect(commit(() => { order.push('fifth'); return true; }, [], inflight, [], false)).toBe(true);
    expect(inflight.n).toBe(0);
    expect(sharedEl.style.viewTransitionName).toBe('');
    expect(commit(() => { order.push('sixth'); return true; }, [], inflight, [], false)).toBe(true);
    expect(order).toEqual(['first', 'second', 'after', 'third', 'fourth', 'fifth', 'sixth']);
  } finally {
    if (prev.CSS === undefined) delete globalThis.CSS; else globalThis.CSS = prev.CSS;
    if (prev.document === undefined) delete globalThis.document; else globalThis.document = prev.document;
    if (prev.matchMedia === undefined) delete globalThis.matchMedia; else globalThis.matchMedia = prev.matchMedia;
    if (prev.exact === undefined) delete globalThis.exact; else globalThis.exact = prev.exact;
  }
});

test('a ViewTransition whose ready rejects before update still holds inflight and viewTransition until drain', async () => {
  const prev = {
    document: globalThis.document,
    matchMedia: globalThis.matchMedia,
    exact: globalThis.exact,
    CSS: globalThis.CSS,
  };
  try {
    globalThis.CSS = { escape: s => s };
    const style = { textContent: '' };
    let runUpdate, resolveUpdate, resolveFinished, rejectReady;
    globalThis.document = {
      head: { appendChild: el => el },
      createElement: () => style,
      getElementById: () => null,
      querySelectorAll: () => [],
      startViewTransition(cb) {
        runUpdate = cb;
        const updateCallbackDone = new Promise(res => { resolveUpdate = res; });
        const ready = new Promise((_, rej) => { rejectReady = rej; });
        const finished = new Promise(res => { resolveFinished = res; });
        return { updateCallbackDone, ready, finished, skipTransition() {} };
      },
    };
    globalThis.matchMedia = () => ({ matches: false });
    globalThis.exact = sharedAfterCommit ? { afterCommit: sharedAfterCommit } : {};

    const { rt } = await import('./rt-link.js');
    rt.Invocation ??= { id: 0, pending: new Set() };
    const { commit } = await import('./shared.js');
    sharedAfterCommit ??= globalThis.exact.afterCommit;

    const order = [];
    const inflight = { n: 0 };
    expect(commit(() => { order.push('mounted'); return true; }, [], inflight, [], true)).toBe(true);
    expect(inflight.n).toBe(1);

    let vtDone = false;
    const vt = globalThis.exact.viewTransition().then(() => { vtDone = true; });

    // Reject `ready` before the update callback runs: inflight and viewTransition must stay held.
    rejectReady(new Error('skipped early'));
    await new Promise(r => setTimeout(r, 0));
    expect(order).toEqual([]);
    expect(inflight.n).toBe(1);
    expect(vtDone).toBe(false);

    // Once the update callback runs and drains the tail, both settle cleanly.
    runUpdate();
    resolveUpdate();
    resolveFinished();
    await vt;
    expect(order).toEqual(['mounted']);
    expect(inflight.n).toBe(0);
    expect(vtDone).toBe(true);

    // A throwing afterCommit callback must still run sibling afterCommit callbacks,
    // release inflight, and resolve viewTransition even when updateCallbackDone rejects.
    const origConsoleError = console.error;
    const logged = [];
    console.error = (...a) => logged.push(a);
    try {
      let rejectUpdate;
      globalThis.document.startViewTransition = cb => {
        runUpdate = cb;
        const updateCallbackDone = new Promise((_, rej) => { rejectUpdate = rej; });
        const ready = new Promise((_, rej) => { rejectReady = rej; });
        const finished = new Promise(res => { resolveFinished = res; });
        return { updateCallbackDone, ready, finished, skipTransition() {} };
      };
      expect(commit(() => { order.push('after-throw'); return true; }, [], inflight, [], true)).toBe(true);
      globalThis.exact.afterCommit(() => { throw new Error('after hook failed'); });
      globalThis.exact.afterCommit(() => order.push('after-sibling'));
      const vtAfterThrow = globalThis.exact.viewTransition();
      let caught = null;
      try { runUpdate(); } catch (err) { caught = err; }
      expect(caught?.message).toBe('after hook failed');
      rejectUpdate(caught);
      rejectReady(caught);
      resolveFinished();
      await vtAfterThrow;
      expect(order).toContain('after-sibling');
      expect(inflight.n).toBe(0);
    } finally {
      console.error = origConsoleError;
    }
  } finally {
    if (prev.CSS === undefined) delete globalThis.CSS; else globalThis.CSS = prev.CSS;
    if (prev.document === undefined) delete globalThis.document; else globalThis.document = prev.document;
    if (prev.matchMedia === undefined) delete globalThis.matchMedia; else globalThis.matchMedia = prev.matchMedia;
    if (prev.exact === undefined) delete globalThis.exact; else globalThis.exact = prev.exact;
  }
});

test('a route entrance does not overwrite a pending cross-fade started synchronously inside tail', async () => {
  const prev = {
    document: globalThis.document,
    matchMedia: globalThis.matchMedia,
    exact: globalThis.exact,
    CSS: globalThis.CSS,
  };
  try {
    globalThis.CSS = { escape: s => s };
    const style = { textContent: '' };
    const transitions = [];
    const routeA = {
      getAttribute: () => null,
      style: { viewTransitionName: '', getPropertyValue: () => '' },
    };
    const routeB = {
      getAttribute: () => null,
      style: { viewTransitionName: '', getPropertyValue: p => p === '--exact-view-transition-new' ? 'enter 80ms linear both' : '' },
    };
    const nav = {};
    let stack = [routeA];
    const rootEl = {
      querySelectorAll: () => [],
    };
    globalThis.document = {
      head: { appendChild: el => el },
      createElement: () => style,
      getElementById: id => id === 'exact-root' ? rootEl : null,
      querySelectorAll: () => [],
      startViewTransition(cb) {
        let resolveUpdate, resolveReady, resolveFinished;
        const entry = {
          cb,
          skipped: false,
          finish() {
            cb();
            resolveUpdate();
            resolveReady();
            resolveFinished();
          },
        };
        const t = {
          updateCallbackDone: new Promise(r => { resolveUpdate = r; }),
          ready: new Promise(r => { resolveReady = r; }),
          finished: new Promise(r => { resolveFinished = r; }),
          skipTransition() { entry.skipped = true; },
        };
        transitions.push(entry);
        return t;
      },
    };
    globalThis.matchMedia = () => ({ matches: false });
    globalThis.exact = sharedAfterCommit ? { afterCommit: sharedAfterCommit } : {};

    mockShownStacks = () => new Map([[nav, stack]]);
    const { rt } = await import('./rt-link.js');
    rt.Invocation ??= { id: 0, pending: new Set() };
    const { commit } = await import('./shared.js');
    sharedAfterCommit ??= globalThis.exact.afterCommit;

    const inflight = { n: 0 };
    const order = [];
    const startPending = rt.Invocation.pending.size;
    commit(() => {
      stack = [routeA, routeB];
      order.push('outer');
      // Synchronous autofocus-style commit inside tail starts a cross-fade.
      commit(() => { order.push('inner-fade'); return true; }, [], inflight, [], true);
      return true;
    }, [], inflight, [], false);

    expect(transitions.length).toBe(1);
    expect(inflight.n).toBe(1);
    const vt = globalThis.exact.viewTransition();
    transitions[0].finish();
    await vt;
    expect(order).toEqual(['outer', 'inner-fade']);
    expect(inflight.n).toBe(0);
    expect(rt.Invocation.pending.size).toBe(startPending);
  } finally {
    mockShownStacks = () => new Map();
    if (prev.CSS === undefined) delete globalThis.CSS; else globalThis.CSS = prev.CSS;
    if (prev.document === undefined) delete globalThis.document; else globalThis.document = prev.document;
    if (prev.matchMedia === undefined) delete globalThis.matchMedia; else globalThis.matchMedia = prev.matchMedia;
    if (prev.exact === undefined) delete globalThis.exact; else globalThis.exact = prev.exact;
  }
});

test('a reentrant cross-fade inside a shared-element tail does not lose commits and viewTransition waits for it', async () => {
  const prev = {
    document: globalThis.document,
    matchMedia: globalThis.matchMedia,
    exact: globalThis.exact,
    CSS: globalThis.CSS,
  };
  try {
    globalThis.CSS = { escape: s => s };
    const style = { textContent: '' };
    const transitions = [];
    const leaver = {
      nodeType: 1,
      isConnected: false,
      style: { viewTransitionName: '', getPropertyValue: () => '' },
      matches: () => true,
      getAttribute: () => 'pill',
      querySelectorAll: () => [],
    };
    const startAnchor = { parentNode: null, nextSibling: leaver };
    const endAnchor = { parentNode: null, nextSibling: null };
    const parent = {};
    startAnchor.parentNode = parent;
    endAnchor.parentNode = parent;
    leaver.nextSibling = endAnchor;
    const region = { gone: false, $r: () => [startAnchor, endAnchor] };
    globalThis.document = {
      head: { appendChild: el => el },
      createElement: () => style,
      getElementById: () => null,
      querySelectorAll: () => [],
      startViewTransition(cb) {
        let resolveUpdate, resolveReady, resolveFinished;
        const entry = {
          cb,
          skipped: false,
          finish() {
            cb();
            resolveUpdate();
            resolveReady();
            resolveFinished();
          },
        };
        const t = {
          updateCallbackDone: new Promise(r => { resolveUpdate = r; }),
          ready: new Promise(r => { resolveReady = r; }),
          finished: new Promise(r => { resolveFinished = r; }),
          skipTransition() { entry.skipped = true; },
        };
        transitions.push(entry);
        return t;
      },
    };
    globalThis.matchMedia = () => ({ matches: false });
    globalThis.exact = sharedAfterCommit ? { afterCommit: sharedAfterCommit } : {};

    const { rt } = await import('./rt-link.js');
    rt.Invocation ??= { id: 0, pending: new Set() };
    const { commit } = await import('./shared.js');
    sharedAfterCommit ??= globalThis.exact.afterCommit;

    const inflight = { n: 0 };
    const order = [];
    const startPending = rt.Invocation.pending.size;
    // Start a shared-element transition, queue a non-fade commit behind it, and have the
    // first tail synchronously trigger a reentrant cross-fade (e.g. from autofocus).
    expect(commit(() => {
      order.push('shared-tail');
      commit(() => { order.push('reentrant-fade'); return true; }, [], inflight, [], true);
      return true;
    }, [region], inflight, [], false)).toBe(true);
    expect(commit(() => { order.push('queued-commit'); return true; }, [], inflight, [], false)).toBe(true);

    expect(transitions.length).toBe(1);
    let vtResolved = false;
    const vt = globalThis.exact.viewTransition().then(() => { vtResolved = true; });
    transitions[0].finish();
    // Reentrant cross-fade started inside tail: remaining queued commits are handed off to
    // the new cross-fade rather than flushing its DOM writes during the old callback, and
    // the existing viewTransition() promise still waits for the reentrant transition.
    expect(transitions.length).toBe(2);
    expect(style.textContent).toBe('');
    expect(order).toEqual(['shared-tail']);
    await new Promise(r => setTimeout(r, 0));
    expect(vtResolved).toBe(false);

    transitions[1].finish();
    await vt;
    expect(vtResolved).toBe(true);
    expect(order).toEqual(['shared-tail', 'queued-commit', 'reentrant-fade']);
    expect(inflight.n).toBe(0);
    expect(rt.Invocation.pending.size).toBe(startPending);

    // A fade submitted while a non-fade transition is still pending promotes it to a
    // whole-page cross-fade carrying every waiting tail without losing the fade flag.
    order.length = 0;
    transitions.length = 0;
    expect(commit(() => { order.push('shared-before-promote'); return true; }, [region], inflight, [], false)).toBe(true);
    expect(transitions.length).toBe(1);
    expect(style.textContent).toBe(':root{view-transition-name:none}');
    const vtPromoted = globalThis.exact.viewTransition();
    expect(commit(() => { order.push('promoted-fade'); return true; }, [], inflight, [], true)).toBe(true);
    expect(transitions.length).toBe(2);
    expect(transitions[0].skipped).toBe(true);
    expect(style.textContent).toBe('');
    expect(inflight.n).toBe(1);
    transitions[1].finish();
    await vtPromoted;
    expect(order).toEqual(['shared-before-promote', 'promoted-fade']);
    expect(inflight.n).toBe(0);
    expect(rt.Invocation.pending.size).toBe(startPending);
  } finally {
    if (prev.CSS === undefined) delete globalThis.CSS; else globalThis.CSS = prev.CSS;
    if (prev.document === undefined) delete globalThis.document; else globalThis.document = prev.document;
    if (prev.matchMedia === undefined) delete globalThis.matchMedia; else globalThis.matchMedia = prev.matchMedia;
    if (prev.exact === undefined) delete globalThis.exact; else globalThis.exact = prev.exact;
  }
});

test('reentrant cross-fade during holdRoutes in entrance and shared transitions does not orphan pending or clobber styles', async () => {
  const prev = {
    document: globalThis.document,
    matchMedia: globalThis.matchMedia,
    exact: globalThis.exact,
    CSS: globalThis.CSS,
  };
  try {
    globalThis.CSS = { escape: s => s };
    const style = { textContent: '' };
    const transitions = [];
    const nav = {};
    const routeA = { isConnected: true, style: { viewTransitionName: '', getPropertyValue: () => '' }, getAttribute: () => null };
    const routeB = { isConnected: true, style: { viewTransitionName: '', getPropertyValue: k => (k === '--exact-view-transition-new' ? 'slide 200ms' : '') }, getAttribute: () => null };
    let stack = [routeA];
    const leaver = {
      nodeType: 1,
      isConnected: false,
      style: { viewTransitionName: '', getPropertyValue: () => '' },
      matches: () => true,
      getAttribute: () => 'card',
      querySelectorAll: () => [],
    };
    const startAnchor = { parentNode: null, nextSibling: leaver };
    const endAnchor = { parentNode: null, nextSibling: null };
    const parent = {};
    startAnchor.parentNode = parent;
    endAnchor.parentNode = parent;
    leaver.nextSibling = endAnchor;
    const region = { gone: false, $r: () => [startAnchor, endAnchor] };
    globalThis.document = {
      head: { appendChild: el => el },
      createElement: () => style,
      getElementById: id => (id === 'exact-root' ? {} : null),
      querySelectorAll: () => [],
      startViewTransition(cb) {
        let resolveUpdate, resolveReady, resolveFinished;
        const entry = {
          cb,
          skipped: false,
          finish() {
            cb();
            resolveUpdate();
            resolveReady();
            resolveFinished();
          },
        };
        const t = {
          updateCallbackDone: new Promise(r => { resolveUpdate = r; }),
          ready: new Promise(r => { resolveReady = r; }),
          finished: new Promise(r => { resolveFinished = r; }),
          skipTransition() { entry.skipped = true; },
        };
        transitions.push(entry);
        return t;
      },
    };
    globalThis.matchMedia = () => ({ matches: false });
    globalThis.exact = sharedAfterCommit ? { afterCommit: sharedAfterCommit } : {};

    mockShownStacks = () => new Map([[nav, stack]]);
    const { rt } = await import('./rt-link.js');
    rt.Invocation ??= { id: 0, pending: new Set() };
    const { commit } = await import('./shared.js');
    sharedAfterCommit ??= globalThis.exact.afterCommit;

    const inflight = { n: 0 };
    const order = [];
    const startPending = rt.Invocation.pending.size;
    let released = 0;
    mockHoldRoutes = () => {
      commit(() => { order.push('blur-fade'); return true; }, [], inflight, [], true);
      return () => { released++; };
    };
    // Entrance path: blur inside holdRoutes starts a cross-fade -> outer entrance must bail out cleanly.
    commit(() => { stack = [routeA, routeB]; order.push('push'); return true; }, [], inflight, [], false);
    expect(transitions.length).toBe(1);
    expect(released).toBe(1);
    expect(inflight.n).toBe(1);
    expect(style.textContent).toBe('');
    const vt1 = globalThis.exact.viewTransition();
    transitions[0].finish();
    await vt1;
    expect(order).toEqual(['push', 'blur-fade']);
    expect(inflight.n).toBe(0);
    expect(rt.Invocation.pending.size).toBe(startPending);

    // Shared-element path: blur inside holdRoutes during callback must not clobber the newer fade's stylesheet.
    order.length = 0;
    transitions.length = 0;
    released = 0;
    stack = [routeA];
    commit(() => { stack = [routeA, routeB]; order.push('shared-push'); return true; }, [region], inflight, [], false);
    expect(transitions.length).toBe(1);
    const vt2 = globalThis.exact.viewTransition();
    transitions[0].finish();
    expect(transitions.length).toBe(2);
    expect(released).toBe(1);
    expect(style.textContent).toBe('');
    transitions[1].finish();
    await vt2;
    expect(order).toEqual(['shared-push', 'blur-fade']);
    expect(inflight.n).toBe(0);
    expect(rt.Invocation.pending.size).toBe(startPending);

    // Promotion handoff -> route release synchronously starting a reentrant fade must join
    // that fade's queue rather than overwriting `pending` and stranding tails/inflight.
    order.length = 0;
    transitions.length = 0;
    released = 0;
    stack = [routeA];
    let reenterOnRelease = false;
    mockHoldRoutes = () => () => {
      released++;
      if (reenterOnRelease) {
        reenterOnRelease = false;
        commit(() => { order.push('release-fade'); return true; }, [], inflight, [], true);
      }
    };
    commit(() => { stack = [routeA, routeB]; order.push('entrance-push'); return true; }, [], inflight, [], false);
    expect(transitions.length).toBe(1);
    reenterOnRelease = true;
    const vtPromoReenter = globalThis.exact.viewTransition();
    commit(() => { order.push('promoting-fade'); return true; }, [], inflight, [], true);
    expect(transitions.length).toBe(2);
    expect(released).toBe(1);
    expect(inflight.n).toBe(1);
    transitions[1].finish();
    await vtPromoReenter;
    expect(order).toEqual(['entrance-push', 'promoting-fade', 'release-fade']);
    expect(inflight.n).toBe(0);
    expect(rt.Invocation.pending.size).toBe(startPending);

    // Promotion handoff -> route release committing an ordinary cut (fade=false) must NOT
    // flush DOM writes before the promoting cross-fade; it must queue behind the promoting fade.
    order.length = 0;
    transitions.length = 0;
    released = 0;
    stack = [routeA];
    let cutOnRelease = false;
    mockHoldRoutes = () => () => {
      released++;
      if (cutOnRelease) {
        cutOnRelease = false;
        commit(() => { order.push('release-cut'); return true; }, [], inflight, [], false);
      }
    };
    commit(() => { stack = [routeA, routeB]; order.push('entrance-before-cut'); return true; }, [], inflight, [], false);
    expect(transitions.length).toBe(1);
    cutOnRelease = true;
    const vtPromoCut = globalThis.exact.viewTransition();
    commit(() => { order.push('promoting-fade-cut'); return true; }, [], inflight, [], true);
    expect(transitions.length).toBe(2);
    expect(order).toEqual(['entrance-before-cut']);
    transitions[1].finish();
    await vtPromoCut;
    expect(order).toEqual(['entrance-before-cut', 'promoting-fade-cut', 'release-cut']);
    expect(inflight.n).toBe(0);
    expect(rt.Invocation.pending.size).toBe(startPending);

    // Reentrant tab cut inside holdRoutes (no transition started, old route stays connected
    // in another tab): entrance and shared transitions must revalidate `stillTop` and bail out,
    // including when an outer tab hides a nested navigation root (`hidden = true`).
    order.length = 0;
    transitions.length = 0;
    released = 0;
    const otherTabNav = {};
    const routeOtherTab = { isConnected: true, style: { viewTransitionName: '', getPropertyValue: () => '' }, getAttribute: () => null };
    let activeTab = 'a';
    mockShownStacks = () => (activeTab === 'a' ? new Map([[nav, stack]]) : new Map([[otherTabNav, [routeOtherTab]]]));
    mockHoldRoutes = () => {
      activeTab = 'b';
      return () => { released++; };
    };
    stack = [routeA];
    commit(() => { stack = [routeA, routeB]; order.push('cut-entrance'); return true; }, [], inflight, [], false);
    expect(transitions.length).toBe(0);
    expect(released).toBe(1);
    expect(inflight.n).toBe(0);

    // Entrance whose queued tail switches tabs during callback must not leave routeB named or held.
    order.length = 0;
    transitions.length = 0;
    released = 0;
    activeTab = 'a';
    stack = [routeA];
    mockHoldRoutes = () => () => { released++; };
    commit(() => { stack = [routeA, routeB]; order.push('entrance-tail-base'); return true; }, [], inflight, [], false);
    commit(() => { activeTab = 'b'; order.push('tail-tab-cut'); return true; }, [], inflight, [], false);
    const vtTailCut = globalThis.exact.viewTransition();
    transitions[0].finish();
    await vtTailCut;
    expect(released).toBe(1);
    expect(routeB.style.viewTransitionName).toBe('');
    expect(transitions[0].skipped).toBe(true);
    expect(inflight.n).toBe(0);

    // Shared-element tab cut during holdRoutes must not remove ghost or scroll arriver.
    activeTab = 'a';
    stack = [routeA];
    released = 0;
    let ghostRemoved = 0, arriverScrolled = 0;
    const ghostEl = {
      nodeType: 1, isConnected: true, style: { viewTransitionName: '', getPropertyValue: k => (k === '--exact-layout-transition' ? '100ms ease' : '') },
      matches: () => true, getAttribute: () => 'card', querySelectorAll: () => [],
      closest: sel => (sel === '[data-exiting]' ? ghostEl : null), remove() { ghostRemoved++; },
    };
    const arriverEl = {
      style: { viewTransitionName: '', getPropertyValue: k => (k === '--exact-layout-transition' ? '100ms ease' : '') },
      scrollIntoView() { arriverScrolled++; },
    };
    const prevQSA = globalThis.document.querySelectorAll;
    globalThis.document.querySelectorAll = sel => (sel.includes('card') ? [arriverEl] : []);
    startAnchor.nextSibling = ghostEl;
    ghostEl.nextSibling = endAnchor;
    mockHoldRoutes = () => { activeTab = 'b'; return () => { released++; }; };
    transitions.length = 0;
    commit(() => { stack = [routeA, routeB]; order.push('cut-shared'); return true; }, [region], inflight, [], false);
    expect(transitions.length).toBe(1);
    const vtCut = globalThis.exact.viewTransition();
    transitions[0].finish();
    await vtCut;
    globalThis.document.querySelectorAll = prevQSA;
    expect(released).toBe(1);
    expect(transitions[0].skipped).toBe(true);
    expect(routeB.style.viewTransitionName).toBe('');
    expect(ghostRemoved).toBe(0);
    expect(arriverScrolled).toBe(0);
    expect(inflight.n).toBe(0);
  } finally {
    mockShownStacks = () => new Map();
    mockHoldRoutes = () => () => {};
    if (prev.CSS === undefined) delete globalThis.CSS; else globalThis.CSS = prev.CSS;
    if (prev.document === undefined) delete globalThis.document; else globalThis.document = prev.document;
    if (prev.matchMedia === undefined) delete globalThis.matchMedia; else globalThis.matchMedia = prev.matchMedia;
    if (prev.exact === undefined) delete globalThis.exact; else globalThis.exact = prev.exact;
  }
});
