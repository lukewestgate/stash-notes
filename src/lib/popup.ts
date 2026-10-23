/** Close on a primary press on the backdrop, leaving interactions inside alone. */
export function dismissOnBackdrop(node: HTMLElement, onClose: () => void) {
  function pointerdown(event: PointerEvent) {
    if (event.button === 0 && event.target === node) {
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
