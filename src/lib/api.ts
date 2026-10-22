// Typed wrappers around the Rust commands. Outside Tauri (plain `npm run dev`
// in a browser) calls go to an in-memory mock so the UI can be worked on alone.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

export type NoteKind = "daily" | "ticket" | "note";

export interface NoteMeta {
  path: string;
  kind: NoteKind;
  archived: boolean;
  title: string;
  label: string | null;
  key: string | null;
  date: string | null;
  jira: string | null;
  gerrit: string[];
  aliases: string[];
  status: string | null;
}

/** A note opened in a window: its buffer's text and version (see buffers.rs). */
export interface Opened {
  text: string;
  version: number;
  meta: NoteMeta;
  dirty: boolean;
  deleted: boolean;
  clientID: string;
}

/** A CodeMirror change set (as JSON) and the view that made it. */
export interface BufferUpdate {
  changes: unknown;
  clientID: string;
}

export interface SearchHit {
  path: string;
  kind: NoteKind;
  title: string;
  label: string | null;
  key: string | null;
  status: string | null;
  archived: boolean;
  mtime: number;
  snippet: string | null;
  score: number;
}

export interface StatusDef {
  name: string;
  symbol: string;
  color: string;
  active: boolean;
}

export interface StatusResult {
  path: string;
  edit: TextEdit | null;
  meta: NoteMeta;
}

export interface SearchResults {
  hits: SearchHit[];
  exact: boolean;
}

export interface Keys {
  hide: string;
  quickSwitcher: string;
  dailyNote: string;
  followLink: string;
  back: string;
  forward: string;
  openJira: string;
  openGerrit: string;
  addGerrit: string;
  timestamp: string;
  backlinks: string;
  archive: string;
  help: string;
  quit: string;
  rename: string;
  openSettings: string;
  setStatus: string;
  notesView: string;
  toggleSidebar: string;
  focusSidebar: string;
  pinNote: string;
}

export type SortBy = "edited" | "opened" | "status" | "title";

export interface NotesViewSettings {
  showDaily: boolean;
  showClosed: boolean;
  sort: SortBy;
  groupByStatus: boolean;
}

export interface Settings {
  schemaVersion: number;
  vaultPath: string;
  hotkey: string;
  switcherHotkey: string;
  timestampFormat: string;
  onSummon: "last" | "daily";
  startHidden: boolean;
  autostart: boolean;
  showInTaskbar: boolean;
  alwaysOnTop: boolean;
  hideAfterExternalLink: boolean;
  ticketPattern: string;
  jira: { urlTemplate: string };
  gerrit: { urlTemplate: string };
  theme: "system" | "light" | "dark";
  fontSize: number;
  fontFamily: string;
  lineHeight: number;
  spellCheck: boolean;
  accent: string;
  liveFormatting: boolean;
  smartPaste: boolean;
  restorePinned: boolean;
  notesView: NotesViewSettings;
  statuses: StatusDef[];
  defaultStatus: string;
  keys: Keys;
}

export interface Startup {
  settings: Settings;
  vaultPath: string;
  settingsPath: string;
  warnings: string[];
  lastPath: string | null;
  lastAnchor: number | null;
  indexReady: boolean;
  ui: Record<string, unknown>;
}

export interface RenameResult {
  newPath: string;
  linksUpdated: number;
  filesChanged: number;
  undoId: number;
}

export interface TextEdit {
  from: number;
  to: number;
  insert: string;
}

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

type Args = Record<string, unknown>;
let mock: Promise<typeof import("./mock")> | null = null;

async function call<T>(cmd: string, args: Args = {}): Promise<T> {
  if (inTauri) return invoke<T>(cmd, args);
  mock ??= import("./mock");
  return (await mock).handle(cmd, args) as T;
}

