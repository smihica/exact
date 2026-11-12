export const selectedTexts = new Map();
// Text selections use source UTF-16 offsets; a Markdown <br> represents one newline.
export function textSource(e) {
  return e.nodeType === Node.TEXT_NODE ? e.data : e.nodeName === "BR" ? "\n" : [...e.childNodes].map(textSource).join("");
}
export function textPosition(e, offset) {
  const walker = document.createTreeWalker(e, NodeFilter.SHOW_TEXT | NodeFilter.SHOW_ELEMENT);
  for (let n = walker.nextNode(); n; n = walker.nextNode()) {
    if (n.nodeType === Node.TEXT_NODE) {
      if (offset <= n.length) {
        return [n, offset];
      }
      offset -= n.length;
    } else if (n.nodeName === "BR") {
      const i = [...n.parentNode.childNodes].indexOf(n);
      if (offset <= 1) {
        return [n.parentNode, i + offset];
      }
      offset--;
    }
  }
  return [e, e.childNodes.length];
}
function selectionRects(e, a, b) {
  if (a === b) {
    return [];
  }
  const range = document.createRange();
  range.setStart(...textPosition(e, a));
  range.setEnd(...textPosition(e, b));
  return [...range.getClientRects()].map((r) => [
    r.x,
    r.y,
    r.width,
    r.height,
    false,
    false
  ]);
}
function selectionChanged() {
  const s = getSelection(), r = s.rangeCount && !s.isCollapsed ? s.getRangeAt(0) : null;
  for (const [e, h] of selectedTexts) {
    if (!e.isConnected) {
      selectedTexts.delete(e);
      continue;
    }
    const at = (n, o) => {
      const p = document.createRange();
      p.selectNodeContents(e);
      const c = p.comparePoint(n, o);
      if (!c) {
        p.setEnd(n, o);
      }
      return c < 0 ? 0 : textSource(p.cloneContents()).length;
    };
    let a = 0, b = 0;
    if (r?.intersectsNode(e)) {
      a = at(r.startContainer, r.startOffset);
      b = at(r.endContainer, r.endOffset);
    }
    if (a === b) {
      a = b = 0;
    }
    if (h.a !== a || h.b !== b) {
      h.a = a;
      h.b = b;
      h.report(textSource(e).slice(a, b), a, b, selectionRects(e, a, b));
    }
  }
  if (!selectedTexts.size) {
    document.removeEventListener("selectionchange", selectionChanged);
  }
}
export function onSelection(e, report) {
  const held = {
    report,
    a: 0,
    b: 0
  };
  selectedTexts.set(e, held);
  document.addEventListener("selectionchange", selectionChanged);
  return () => {
    if (selectedTexts.get(e) === held) {
      selectedTexts.delete(e);
    }
    if (!selectedTexts.size) {
      document.removeEventListener("selectionchange", selectionChanged);
    }
  };
}
