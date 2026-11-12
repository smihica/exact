// Observe's web service in two tabs of one Chromium profile, which share its localStorage
// queue: a tab adds its rows to the stored queue, and each row reaches the server once.
// `bun test ./modules/observe/tests/tabs.test.mjs`; it skips when the pinned Chromium is missing.
import { afterAll, expect } from 'bun:test';
import { readFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { join } from 'node:path';
import { suiteBrowser } from '../../../scripts/agent-launch.mjs';
import { browserMissing } from '../../../scripts/agent-playwright.mjs';
import { need } from '../../../scripts/coverage.mjs';

const service = readFileSync(join(import.meta.dir, '../web/service.js'), 'utf8');
const KEY = 'exact-observe:com.example.tabs';
const page = `<!doctype html><meta charset="utf-8"><script type="module">
  import { start } from './service.js';
  window.observe = start({ projectId: 'p', endpoint: location.origin + '/collect', 'app.CFBundleIdentifier': 'com.example.tabs' });
  window.event = name => observe.event({ kind: 'app.event', name, attributes: {}, wall: Date.now() });
  window.queued = () => JSON.parse(localStorage.getItem(${JSON.stringify(KEY)})).logs.map(r => r.name);
  window.ready = true;
</script>`;

const test = need('chrome', await browserMissing('chromium'));
let browser;
afterAll(() => browser?.close());

test('two tabs add to one stored queue and send each row once', async () => {
  // Each event name, as many times as a logs request carried it.
  const posted = [];
  const server = createServer((req, res) => {
    if (req.method === 'POST') {
      let body = '';
      req.on('data', c => { body += c; });
      req.on('end', () => {
        if (req.url.endsWith('/v1/logs')) {
          for (const r of JSON.parse(body).resourceLogs) for (const s of r.scopeLogs) for (const l of s.logRecords)
            posted.push(l.attributes.find(a => a.key === 'event.name').value.stringValue);
        }
        res.writeHead(200).end();
      });
      return;
    }
    if (req.url === '/service.js') { res.writeHead(200, { 'content-type': 'text/javascript' }).end(service); return; }
    res.writeHead(200, { 'content-type': 'text/html' }).end(page);
  });
  await new Promise(ok => server.listen(0, '127.0.0.1', ok));
  try {
    browser = await suiteBrowser('chromium');
    const context = await browser.newContext();
    // The batch timers never fire: each tab dispatches when the test calls `background()`.
    await context.clock.install();
    await context.clock.pauseAt(Date.now() + 1000);
    const open = async () => {
      const p = await context.newPage();
      await p.goto(`http://127.0.0.1:${server.address().port}/`);
      await p.waitForFunction(() => window.ready);
      return p;
    };
    const one = await open();
    await one.evaluate(() => event('first'));
    const two = await open();
    await two.evaluate(() => event('second'));
    // Wait until the first tab sees the second's row: localStorage reaches another renderer asynchronously.
    await one.waitForFunction(() => queued().includes('second'));
    await one.evaluate(() => event('third'));
    expect(await two.evaluate(() => queued())).toEqual(['first', 'second', 'third']);

    await Promise.all([one.evaluate(() => observe.background()), two.evaluate(() => observe.background())]);
    expect(posted.sort()).toEqual(['first', 'second', 'third']);
    for (const p of [one, two]) expect(await p.evaluate(() => queued())).toEqual([]);
  } finally {
    await browser?.close();
    server.close();
  }
}, 30_000);