export const api = {
  startup: () => call<Startup>("get_startup"),
  getSettings: () => call<Settings>("get_settings"),
  setSettings: (settings: Settings) => call<void>("set_settings", { settings }),
  defaultSettings: () => call<Settings>("default_settings"),
  testHotkey: (spec: string, which: "main" | "switcher") => call<void>("test_hotkey", { spec, which }),
  testTicketPattern: (pattern: string, sample: string) => call<string[]>("test_ticket_pattern", { pattern, sample }),
  pickFolder: () => call<string | null>("pick_folder"),
  getDailyTemplate: () => call<string>("get_daily_template"),
  setDailyTemplate: (text: string) => call<void>("set_daily_template", { text }),
  bufferOpen: (path: string) => call<Opened>("buffer_open", { path }),
  bufferClose: (path: string) => call<void>("buffer_close", { path }),
  bufferPush: (path: string, version: number, updates: BufferUpdate[]) =>
    call<boolean>("buffer_push", { path, version, updates }),
  bufferPull: (path: string, version: number) => call<BufferUpdate[] | null>("buffer_pull", { path, version }),
  bufferResolve: (path: string, reload: boolean) => call<void>("buffer_resolve", { path, reload }),
  bufferFlush: (path: string | null) => call<void>("buffer_flush", { path }),
  pinToggle: (path: string) => call<boolean>("pin_toggle", { path }),
  pinPath: () => call<string | null>("pin_path"),
  pinClose: () => call<void>("pin_close"),
  pinSetOnTop: (on: boolean) => call<void>("pin_set_on_top", { on }),
  openInMain: (path: string) => call<void>("open_in_main", { path }),
  smokeEnabled: () => call<boolean>("smoke_enabled"),
  smokeStep: (step: string) => call<void>("smoke_step", { step }),
  quickHide: () => call<void>("quick_hide"),
  quickResize: (height: number) => call<void>("quick_resize", { height }),
  quickOpen: (path: string) => call<void>("quick_open", { path }),
  quickCreate: (title: string) => call<string>("quick_create", { title }),
  resolveLink: (target: string) => call<string>("resolve_link", { target }),
  createNote: (title: string) => call<string>("create_note", { title }),
  dailyNote: (date: string) => call<string>("daily_note", { date }),
  archiveNote: (path: string) => call<string>("archive_note", { path }),
  renameNote: (path: string, title: string) => call<RenameResult>("rename_note", { path, title }),
  undoRename: (undoId: number) => call<string>("undo_rename", { undoId }),
  search: (query: string, includeArchived: boolean, current: string | null, limit = 50) =>
    call<SearchResults>("search", { query, includeArchived, current, limit }),
  setStatus: (path: string, status: string | null) => call<StatusResult>("set_status", { path, status }),
  listNotes: (query: string, includeDaily: boolean, includePutAway: boolean, sort: SortBy) =>
    call<SearchResults>("list_notes", { query, includeDaily, includePutAway, sort }),
  sidebar: () => call<{ active: SearchHit[]; more: number; recent: SearchHit[] }>("sidebar"),
  setUi: (key: string, value: unknown) => call<void>("set_ui", { key, value }),
  backlinks: (path: string) => call<SearchHit[]>("backlinks", { path }),
  archivedNames: () => call<string[]>("archived_names"),
  gerritEdit: (content: string, link: string) => call<TextEdit>("gerrit_edit", { content, link }),
  openUrl: (url: string) => call<void>("open_url", { url }),
  hideWindow: () => call<void>("hide_window"),
  quit: () => call<void>("quit_app"),
  saveSession: (path: string | null, anchor: number | null) => call<void>("save_session", { path, anchor }),
  readClipboard: () => call<string>("read_clipboard"),
  openSettingsFile: () => call<void>("open_settings_file"),
  openVaultFolder: () => call<void>("open_vault_folder"),
};

type Handler<T> = (payload: T) => void;
const mockListeners = new Map<string, Set<Handler<unknown>>>();

/** Subscribe to an event from Rust. Returns an unsubscribe function. */
export async function on<T>(event: string, handler: Handler<T>): Promise<() => void> {
  if (inTauri) return listen<T>(event, (e) => handler(e.payload));
  const set = mockListeners.get(event) ?? new Set();
  set.add(handler as Handler<unknown>);
  mockListeners.set(event, set);
  return () => set.delete(handler as Handler<unknown>);
}

/** Used by the mock backend to simulate events. */
export function mockEmit(event: string, payload: unknown) {
  mockListeners.get(event)?.forEach((h) => h(payload));
}

/** Window focus changes (hide/show/alt-tab). */
export async function onWindowFocus(handler: (focused: boolean) => void): Promise<() => void> {
  if (inTauri) {
    return getCurrentWindow().onFocusChanged(({ payload }) => handler(payload));
  }
  const f = () => handler(true);
  const b = () => handler(false);
  window.addEventListener("focus", f);
  window.addEventListener("blur", b);
  return () => {
    window.removeEventListener("focus", f);
    window.removeEventListener("blur", b);
  };
}
