import { root, contextAnchors, contextTransforms, views } from "../glue.js";
import { liveById } from "../navigation.js";
export function contextPanel(preview) {
  for (let parent = preview.parentElement; parent && parent !== root; parent = parent.parentElement) {
    if (getComputedStyle(parent).position === "absolute") {
      return parent;
    }
  }
  return null;
}
function contextAnchor(target, port = root.getBoundingClientRect()) {
  const box = target.getBoundingClientRect();
  const scroll = contextContent(target)?.parentElement;
  return {
    target,
    left: box.left - port.left,
    top: box.top - port.top,
    width: box.width,
    height: box.height,
    viewportWidth: port.width,
    scroll,
    scrollTop: scroll ? scroll.getBoundingClientRect().top - port.top : null
  };
}
export function prepareContexts(batch) {
  for (const op of batch.ops ?? []) {
    const props = op.op === "create" ? op.props : op.op === "props" ? op.set : null;
    if (!props?.contextTarget) {
      continue;
    }
    const target = liveById(props.contextTarget);
    if (target && contextAnchors.get(op.id)?.target !== target) {
      contextAnchors.set(op.id, contextAnchor(target));
    }
  }
}
function contextContent(source) {
  for (let child = source; child?.parentElement && child.parentElement !== root; child = child.parentElement) {
    const parent = child.parentElement;
    if (parent.dataset.scroll === "true" && /^(auto|scroll)$/.test(getComputedStyle(parent).overflowY)) {
      return child;
    }
  }
  return null;
}
export function positionContexts() {
  for (const node of contextTransforms) {
    node.style.transform = "";
  }
  contextTransforms.clear();
  for (const [id, anchor] of contextAnchors) {
    const preview = views.get(id);
    if (!preview?.isConnected || !anchor.target.isConnected || liveById(preview.getAttribute("contextTarget")) !== anchor.target) {
      contextAnchors.delete(id);
    }
  }
  const project = (node, transform) => {
    node.style.transform = transform;
    contextTransforms.add(node);
  };
  for (const preview of root.querySelectorAll("[contextTarget]")) {
    const target = liveById(preview.getAttribute("contextTarget"));
    const panel = contextPanel(preview);
    if (!target || !panel) {
      continue;
    }
    const box = panel.getBoundingClientRect(), content = preview.getBoundingClientRect();
    if (content.width <= 0 || content.height <= 0) {
      continue;
    }
    const liveSource = target.getBoundingClientRect(), port = root.getBoundingClientRect();
    const id = Number(preview.dataset.view);
    if (!contextAnchors.has(id) || contextAnchors.get(id).viewportWidth !== port.width) {
      contextAnchors.set(id, contextAnchor(target, port));
    }
    const anchor = contextAnchors.get(id);
    const source = {
      left: port.left + anchor.left,
      top: port.top + anchor.top,
      right: port.left + anchor.left + anchor.width,
      width: anchor.width,
      height: anchor.height
    };
    const scale = preview.getAttribute("contextMagnify") === "false" ? 1 : Math.min(1.15, 1 + 26 / Math.max(content.width, content.height));
    const extra = content.height * (scale - 1);
    const trailing = source.left + source.width / 2 > port.left + port.width / 2;
    const dx = trailing ? source.right - content.right - content.width * (scale - 1) / 2 : source.left - content.left + content.width * (scale - 1) / 2;
    for (const sibling of preview.parentElement.children) {
      if (sibling === preview) {
        continue;
      }
      const side = sibling.getBoundingClientRect();
      if (Math.abs(side.top - content.top) >= .01) {
        continue;
      }
      if (side.right <= content.left + .01) {
        project(sibling, `translateX(${dx - content.width * (scale - 1) / 2}px)`);
      } else if (side.left >= content.right - .01) {
        project(sibling, `translateX(${dx + content.width * (scale - 1) / 2}px)`);
      }
    }
    for (let child = preview; child && child !== panel; child = child.parentElement) {
      const bottom = child.getBoundingClientRect().bottom;
      for (const sibling of child.parentElement.children) {
        if (sibling !== child && sibling.getBoundingClientRect().top >= bottom - .01) {
          project(sibling, `translateY(${extra / 2}px)`);
        }
      }
    }
    project(preview, `translate(${dx}px, ${extra / 2}px) scale(${scale})`);
    const wanted = source.top + source.height / 2 - (content.top - box.top) - content.height * scale / 2;
    const overflow = Math.max(extra / 2, content.bottom + extra - box.bottom);
    const region = panel.offsetParent?.getBoundingClientRect() || port;
    const minimum = Math.max(region.top, port.top + 8);
    const maximum = Math.min(region.bottom, port.bottom - 8) - box.height - overflow;
    const top = Math.max(minimum, Math.min(wanted, maximum));
    panel.style.top = `${parseFloat(getComputedStyle(panel).top) + top - box.top}px`;
    let branch = preview;
    while (branch.parentElement && branch.parentElement !== panel) {
      branch = branch.parentElement;
    }
    const branchBottom = branch.getBoundingClientRect().bottom;
    const trailingControls = [...panel.children].filter((node) => node !== branch && node.getBoundingClientRect().top >= branchBottom - .01);
    if (trailingControls.length) {
      const first = Math.min(...trailingControls.map((node) => node.getBoundingClientRect().top));
      const last = Math.max(...trailingControls.map((node) => node.getBoundingClientRect().bottom));
      const overflow = Math.min(Math.max(0, last - Math.min(region.bottom, port.bottom - 8)), Math.max(0, first - minimum));
      if (overflow) {
        for (const node of trailingControls) {
          project(node, `translateY(${-overflow}px) ${node.style.transform}`);
        }
      }
    }
    const contentRoot = contextContent(target);
    if (contentRoot && !contentRoot.contains(panel)) {
      const scroll = contentRoot.parentElement;
      const scrollDelta = scroll === anchor.scroll && !scroll.contains(panel) ? port.top + anchor.scrollTop - scroll.getBoundingClientRect().top : 0;
      if (scrollDelta) {
        project(scroll, `translateY(${scrollDelta}px)`);
      }
      project(contentRoot, `translateY(${source.top + top - wanted - liveSource.top - scrollDelta}px)`);
    }
  }
}
