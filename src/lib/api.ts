// Typed wrappers around the Rust commands. Outside Tauri (plain `npm run dev`
// in a browser) calls go to an in-memory mock so the UI can be worked on alone.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

export type NoteKind = "daily" | "ticket" | "note";

export interface NoteMeta {
  /** `vault:relative/path.md`. */
  path: string;
  vault: string;
  kind: NoteKind;
  archived: boolean;
  title: string;
  label: string | null;
  key: string | null;
  date: string | null;
  /** Front matter `jira`: overrides where the ticket opens. */
  ticketUrl: string | null;
  /** Other front matter keys as lists; link types pick theirs by key. */
  fields: Record<string, string[]>;
  aliases: string[];
  status: string | null;
  tags: string[];
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

/** An existing note's current text, read without opening or creating a note. */
export interface NotePreview {
  path: string;
  text: string;
}

/** A CodeMirror change set (as JSON) and the view that made it. */
export interface BufferUpdate {
  changes: unknown;
  clientID: string;
}

export interface SearchHit {
  path: string;
  vault: string;
  kind: NoteKind;
  title: string;
  label: string | null;
  key: string | null;
  status: string | null;
  tags: string[];
  archived: boolean;
  mtime: number;
  snippet: string | null;
  score: number;
}

export interface StatusDef {
  name: string;
  /** A Lucide icon name; see lib/icons.ts. */
  icon: string;
  color: string;
  active: boolean;
}

export interface StatusResult {
  path: string;
  edit: TextEdit | null;
  meta: NoteMeta;
}

export interface TagCount {
  name: string;
  count: number;
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
  openTicket: string;
  openLink: string;
  addLink: string;
  timestamp: string;
  backlinks: string;
  archive: string;
  help: string;
  quit: string;
  rename: string;
  openSettings: string;
  setStatus: string;
  setTags: string;
  notesView: string;
  toggleSidebar: string;
  focusSidebar: string;
  pinNote: string;
  switchVault: string;
}

export interface VaultDef {
  /** Stable id used in paths, `[[id:Note]]` links and `in:id` searches. */
  id: string;
  name: string;
  /** Empty for the default folder (Documents\StashNotes). */
  path: string;
  color: string;
  tickets: TicketSystem[];
  links: LinkType[];
}

/** A system whose items get a note each (Jira, GitHub issues). */
export interface TicketSystem {
  name: string;
  icon: string;
  /** Regex for keys. */
  pattern: string;
  /** `{key}` = the key, `{n}` = its trailing number. */
  url: string;
}

/** A kind of link notes collect in a front matter list (Gerrit, PRs, pages). */
export interface LinkType {
  name: string;
  icon: string;
  /** The front matter key. */
  field: string;
  /** Where a bare entry opens: `{n}` = the entry. */
  url: string;
}

export type SortBy = "edited" | "opened" | "status" | "title";

export interface NotesViewSettings {
  showDaily: boolean;
  showClosed: boolean;
  sort: SortBy;
  groupBy: "none" | "status" | "tag";
}

export interface Settings {
  schemaVersion: number;
  vaults: VaultDef[];
  hotkey: string;
  switcherHotkey: string;
  timestampFormat: string;
  onSummon: "last" | "daily";
  startHidden: boolean;
  autostart: boolean;
  showInTaskbar: boolean;
  alwaysOnTop: boolean;
  hideAfterExternalLink: boolean;
  theme: "system" | "light" | "dark";
  fontSize: number;
  fontFamily: string;
  lineHeight: number;
  spellCheck: boolean;
  accent: string;
  readableWidth: boolean;
  /** `system` follows the OS setting. */
  animations: "system" | "on" | "off";
  liveFormatting: boolean;
  smartPaste: boolean;
  restorePinned: boolean;
  notesView: NotesViewSettings;
  statuses: StatusDef[];
  /** Tag -> color name. */
  tagColors: Record<string, string>;
  defaultStatus: string;
  keys: Keys;
}

export interface Startup {
  settings: Settings;
  /** Each vault's folder, by id. */
  vaultPaths: Record<string, string>;
  currentVault: string;
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
  getDailyTemplate: (vault: string) => call<string>("get_daily_template", { vault }),
  setDailyTemplate: (vault: string, text: string) => call<void>("set_daily_template", { vault, text }),
  getVaultPaths: () => call<Record<string, string>>("get_vault_paths"),
  defaultVaultFolder: () => call<string>("default_vault_folder"),
  /** Make `vault` current; returns the note last open there, if any. */
  switchVault: (vault: string) => call<string | null>("switch_vault", { vault }),
  bufferOpen: (path: string) => call<Opened>("buffer_open", { path }),
  bufferClose: (path: string) => call<void>("buffer_close", { path }),
  bufferPush: (path: string, version: number, updates: BufferUpdate[]) =>
    call<boolean>("buffer_push", { path, version, updates }),
  bufferPull: (path: string, version: number) => call<BufferUpdate[] | null>("buffer_pull", { path, version }),
  bufferResolve: (path: string, reload: boolean) => call<void>("buffer_resolve", { path, reload }),
  bufferFlush: (path: string | null) => call<void>("buffer_flush", { path }),
  pinToggle: (path: string) => call<boolean>("pin_toggle", { path }),
  pinnedPaths: () => call<string[]>("pinned_paths"),
  pinPath: () => call<string | null>("pin_path"),
  pinClose: () => call<void>("pin_close"),
  pinSetOnTop: (on: boolean) => call<void>("pin_set_on_top", { on }),
  openInMain: (path: string) => call<void>("open_in_main", { path }),
  smokeEnabled: () => call<boolean>("smoke_enabled"),
  smokeStep: (step: string) => call<void>("smoke_step", { step }),
  quickHide: () => call<void>("quick_hide"),
  quickResize: (height: number) => call<void>("quick_resize", { height }),
  quickOpen: (path: string) => call<void>("quick_open", { path }),
  /** `vault`: where it goes (null: the current vault, or the one its ticket key belongs to). */
  quickCreate: (title: string, vault: string | null) => call<string>("quick_create", { title, vault }),
  resolveLink: (target: string, from: string | null) => call<string>("resolve_link", { target, from }),
  notePreview: (target: string, from: string | null) => call<NotePreview>("note_preview", { target, from }),
  importImage: (path: string, bytes: number[]) => call<string>("import_image", { path, bytes }),
  readImage: (path: string, source: string) => call<ArrayBuffer>("read_image", { path, source }),
  createNote: (title: string, vault: string) => call<string>("create_note", { title, vault }),
  dailyNote: (date: string, vault: string) => call<string>("daily_note", { date, vault }),
  archiveNote: (path: string) => call<string>("archive_note", { path }),
  renameNote: (path: string, title: string) => call<RenameResult>("rename_note", { path, title }),
  undoRename: (undoId: number) => call<string>("undo_rename", { undoId }),
  /** `prefer`: the vault ranked first (default: the current one). */
  search: (query: string, includeArchived: boolean, current: string | null, limit = 50, prefer: string | null = null) =>
    call<SearchResults>("search", { query, includeArchived, current, limit, prefer }),
  setStatus: (path: string, status: string | null) => call<StatusResult>("set_status", { path, status }),
  setTags: (path: string, tags: string[]) => call<StatusResult>("set_tags", { path, tags }),
  listTags: () => call<TagCount[]>("list_tags"),
  renameTag: (from: string, to: string) => call<{ notes: number; undoId: number }>("rename_tag", { from, to }),
  undoTagRename: (undoId: number) => call<void>("undo_tag_rename", { undoId }),
  listNotes: (query: string, includeDaily: boolean, includePutAway: boolean, sort: SortBy, vault: string) =>
    call<SearchResults>("list_notes", { query, includeDaily, includePutAway, sort, vault }),
  sidebar: (vault: string) => call<{ active: SearchHit[]; more: number; recent: SearchHit[] }>("sidebar", { vault }),
  setUi: (key: string, value: unknown) => call<void>("set_ui", { key, value }),
  backlinks: (path: string) => call<SearchHit[]>("backlinks", { path }),
  archivedNames: (vault: string) => call<string[]>("archived_names", { vault }),
  linkEdit: (content: string, field: string, item: string) => call<TextEdit>("link_edit", { content, field, item }),
  openUrl: (url: string) => call<void>("open_url", { url }),
  hideWindow: () => call<void>("hide_window"),
  quit: () => call<void>("quit_app"),
  saveSession: (path: string | null, anchor: number | null) => call<void>("save_session", { path, anchor }),
  readClipboard: () => call<string>("read_clipboard"),
  openSettingsFile: () => call<void>("open_settings_file"),
  openVaultFolder: (vault: string) => call<void>("open_vault_folder", { vault }),
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
