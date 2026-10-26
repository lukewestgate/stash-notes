/** Close on a press on the backdrop, leaving interactions inside alone. */
export function dismissOnBackdrop(node: HTMLElement, onClose: () => void) {
  function pointerdown(event: PointerEvent) {
    // Any button: a right-click elsewhere also closes an anchored menu.
    if (event.button <= 2 && event.target === node) {
      // Don't let the browser clear the focus the close handler restores.
      event.preventDefault();
      onClose();
    }
  }

  node.addEventListener("pointerdown", pointerdown);
  return {
    update(next: () => void) { onClose = next; },
    destroy() { node.removeEventListener("pointerdown", pointerdown); },
  };
}

/**
 * Where a popup opened from: the element (or pointer position) that opened it.
 * A popup with an anchor opens beside it; one without opens centered.
 */
export interface Anchor {
  x: number;
  y: number;
  w: number;
  h: number;
  /** `below` (default): under the anchor. `right`: beside it, for submenus. */
  side?: "below" | "right";
}

/** Anchor a popup to an element, e.g. the button that opened it. */
export function anchorOf(el: Element, side: Anchor["side"] = "below"): Anchor {
  const r = el.getBoundingClientRect();
  return { x: r.left, y: r.top, w: r.width, h: r.height, side };
}

/** Anchor a popup to the pointer, e.g. for a right-click menu. */
export function anchorAt(e: MouseEvent): Anchor {
  return { x: e.clientX, y: e.clientY, w: 0, h: 0 };
}

const GAP = 4;
const MARGIN = 8;

/** Where a `width` × `height` popup goes for `a`, kept inside the window. */
export function placement(a: Anchor, width: number, height: number, vw: number, vh: number): { left: number; top: number } {
  let left: number;
  let top: number;
  if (a.side === "right") {
    left = a.x + a.w + 2;
    if (left + width > vw - MARGIN) left = a.x - width - 2;
    top = a.y - 5;
  } else {
    left = a.x;
    top = a.y + a.h + GAP;
    if (top + height > vh - MARGIN && a.y - height - GAP >= MARGIN) top = a.y - height - GAP;
  }
  left = Math.max(MARGIN, Math.min(left, vw - MARGIN - width));
  top = Math.max(MARGIN, Math.min(top, vh - MARGIN - height));
  return { left, top };
}

/** Position a popup panel beside its anchor (no-op without one); follows size changes. */
export function placeAt(node: HTMLElement, anchor: Anchor | null | undefined) {
  let current = anchor;
  function apply() {
    if (!current) return;
    const { left, top } = placement(current, node.offsetWidth, node.offsetHeight, window.innerWidth, window.innerHeight);
    node.style.position = "fixed";
    node.style.margin = "0";
    node.style.left = `${left}px`;
    node.style.top = `${top}px`;
  }
  apply();
  const observer = typeof ResizeObserver === "function" ? new ResizeObserver(apply) : null;
  observer?.observe(node);
  window.addEventListener("resize", apply);
  return {
    update(next: Anchor | null | undefined) {
      current = next;
      apply();
    },
    destroy() {
      observer?.disconnect();
      window.removeEventListener("resize", apply);
    },
  };
}
