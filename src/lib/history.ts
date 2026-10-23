// Browser-style navigation history. Lives for the whole process (the window
// is never destroyed), so it survives hide/show and is cleared only on quit.

export interface Entry {
  path: string;
  anchor: number;
  scroll: number;
}

export const MAX_HISTORY = 100;

export class History {
  entries: Entry[] = [];
  index = -1;
  /** A note opened with Alt+Enter from the switcher: shown, but not recorded. */
  peek: Entry | null = null;

  /** The entry currently on screen. */
  get current(): Entry | null {
    return this.peek ?? this.entries[this.index] ?? null;
  }

  get canBack(): boolean {
    return this.peek !== null || this.index > 0;
  }

  get canForward(): boolean {
    return this.peek === null && this.index < this.entries.length - 1;
  }

  /** Record opening `path`, discarding forward entries. */
  push(path: string): Entry {
    this.peek = null;
    if (this.entries[this.index]?.path === path) return this.entries[this.index];
    this.entries.splice(this.index + 1);
    const entry = { path, anchor: -1, scroll: 0 };
    this.entries.push(entry);
    if (this.entries.length > MAX_HISTORY) this.entries.splice(0, this.entries.length - MAX_HISTORY);
    this.index = this.entries.length - 1;
    return entry;
  }

  /** Show `path` without touching the stack; Back returns to the current entry. */
  peekAt(path: string): Entry {
    this.peek = { path, anchor: -1, scroll: 0 };
    return this.peek;
  }

  back(): Entry | null {
    if (this.peek) {
      this.peek = null;
      return this.entries[this.index] ?? null;
    }
    if (this.index <= 0) return null;
    this.index--;
    return this.entries[this.index];
  }

  forward(): Entry | null {
    if (!this.canForward) return null;
    this.index++;
    return this.entries[this.index];
  }

  /** A note moved (archive, rename): follow it everywhere in history. */
  rename(from: string, to: string) {
    for (const e of this.entries) if (e.path === from) e.path = to;
    if (this.peek?.path === from) this.peek.path = to;
  }

  /** Drop every reference to a deleted file, preserving our position. */
  remove(path: string) {
    const before = this.entries.slice(0, this.index + 1).filter((e) => e.path !== path).length;
    this.entries = this.entries.filter((e) => e.path !== path);
    this.index = Math.min(this.entries.length - 1, before - 1);
    if (this.peek?.path === path) this.peek = null;
  }
}
