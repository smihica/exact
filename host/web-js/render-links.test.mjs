import { expect, test } from 'bun:test';
import { createDocument } from './dom.js';

// A page rendered ahead of time has a placeholder base (`render.invalid`, render.mjs); a Markdown link to an app path
// was written resolved against it, and a click on the page as served went there. It is written as authored.
test('a Markdown link to an app path is written as authored on a rendered page', async () => {
  const document = createDocument();
  Object.assign(document, { baseURI: 'http://render.invalid/' });
  const saved = globalThis.document;
  globalThis.document = document;
  try {
    const { renderMarkup } = await import('../web/navigation/markup.js');
    const el = document.createElement('div');
    renderMarkup(el, JSON.stringify([['the other page', 1, 0, 8, '/other'], [' and ', 1, 0, 0, ''], ['out', 1, 0, 8, 'https://example.com/a'], ['bad', 1, 0, 8, 'javascript:alert(1)']]));
    const links = el.childNodes.filter((n) => n.localName === 'a').map((a) => [a.getAttribute('href'), a.getAttribute('target')]);
    expect(links).toEqual([['/other', null], ['https://example.com/a', '_blank']]);
  } finally { globalThis.document = saved; }
});
