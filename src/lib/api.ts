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

export interface NoteData {
  content: string;
  meta: NoteMeta;
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
}

export interface Settings {
  vaultPath: string;
  hotkey: string;
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
  liveFormatting: boolean;
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
  openNote: (path: string) => call<NoteData>("open_note", { path }),
  readNote: (path: string) => call<string | null>("read_note", { path }),
  saveNote: (path: string, content: string) => call<NoteMeta>("save_note", { path, content }),
  resolveLink: (target: string) => call<string>("resolve_link", { target }),
  createNote: (title: string) => call<string>("create_note", { title }),
  dailyNote: (date: string) => call<string>("daily_note", { date }),
  archiveNote: (path: string) => call<string>("archive_note", { path }),
  renameNote: (path: string, title: string) => call<RenameResult>("rename_note", { path, title }),
  undoRename: (undoId: number) => call<string>("undo_rename", { undoId }),
  search: (query: string, includeArchived: boolean, current: string | null, limit = 50) =>
    call<SearchResults>("search", { query, includeArchived, current, limit }),
  setStatus: (path: string, status: string | null) => call<StatusResult>("set_status", { path, status }),
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
