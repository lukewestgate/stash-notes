// Keeps an editor in sync with the note's buffer in Rust (see buffers.rs),
// using CodeMirror's collab protocol. Local edits are pushed as change sets;
// edits from other windows (or from Rust: status changes, quick capture,
// reloads from disk) arrive as `buffer-updates` events.

import { collab, getSyncedVersion, receiveUpdates, sendableUpdates, type Update } from "@codemirror/collab";
import { ChangeSet, type Extension } from "@codemirror/state";
import type { EditorView } from "@codemirror/view";
import { api, on, type BufferUpdate, type Opened } from "./api";

interface UpdatesEvent {
  path: string;
  version: number;
  updates: BufferUpdate[];
}

const live = new Set<NoteSync>();
let listening = false;

function listen() {
  if (listening) return;
  listening = true;
  on<UpdatesEvent>("buffer-updates", (ev) => {
    for (const s of live) if (s.path === ev.path) s.receive(ev.version, ev.updates);
  });
}

function toUpdates(list: BufferUpdate[]): Update[] {
  return list.map((u) => ({ changes: ChangeSet.fromJSON(u.changes), clientID: u.clientID }));
}

export class NoteSync {
  private pushing = false;
  private closed = false;

  constructor(
    public path: string,
    private view: EditorView,
    /** Called when this view fell too far behind and must reopen the note. */
    private onReset: () => void,
  ) {
    listen();
    live.add(this);
  }

  /** The collab extension for a freshly opened note. */
  static extension(opened: Opened): Extension {
    return collab({ startVersion: opened.version, clientID: opened.clientID });
  }

  /** Local edits not yet accepted by the buffer. */
  get pending(): boolean {
    return sendableUpdates(this.view.state).length > 0;
  }

  close() {
    this.closed = true;
    live.delete(this);
  }

  /** Send local edits; call after every local change. */
  async push(): Promise<void> {
    if (this.pushing || this.closed) return;
    const updates = sendableUpdates(this.view.state);
    if (!updates.length) return;
    this.pushing = true;
    const version = getSyncedVersion(this.view.state);
    try {
      const ok = await api.bufferPush(
        this.path,
        version,
        updates.map((u) => ({ changes: u.changes.toJSON(), clientID: u.clientID })),
      );
      if (this.closed) return;
      if (ok) {
        // Confirm right away unless the broadcast already did.
        if (getSyncedVersion(this.view.state) === version) {
          this.view.dispatch(receiveUpdates(this.view.state, updates as readonly Update[]));
        }
      } else {
        await this.pull();
      }
    } finally {
      this.pushing = false;
    }
    if (this.pending) queueMicrotask(() => this.push());
  }

  /** Wait until every local edit has been accepted. */
  async drain(): Promise<void> {
    for (let i = 0; i < 50 && this.pending && !this.closed; i++) {
      await this.push();
      if (this.pending) await new Promise((r) => setTimeout(r, 10));
    }
  }

  receive(version: number, updates: BufferUpdate[]) {
    if (this.closed) return;
    const synced = getSyncedVersion(this.view.state);
    if (version > synced) {
      this.pull();
      return;
    }
    const skip = synced - version;
    if (skip >= updates.length) return;
    this.view.dispatch(receiveUpdates(this.view.state, toUpdates(updates.slice(skip))));
  }

  async pull() {
    if (this.closed) return;
    const version = getSyncedVersion(this.view.state);
    const updates = await api.bufferPull(this.path, version);
    if (this.closed) return;
    if (updates === null) this.onReset();
    else this.receive(version, updates);
  }
}
