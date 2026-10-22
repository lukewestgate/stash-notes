// Toast messages: short-lived notices in the corner. Errors and warnings stay
// until dismissed; the rest go away on their own, but not while the pointer
// is over them.

export type ToastKind = "info" | "done" | "warn" | "error";

export interface ToastAction {
  label: string;
  run: () => void;
}

export interface Toast {
  id: number;
  text: string;
  kind: ToastKind;
  action?: ToastAction;
}

interface Timer {
  handle?: ReturnType<typeof setTimeout>;
  left: number;
  started: number;
}

class Toaster {
  list = $state<Toast[]>([]);
  private nextId = 0;
  private timers = new Map<number, Timer>();
  private paused = false;

  show(text: string, kind: ToastKind = "info", action?: ToastAction, ms?: number): number {
    const id = ++this.nextId;
    this.list = [...this.list, { id, text, kind, action }];
    const sticky = kind === "error" || kind === "warn";
    if (!sticky) {
      this.timers.set(id, { left: ms ?? (action ? 5000 : 3500), started: 0 });
      if (!this.paused) this.start(id);
    }
    // Keep the corner tidy: at most four at a time.
    while (this.list.length > 4) this.dismiss(this.list[0].id);
    return id;
  }

  dismiss(id: number) {
    const t = this.timers.get(id);
    if (t?.handle) clearTimeout(t.handle);
    this.timers.delete(id);
    this.list = this.list.filter((x) => x.id !== id);
  }

  pause() {
    this.paused = true;
    for (const t of this.timers.values()) {
      if (t.handle) clearTimeout(t.handle);
      t.handle = undefined;
      t.left = Math.max(800, t.left - (Date.now() - t.started));
    }
  }

  resume() {
    this.paused = false;
    for (const id of this.timers.keys()) this.start(id);
  }

  private start(id: number) {
    const t = this.timers.get(id);
    if (!t) return;
    t.started = Date.now();
    t.handle = setTimeout(() => this.dismiss(id), t.left);
  }
}

export const toaster = new Toaster();
