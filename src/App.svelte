<script lang="ts">
  import { onMount } from "svelte";
  import Toasts from "./components/Toasts.svelte";
  import { toaster, type ToastAction, type ToastKind } from "./lib/toasts.svelte";
  import { EditorState, type Extension } from "@codemirror/state";
  import { EditorView } from "@codemirror/view";
  import { completionStatus } from "@codemirror/autocomplete";
  import Header from "./components/Header.svelte";
  import Switcher from "./components/Switcher.svelte";
  import Backlinks from "./components/Backlinks.svelte";
  import Help from "./components/Help.svelte";
  import Prompt from "./components/Prompt.svelte";
  import NoteActions from "./components/NoteActions.svelte";
  import FileDialog from "./components/FileDialog.svelte";
  import Picker from "./components/Picker.svelte";
  import StatusPicker from "./components/StatusPicker.svelte";
  import TagPicker from "./components/TagPicker.svelte";
  import NotesView, { type NotesViewState } from "./components/NotesView.svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import SettingsView from "./components/SettingsView.svelte";
  import { applyAppearance } from "./lib/appearance";
  import { api, on, onWindowFocus, type Keys, type NoteMeta, type Opened, type SearchHit, type Settings, type TagCount } from "./lib/api";
  import { NoteSync } from "./lib/buffer";
  import { pickImages } from "./lib/images";
  import {
    decoConfig,
    editorExtensions,
    editorState,
    refreshDecorations,
    setEditorDark,
    setLiveFormatting,
    setSpellCheck,
  } from "./lib/editor";
  import { liveConfig, propertiesHooks } from "./lib/live";
  import { anyTickets, linkItems, linkTypeForUrl, systemForKey, ticketFromUrl, ticketUrl, vaultTickets } from "./lib/links";
  import { statusInfo } from "./lib/status";
  import { History, type Entry } from "./lib/history";
  import { formatKey, keyCapture, matches, matchesChordTail, parseChord, parseKey, type KeySpec } from "./lib/keys";
  import {
    checkboxToggle,
    firstTicket,
    isoDate,
    refAt,
    ticketRegex,
    timestamp,
    todoCursor,
  } from "./lib/text";

  type NoteTarget = Pick<NoteMeta, "path" | "title" | "kind" | "status" | "key">;
  type Overlay =
    | { type: "noteActions"; target: NoteTarget }
    | { type: "file"; mode: "move" | "delete"; target: NoteTarget }
    | { type: "switcher"; mode: "open" | "link"; sel?: { from: number; to: number; text: string } }
    | { type: "help" }
    | { type: "prompt"; title: string; value: string; placeholder?: string; submit: (v: string) => void }
    | { type: "picker"; title: string; items: { label: string; detail?: string; color?: string }[]; choose: (i: number) => void }
    | { type: "status"; path: string; current: string | null; title: string }
    | { type: "tags"; path: string; tags: string[]; all: TagCount[]; title: string };


  let settings = $state<Settings | null>(null);
  /** Each vault's folder, by id. */
  let vaultPaths = $state<Record<string, string>>({});
  /** The vault the sidebar, Notes view, Today and new notes belong to. */
  let currentVault = $state("");
  let settingsPath = $state("");
  let meta = $state<NoteMeta | null>(null);
  let dirty = $state(false);
  /** The open note's ticket key (its own, or the first one it mentions). */
  let ticketKey = $state<string | null>(null);
  /** Paths of notes open in pinned windows. */
  let pinnedPaths = $state<string[]>([]);
  let backlinkItems = $state<SearchHit[]>([]);
  let backlinksOpen = $state(false);
  let overlay = $state<Overlay | null>(null);
  /** Set when the open note changed on disk while it had unsaved edits (or it vanished). */
  let conflict = $state<{ deleted: boolean } | null>(null);
  let nav = $state({ canBack: false, canForward: false });
  /** What fills the main area. The editor stays mounted underneath full-window views. */
  let screen = $state<"editor" | "notes" | "settings">("editor");
  /** Where Esc returns to from the settings screen. */
  let beforeSettings: "editor" | "notes" = "editor";
  /** The notes folder is being indexed (after changing it). */
  let indexing = $state(false);
  let notesState = $state<NotesViewState>({ query: "", sort: "edited", group: "none", showDaily: false, showPutAway: false });
  /** Bumped when notes change, so lists reload. */
  let listVersion = $state(0);
  let notesView = $state<NotesView>();

  let side = $state<{ active: SearchHit[]; more: number; recent: SearchHit[] }>({ active: [], more: 0, recent: [] });
  /** Thin rail by default; the hamburger (Ctrl+\\) expands it. Remembered. */
  let sideExpanded = $state(false);
  let sideWidth = $state(240);
  let winWidth = $state(window.innerWidth);
  const sideOverlay = $derived(winWidth < 720);
  let sidebar = $state<Sidebar>();

  /** History path standing for the Notes view. */
  const NOTES = "::notes";

  let editorEl: HTMLDivElement;
  let view: EditorView;
  let extensions: Extension[];
  const hist = new History();
  let currentPath = $state<string | null>(null);
  /** The newest explicitly created note can be discarded directly from its notice. */
  let recentCreation: { target: NoteTarget; toast: number } | null = null;
  /** Sync with the Rust-owned buffer of the open note. */
  let sync: NoteSync | null = null;
  /** The buffer has edits not yet written to disk. */
  let serverDirty = false;
  /** Ticket keys in the open note's vault (never matches in a plain vault). */
  let tickets = /(?!)/g;
  let keySpecs: Partial<Record<keyof Keys, KeySpec | null>> = {};
  let pinChord: KeySpec[] = [];
  /** The first key of a chord was pressed; shows the hint until the second (or 1.5 s). */
  let chordPending = $state(false);
  let chordTimer: ReturnType<typeof setTimeout> | undefined;

  // ---------- small helpers ----------

  function toast(text: string, action?: ToastAction, kind: ToastKind = "info") {
    toaster.show(text, kind, action);
  }

  /** A success notice. */
  function done(text: string, action?: ToastAction) {
    toaster.show(text, "done", action);
  }

  function fail(e: unknown) {
    toaster.show(String(e instanceof Error ? e.message : e), "error");
  }

  function updateNav() {
    nav = { canBack: hist.canBack, canForward: hist.canForward };
  }

  function docText() {
    return view.state.doc.toString();
  }

  function head() {
    return view.state.selection.main.head;
  }

  function focusEditor() {
    if (!overlay) view?.focus();
  }

  /** A ticket key in any vault (new ticket notes go to the vault whose key it is). */
  function isTicketKey(q: string) {
    const re = anyTickets(settings?.vaults ?? []);
    return !!re && new RegExp(`^(?:${re.source})$`).test(q.trim().toUpperCase());
  }

  /** The settings of the vault the open note is in (else the current one). */
  function noteVault() {
    return settings?.vaults.find((v) => v.id === (meta?.vault ?? currentVault));
  }

  /** Point key highlighting and following at the open note's vault. */
  function applyTickets() {
    const re = vaultTickets(noteVault());
    tickets = re ? ticketRegex(re.source) : /(?!)/g;
    decoConfig.tickets = tickets;
    if (view) refreshDecorations(view);
  }

  /** `PROJ-1234` or `PROJ-1234 Do this thing`: creates a ticket note. */
  function startsWithTicket(q: string) {
    return isTicketKey(q.trim().split(/\s+/)[0] ?? "");
  }

  function applySettings(s: Settings) {
    settings = s;
    keySpecs = Object.fromEntries(Object.entries(s.keys).map(([k, v]) => [k, parseKey(v)]));
    pinChord = parseChord(s.keys.pinNote);
    keySpecs.pinNote = null; // a chord, handled separately
    applyTickets();
    applyAppearance(s);
    liveConfig.enabled = s.liveFormatting;
    editorState.spellCheck = s.spellCheck;
    applyDark();
    if (view) {
      setLiveFormatting(view, s.liveFormatting);
      setSpellCheck(view, s.spellCheck);
      ticketKey = computeTicketKey();
    }
  }

  const darkQuery = window.matchMedia("(prefers-color-scheme: dark)");

  /** Keep the editor's dark flag in step with the theme setting and Windows. */
  function applyDark() {
    const theme = settings?.theme ?? "system";
    const dark = theme === "dark" || (theme === "system" && darkQuery.matches);
    editorState.dark = dark;
    if (view) setEditorDark(view, dark);
  }

  function computeTicketKey(): string | null {
    if (meta?.kind === "ticket" && meta.key) return meta.key;
    return meta?.kind === "daily" ? null : firstTicket(docText(), tickets);
  }

  // ---------- saving ----------
  // Rust owns the text and autosaves it; the editor only pushes its edits.

  function updateDirty() {
    dirty = serverDirty || (sync?.pending ?? false);
  }

  /** Push local edits and have Rust write the note now. */
  async function flush(): Promise<void> {
    if (!sync) return;
    await sync.drain();
    await api.bufferFlush(sync.path).catch(() => {});
  }

  function onDocChange() {
    sync?.push().then(updateDirty);
    updateDirty();
  }

  // ---------- opening notes ----------

  /** Where the cursor goes when a note is opened fresh. */
  function defaultCursor(data: Opened): number {
    if (data.meta.kind === "daily") return todoCursor(data.text) ?? data.text.length;
    // A ticket with no name yet: put the caret after `# PROJ-123 ` so the name can be typed.
    if (data.meta.kind === "ticket" && !data.meta.label) {
      const m = /^# .*$/m.exec(data.text);
      if (m) return m.index + m[0].length;
    }
    return data.text.length;
  }

  /** Remember cursor and scroll for the entry on screen. */
  function remember() {
    const e = hist.current;
    if (e && view && screen === "editor" && e.path === currentPath) {
      e.anchor = head();
      e.scroll = view.scrollDOM.scrollTop;
    }
  }

  async function show(path: string, entry: Entry | null, restore: boolean) {
    if (path === NOTES) {
      screen = "notes";
      backlinksOpen = false;
      updateNav();
      requestAnimationFrame(() => notesView?.focus());
      return;
    }
    const data = await api.bufferOpen(path);
    if (sync && sync.path !== path) {
      const old = sync.path;
      sync.close();
      api.bufferClose(old).catch(() => {});
    } else sync?.close();
    screen = "editor";
    currentPath = path;
    meta = data.meta;
    // The window follows the note into its vault (Rust records it too).
    if (data.meta.vault && data.meta.vault !== currentVault) enterVault(data.meta.vault);
    applyTickets();
    serverDirty = data.dirty;
    conflict = data.deleted ? { deleted: true } : null;
    view.setState(EditorState.create({ doc: data.text, extensions: [extensions, NoteSync.extension(data)] }));
    sync = new NoteSync(path, view, () => reopen());
    updateDirty();
    const len = data.text.length;
    const anchor = restore && entry && entry.anchor >= 0 ? Math.min(entry.anchor, len) : defaultCursor(data);
    view.dispatch({ selection: { anchor }, effects: EditorView.scrollIntoView(anchor, { y: "center" }) });
    if (restore && entry) {
      const top = entry.scroll;
      requestAnimationFrame(() => (view.scrollDOM.scrollTop = top));
    }
    ticketKey = computeTicketKey();
    // The properties row shows the ticket system, which needed the key first.
    refreshDecorations(view);
    updateNav();
    refreshBacklinks();
    refreshSidebar();
    if (sideOverlay) sideExpanded = false;
    focusEditor();
  }

  /** The view fell behind its buffer: load it again where we are. */
  async function reopen() {
    if (!currentPath) return;
    remember();
    await show(currentPath, hist.current, true).catch(fail);
  }

  /** Point the sidebar, Notes view and link styling at vault `id`. */
  function enterVault(id: string) {
    currentVault = id;
    listVersion++;
    refreshArchived();
  }

  async function refreshArchived() {
    try {
      decoConfig.archived = new Set(await api.archivedNames(currentVault));
      refreshDecorations(view);
    } catch {
      // keep the old set
    }
  }

  /** Switch to vault `id`: back to the note last open there, else its daily note. */
  async function switchVault(id: string) {
    try {
      await flush();
      const last = await api.switchVault(id);
      enterVault(id);
      navigate(last ?? (await api.dailyNote(isoDate(), id)));
    } catch (e) {
      fail(e);
    }
  }

  function pickVault() {
    const vs = settings?.vaults ?? [];
    if (vs.length < 2) {
      toast("Add another vault in Settings → Vaults to switch between them");
      return;
    }
    overlay = {
      type: "picker",
      title: "Switch vault",
      items: vs.map((v) => ({ label: v.name, detail: v.id === currentVault ? "current" : `in:${v.id}`, color: v.color })),
      choose: (i) => {
        closeOverlay();
        if (vs[i].id !== currentVault) switchVault(vs[i].id);
      },
    };
  }

  async function refreshSidebar() {
    try {
      side = await api.sidebar(currentVault);
    } catch {
      // keep the previous groups
    }
  }

  function setSideExpanded(on: boolean) {
    sideExpanded = on;
    api.setUi("sidebarExpanded", on).catch(() => {});
  }

  let widthTimer: ReturnType<typeof setTimeout> | undefined;
  function setSideWidth(w: number) {
    sideWidth = w;
    clearTimeout(widthTimer);
    widthTimer = setTimeout(() => api.setUi("sidebarWidth", w).catch(() => {}), 300);
  }

  function sidebarHasFocus() {
    return !!document.activeElement?.closest(".sidebar");
  }

  function focusMain() {
    if (screen === "notes") notesView?.focus();
    else view.focus();
  }

  /** Open a note, recording it in history (or just peeking at it). */
  async function navigate(path: string, opts: { peek?: boolean } = {}) {
    try {
      await flush();
      remember();
      if (path === currentPath && !opts.peek && screen === "editor") {
        focusEditor();
        return;
      }
      const entry = opts.peek ? hist.peekAt(path) : hist.push(path);
      await show(path, entry, false);
    } catch (e) {
      fail(e);
    }
  }

  async function back() {
    await flush();
    remember();
    const e = hist.back();
    if (e) await show(e.path, e, true).catch(fail);
    updateNav();
  }

  async function forward() {
    await flush();
    remember();
    const e = hist.forward();
    if (e) await show(e.path, e, true).catch(fail);
    updateNav();
  }

  /** Open the Notes view, optionally with a query (e.g. `@active`). */
  async function openNotesView(query?: string) {
    if (query !== undefined) notesState.query = query;
    if (screen === "notes") {
      notesView?.focus();
      return;
    }
    await navigate(NOTES);
  }

  /** Show the settings screen (or leave it). It isn't a history entry. */
  async function toggleSettings(open = screen !== "settings") {
    if (!open) {
      screen = beforeSettings;
      requestAnimationFrame(focusMain);
      return;
    }
    if (screen === "settings") return;
    await flush();
    remember();
    beforeSettings = screen;
    screen = "settings";
  }

  /** Leave the Notes view: back to where you came from, else today's note. */
  async function leaveNotesView() {
    if (hist.canBack) await back();
    else await openDaily();
  }

  async function openDaily() {
    try {
      navigate(await api.dailyNote(isoDate(), currentVault));
    } catch (e) {
      fail(e);
    }
  }

  // ---------- links ----------

  /** Follow the reference at `pos`. Returns false if there is none. */
  function followAt(pos: number): boolean {
    const line = view.state.doc.lineAt(pos);
    const ref = refAt(line.text, pos - line.from, tickets);
    if (!ref) return false;
    if (ref.kind === "url") openExternal(ref.target);
    else api.resolveLink(ref.target, currentPath).then((p) => navigate(p), fail);
    return true;
  }

  function followOrToggle() {
    const pos = head();
    if (followAt(pos)) return;
    const line = view.state.doc.lineAt(pos);
    const box = checkboxToggle(line.text);
    if (box) {
      view.dispatch({ changes: { from: line.from + box.at, to: line.from + box.at + 1, insert: box.insert } });
    }
  }

  async function openExternal(url: string) {
    await flush();
    remember();
    api.saveSession(currentPath, head());
    api.openUrl(url).catch(fail);
  }

  /** Open the ticket this note is about (or the key under the cursor, or the first it mentions). */
  function openTicket() {
    if (!meta || !settings) return;
    const v = noteVault();
    if (meta.kind === "ticket" && meta.key) {
      const sys = systemForKey(v, meta.key);
      if (meta.ticketUrl) openExternal(meta.ticketUrl);
      else if (sys) openExternal(ticketUrl(sys, meta.key));
      return;
    }
    const pos = head();
    const line = view.state.doc.lineAt(pos);
    const ref = refAt(line.text, pos - line.from, tickets);
    let key: string | null = null;
    if (ref?.kind === "ticket") key = ref.target;
    else if (ref?.kind === "wiki" && isTicketKey(ref.target)) key = ref.target.toUpperCase();
    key ??= firstTicket(docText(), tickets);
    const sys = key ? systemForKey(v, key) : null;
    if (key && sys) openExternal(ticketUrl(sys, key));
    else toast(v?.tickets.length ? "No ticket key in this note" : "This vault has no ticket system — add one in Settings → Vaults");
  }

  /** Open one of the note's attached links (a picker when there are several). */
  async function openLink() {
    if (!settings || !meta) return;
    await flush();
    const v = noteVault();
    const items = linkItems(meta, v);
    if (items.length === 0) {
      toast(v?.links.length ? `No links yet — add one with ${formatKey(settings.keys.addLink)}` : "This vault has no link types — add one in Settings → Vaults");
    } else if (items.length === 1) {
      openExternal(items[0].url);
    } else {
      overlay = {
        type: "picker",
        title: "Open link",
        items: items.map((l) => ({ label: `${l.type.name} · ${l.item}`, detail: l.url })),
        choose: (i) => {
          closeOverlay();
          openExternal(items[i].url);
        },
      };
    }
  }

  /** Ask for a link to attach: its URL picks the link type (or a picker, for a bare id). */
  async function addLink() {
    if (!settings || !currentPath || !meta) return;
    const v = noteVault();
    const types = v?.links ?? [];
    if (!types.length) {
      toast("This vault has no link types — add one in Settings → Vaults");
      return;
    }
    const clip = (await api.readClipboard().catch(() => "")).trim();
    const names = types.map((t) => t.name).join(", ");
    overlay = {
      type: "prompt",
      title: `Add a link (${names}) — paste a URL${types.length === 1 ? " or type an id" : ""}`,
      placeholder: types.length === 1 ? `https://… or 12345` : "https://…",
      value: linkTypeForUrl(clip, v) ? clip : "",
      submit: (value) => {
        closeOverlay();
        const text = value.trim();
        if (!text) return;
        const type = linkTypeForUrl(text, v) ?? (types.length === 1 ? types[0] : null);
        if (type) attachLink(type.field, type.name, text, false);
        else
          overlay = {
            type: "picker",
            title: `Which kind of link is ${text}?`,
            items: types.map((t) => ({ label: t.name, detail: t.url.replaceAll("{n}", text) })),
            choose: (i) => {
              closeOverlay();
              attachLink(types[i].field, types[i].name, text, false);
            },
          };
      },
    };
  }

  /** Append `item` to the note's front matter list `field`, optionally with an Undo toast. */
  async function attachLink(field: string, name: string, item: string, undoable: boolean) {
    try {
      const path = currentPath;
      const edit = await api.linkEdit(docText(), field, item);
      view.dispatch({ changes: edit });
      await flush();
      const msg = `Added ${name} link (${(meta?.fields[field]?.length ?? 0) || 1} total)`;
      if (!undoable) return done(msg);
      done(msg, {
        label: "Undo",
        run: () => {
          if (currentPath !== path) return;
          const doc = docText();
          const at = doc.indexOf(edit.insert);
          if (at >= 0 && doc.indexOf(edit.insert, at + 1) < 0) {
            view.dispatch({ changes: { from: at, to: at + edit.insert.length } });
            flush();
          }
        },
      });
    } catch (e) {
      fail(e);
    }
  }

  // ---------- smart paste ----------

  /** Set by Ctrl+Shift+V: the next paste goes in as typed. */
  let rawPaste = false;

  /** A pasted ticket URL (in any vault) becomes its key. */
  function pasteKey(text: string): string | null {
    if (!settings?.smartPaste) return null;
    return ticketFromUrl(text, settings.vaults)?.key ?? null;
  }

  function smartPaste(text: string): boolean {
    if (rawPaste || !settings?.smartPaste) {
      rawPaste = false;
      return false;
    }
    const key = pasteKey(text);
    if (key) {
      view.dispatch(view.state.replaceSelection(key));
      return true;
    }
    // A link the vault collects (a Gerrit change, a PR, ...) goes in the front matter.
    const type = meta && meta.kind !== "daily" ? linkTypeForUrl(text, noteVault()) : null;
    if (type) {
      attachLink(type.field, type.name, text.trim(), true);
      return true;
    }
    return false;
  }

  function insertTimestamp() {
    view.dispatch(view.state.replaceSelection(timestamp(new Date(), settings?.timestampFormat)));
    view.focus();
  }

  // ---------- archive / rename ----------

  function moved(from: string, to: string) {
    hist.rename(from, to);
    if (recentCreation?.target.path === from) recentCreation.target.path = to;
    if (sync?.path === from) sync.path = to;
    if (currentPath === from) {
      currentPath = to;
      const vault = to.slice(0, to.indexOf(":"));
      if (meta) meta = { ...meta, path: to, vault, archived: to.slice(to.indexOf(":") + 1).startsWith("archive/") };
      if (vault !== currentVault) enterVault(vault);
      applyTickets();
      ticketKey = computeTicketKey();
      conflict = null;
      refreshDecorations(view);
      refreshBacklinks();
    }
    listVersion++;
    refreshSide();
  }

  function noteActions(target: NoteTarget | null = meta) {
    if (target) overlay = { type: "noteActions", target };
  }

  async function moveNote(target: NoteTarget, vault: string) {
    await flush();
    const to = await api.moveNote(target.path, vault);
    moved(target.path, to);
    closeOverlay();
    const name = settings?.vaults.find((v) => v.id === vault)?.name ?? vault;
    done(`Moved to ${name}`);
  }

  function clearCreation(path?: string) {
    if (recentCreation && (!path || recentCreation.target.path === path)) {
      toaster.dismiss(recentCreation.toast);
      recentCreation = null;
    }
  }

  function noteCreated(target: NoteTarget) {
    clearCreation();
    const creation = { target: { ...target }, toast: 0 };
    recentCreation = creation;
    creation.toast = toaster.show(`Created ${target.title}`, "done", { label: "Delete", run: async () => {
      if (recentCreation !== creation) return;
      try { await deleteNote(creation.target); }
      catch (e) { fail(e); }
    } }, 10000);
  }

  function deleted(path: string) {
    clearCreation(path);
    hist.remove(path);
    if (currentPath === path) {
      sync?.close();
      sync = null;
      currentPath = null;
      meta = null;
      serverDirty = false;
      dirty = false;
      conflict = null;
      backlinkItems = [];
      view.setState(EditorState.create({ doc: "", extensions }));
      if (screen === "editor") screen = "notes";
    }
    updateNav();
    listVersion++;
    refreshSide();
  }

  async function deleteNote(target: NoteTarget) {
    await flush();
    const undoId = await api.deleteNote(target.path);
    deleted(target.path);
    closeOverlay();
    done(`Deleted ${target.title}`, { label: "Undo", run: async () => {
      try {
        const restored = await api.undoDelete(undoId);
        await navigate(restored);
        refreshSide();
      } catch (e) { fail(e); }
    } });
  }

  async function toggleArchive() {
    if (!currentPath) return;
    try {
      await archivePath(currentPath);
    } catch (e) {
      fail(e);
    }
  }

  async function archivePath(from: string) {
    await flush();
    const to = await api.archiveNote(from);
    moved(from, to);
    clearCreation(to);
    const archived = to.slice(to.indexOf(":") + 1).startsWith("archive/");
    done(archived ? "Archived" : "Unarchived", {
      label: "Undo",
      run: async () => {
        try {
          await flush();
          moved(to, await api.archiveNote(to));
        } catch (e) {
          fail(e);
        }
      },
    });
  }

  async function archiveInstead(target: NoteTarget) {
    if (!target.path.slice(target.path.indexOf(":") + 1).startsWith("archive/")) await archivePath(target.path);
    closeOverlay();
  }

  function startRename(target: NoteTarget | null = meta) {
    if (!target) return;
    if (target.kind !== "note") {
      toast(
        target.kind === "daily"
          ? "Daily notes are named by their date"
          : "Ticket notes are named by their key — edit the name after the key in the first heading instead",
      );
      return;
    }
    const from = target.path;
    overlay = {
      type: "prompt",
      title: "Rename note (links to it are updated)",
      value: target.title,
      submit: async (title) => {
        closeOverlay();
        try {
          await flush();
          remember();
          const r = await api.renameNote(from, title);
          moved(from, r.newPath);
          if (currentPath === r.newPath) await show(r.newPath, hist.current, true);
          listVersion++;
          const links = r.linksUpdated === 1 ? "1 link" : `${r.linksUpdated} links`;
          const notes = r.filesChanged === 1 ? "1 other note" : `${r.filesChanged} other notes`;
          done(`Renamed — updated ${links} in ${notes}`, {
            label: "Undo",
            run: async () => {
              try {
                await flush();
                remember();
                const old = await api.undoRename(r.undoId);
                moved(r.newPath, old);
                if (currentPath === old) await show(old, hist.current, true);
                listVersion++;
                refreshSide();
              } catch (e) {
                fail(e);
              }
            },
          });
        } catch (e) {
          fail(e);
        }
      },
    };
  }

  // ---------- status ----------

  /** Open the status picker for `path` (default: the current note). */
  function pickStatus(path = currentPath, current = meta?.status ?? null, title = meta?.title ?? "") {
    if (!path) return;
    if (path === currentPath && meta?.kind === "daily") {
      toast("Daily notes don't have a status");
      return;
    }
    overlay = { type: "status", path, current, title: `Status for ${title}` };
  }

  async function applyStatus(path: string, status: string | null) {
    closeOverlay();
    try {
      if (path === currentPath) await flush();
      const r = await api.setStatus(path, status);
      // An open note receives the edit through its buffer.
      if (path === currentPath) meta = r.meta;
      if (r.path !== path) {
        moved(path, r.path);
        done(r.meta.archived ? "Closed — moved to the archive" : "Reopened — moved out of the archive");
      }
      listVersion++;
      refreshSide();
    } catch (e) {
      fail(e);
    }
  }

  // ---------- tags ----------

  /** Open the tag picker for `path` (default: the current note). */
  async function pickTags(path = currentPath, tags = meta?.tags ?? [], title = meta?.title ?? "") {
    if (!path) return;
    if (path === currentPath && meta?.kind === "daily") {
      toast("Daily notes don't have tags");
      return;
    }
    let all: TagCount[] = [];
    try {
      all = await api.listTags();
    } catch {
      // the picker still works with the note's own tags
    }
    overlay = { type: "tags", path, tags, all, title: `Tags for ${title}` };
  }

  async function applyTags(path: string, tags: string[]) {
    try {
      if (path === currentPath) await flush();
      const r = await api.setTags(path, tags);
      if (path === currentPath) meta = r.meta;
      listVersion++;
      refreshSide();
    } catch (e) {
      fail(e);
    }
  }

  /** The Notes view, filtered to one tag. */
  function showTag(tag: string) {
    openNotesView(`#${tag}`);
  }

  /** Run `pick` on the focused sidebar row, the selected Notes view row, or the open note. */
  function forTarget(pick: (hit: SearchHit | null) => void) {
    const sideHit = sidebarHasFocus() ? sidebar?.focusedHit() : null;
    if (sideHit) pick(sideHit);
    else if (screen === "notes") {
      const h = notesView?.selected();
      if (h) pick(h);
    } else pick(null);
  }

  // ---------- pinned notes ----------

  /** Pop the current note (or the focused sidebar / Notes view row) out, or close its pinned window. */
  async function togglePin() {
    const hit = sidebarHasFocus() ? sidebar?.focusedHit() : screen === "notes" ? notesView?.selected() : null;
    const path = hit?.path ?? (screen === "editor" ? currentPath : null);
    if (!path) return;
    try {
      await flush();
      const pinned = await api.pinToggle(path);
      done(pinned ? "Pinned — it stays on top beside your work" : "Unpinned");
    } catch (e) {
      fail(e);
    }
  }

  // ---------- overlays ----------

  function openSwitcher() {
    if (overlay?.type === "switcher") {
      closeOverlay();
      return;
    }
    const sel = view.state.selection.main;
    if (!sel.empty) {
      overlay = {
        type: "switcher",
        mode: "link",
        sel: { from: sel.from, to: sel.to, text: view.state.sliceDoc(sel.from, sel.to) },
      };
    } else {
      overlay = { type: "switcher", mode: "open" };
    }
  }

  function closeOverlay() {
    overlay = null;
    // Unless another overlay opened in the meantime (Esc then Ctrl+K, quickly).
    requestAnimationFrame(() => !overlay && (screen === "notes" ? notesView?.focus() : view.focus()));
  }

  function insertLink(target: string, sel: { from: number; to: number; text: string }) {
    const label = sel.text.replace(/[\[\]|\n]/g, " ").trim();
    const insert = label && label !== target ? `[[${target}|${label}]]` : `[[${target}]]`;
    view.dispatch({ changes: { from: sel.from, to: sel.to, insert }, selection: { anchor: sel.from + insert.length } });
  }

  function switcherChoose(hit: SearchHit, peek: boolean) {
    const o = overlay;
    closeOverlay();
    if (o?.type === "switcher" && o.mode === "link" && o.sel) insertLink(hit.title, o.sel);
    else navigate(hit.path, { peek });
  }

  async function switcherCreate(query: string, vault: string | null) {
    const o = overlay;
    closeOverlay();
    try {
      const path = await api.createNote(query, vault ?? currentVault);
      if (o?.type === "switcher" && o.mode === "link" && o.sel) {
        const title = query.split(/\s+/).filter((w) => w && !/^(@|#|in:)./.test(w)).join(" ");
        insertLink(startsWithTicket(title) ? title.split(/\s+/)[0].toUpperCase() : title, o.sel);
      } else {
        navigate(path);
      }
    } catch (e) {
      fail(e);
    }
  }

  // ---------- backlinks & external changes ----------

  async function refreshBacklinks() {
    const path = currentPath;
    if (!path) return;
    try {
      const items = await api.backlinks(path);
      if (path === currentPath) backlinkItems = items;
    } catch {
      // ignore; the strip just stays stale
    }
  }

  let sideTimer: ReturnType<typeof setTimeout> | undefined;
  function refreshSide() {
    clearTimeout(sideTimer);
    sideTimer = setTimeout(async () => {
      refreshBacklinks();
      refreshSidebar();
      try {
        decoConfig.archived = new Set(await api.archivedNames(currentVault));
        refreshDecorations(view);
      } catch {
        // ignore
      }
    }, 150);
  }

  function onVaultChanged() {
    refreshSide();
    listVersion++;
  }

  function onBufferState(ev: { path: string; dirty: boolean; meta: NoteMeta | null }) {
    if (ev.path !== currentPath) return;
    serverDirty = ev.dirty;
    if (ev.meta) {
      meta = ev.meta;
      const key = computeTicketKey();
      if (key !== ticketKey) {
        ticketKey = key;
        refreshDecorations(view);
      }
    }
    if (!ev.dirty && conflict && !conflict.deleted) conflict = null;
    updateDirty();
  }

  function onBufferConflict(ev: { path: string; deleted: boolean }) {
    if (ev.path === currentPath) conflict = { deleted: ev.deleted };
  }

  async function resolveConflict(reload: boolean) {
    if (!currentPath) return;
    try {
      await sync?.drain();
      await api.bufferResolve(currentPath, reload);
      conflict = null;
    } catch (e) {
      fail(e);
    }
    focusEditor();
  }

  // ---------- window lifecycle ----------

  async function onSummoned() {
    if (!settings) return;
    if (screen === "notes") {
      requestAnimationFrame(() => notesView?.focus());
      return;
    }
    const today = isoDate();
    const staleDaily = meta?.kind === "daily" && !meta.archived && meta.date !== null && meta.date < today;
    const wantDaily = settings.onSummon === "daily" || staleDaily;
    if (wantDaily && !(meta?.kind === "daily" && meta.date === today)) await openDaily();
    else if (!overlay) requestAnimationFrame(() => view.focus());
  }

  function onBlur() {
    flush();
    remember();
    if (currentPath) api.saveSession(currentPath, head()).catch(() => {});
  }

  async function quit() {
    await flush();
    remember();
    await api.saveSession(currentPath, view ? head() : null).catch(() => {});
    api.quit();
  }

  // ---------- keyboard ----------

  const actions: Record<keyof Keys, () => void> = {
    hide: () => {},
    quickSwitcher: openSwitcher,
    dailyNote: openDaily,
    followLink: followOrToggle,
    back,
    forward,
    openTicket,
    openLink,
    addLink,
    timestamp: insertTimestamp,
    backlinks: () => {
      backlinksOpen = !backlinksOpen;
      if (!backlinksOpen) focusEditor();
    },
    archive: toggleArchive,
    help: () => (overlay = { type: "help" }),
    quit,
    rename: () => forTarget((h) => startRename(h ?? meta)),
    openSettings: () => toggleSettings(),
    setStatus: () => forTarget((h) => (h ? pickStatus(h.path, h.status, h.key ?? h.title) : pickStatus())),
    setTags: () =>
      forTarget((h) => {
        if (h?.kind === "daily") toast("Daily notes don't have tags");
        else if (h) pickTags(h.path, h.tags, h.key ?? h.title);
        else pickTags();
      }),
    notesView: () => openNotesView(),
    pinNote: () => togglePin(),
    toggleSidebar: () => setSideExpanded(!sideExpanded),
    switchVault: pickVault,
    focusSidebar: () => {
      if (sidebarHasFocus()) focusMain();
      else requestAnimationFrame(() => sidebar?.focus());
    },
  };

  /** Actions that only make sense with a note in the editor. */
  const EDITOR_ONLY = new Set<keyof Keys>([
    "followLink",
    "openTicket",
    "openLink",
    "addLink",
    "timestamp",
    "backlinks",
    "archive",
  ]);

  function onKeydown(e: KeyboardEvent) {
    if (e.isComposing || !settings || keyCapture.active) return;
    if (e.ctrlKey && e.shiftKey && !e.altKey && (e.key === "V" || e.key === "v" || e.code === "KeyV")) {
      // Paste as typed; the browser still performs the paste.
      rawPaste = true;
      setTimeout(() => (rawPaste = false), 1000);
      return;
    }
    const is = (a: keyof Keys) => matches(e, keySpecs[a] ?? null);
    const stop = () => {
      e.preventDefault();
      e.stopPropagation();
    };

    if (chordPending && !["Control", "Shift", "Alt", "Meta"].includes(e.key)) {
      chordPending = false;
      clearTimeout(chordTimer);
      if (matchesChordTail(e, pinChord)) {
        stop();
        togglePin();
        return;
      }
    }
    if (!overlay && pinChord.length === 2 && matches(e, pinChord[0])) {
      stop();
      chordPending = true;
      clearTimeout(chordTimer);
      chordTimer = setTimeout(() => (chordPending = false), 1500);
      return;
    }

    if (overlay) {
      if (e.key === "Tab" && ["noteActions", "file"].includes(overlay.type)) {
        const dialog = document.querySelector<HTMLElement>('[role="dialog"][aria-modal="true"]');
        const controls = dialog ? [...dialog.querySelectorAll<HTMLElement>('button:not(:disabled), input:not(:disabled), select:not(:disabled)')] : [];
        const first = controls[0], last = controls[controls.length - 1];
        if (first && last && (e.shiftKey ? document.activeElement === first : document.activeElement === last)) {
          stop();
          (e.shiftKey ? last : first).focus();
        }
      }
      if (is("hide") || (overlay.type === "help" && is("help")) || (overlay.type === "switcher" && is("quickSwitcher"))) {
        stop();
        if (!e.repeat || is("hide")) closeOverlay();
      }
      return;
    }
    if (is("hide")) {
      // Let the link picker close itself first.
      if (view && completionStatus(view.state) !== null) return;
      stop();
      if (sidebarHasFocus()) {
        if (sideOverlay) sideExpanded = false;
        focusMain();
        return;
      }
      if (screen === "settings") {
        toggleSettings(false);
        return;
      }
      if (screen === "notes") {
        leaveNotesView();
        return;
      }
      if (backlinksOpen) {
        backlinksOpen = false;
        focusEditor();
        return;
      }
      onBlur();
      api.hideWindow();
      return;
    }
    for (const a of Object.keys(actions) as (keyof Keys)[]) {
      if (a !== "hide" && is(a)) {
        if (screen !== "editor" && EDITOR_ONLY.has(a)) return;
        stop();
        if (a === "quickSwitcher" && e.repeat) return;
        actions[a]();
        return;
      }
    }
    if (e.ctrlKey && !e.altKey && !e.shiftKey && e.key.toLowerCase() === "s") {
      e.preventDefault();
      flush();
      return;
    }
    if (isBrowserAccelerator(e)) e.preventDefault();
  }

  /** WebView2 keys that would reload, print, search or navigate the page itself. */
  function isBrowserAccelerator(e: KeyboardEvent): boolean {
    const k = e.key.toLowerCase();
    if (["f3", "f5", "f7", "browserback", "browserforward", "browserrefresh", "browsersearch"].includes(k)) return true;
    if (e.altKey && !e.ctrlKey && (k === "home" || k === "arrowleft" || k === "arrowright")) return true;
    return e.ctrlKey && !e.altKey && ["r", "p", "f", "g", "h", "j", "n", "o", "t", "u", "w", "e", "l"].includes(k);
  }

  function onContextMenu(e: MouseEvent) {
    // Keep cut/copy/paste in text areas; hide the browser menu (reload, inspect…) elsewhere.
    const t = e.target as HTMLElement;
    if (!t.closest(".cm-editor, input, textarea")) e.preventDefault();
  }

  /** CI self-test (STASH_SMOKE_TEST=1, see smoke.rs): pin this note like Ctrl+E P does. */
  async function runSmokeTest() {
    await api.smokeStep("main-loaded");
    if (!currentPath) return api.smokeStep("FAILED: no note open");
    await api.pinToggle(currentPath).catch((e) => api.smokeStep(`FAILED: pin_toggle: ${e}`));
    await api.smokeStep("pin-requested");
  }

  // ---------- startup ----------

  onMount(() => {
    extensions = editorExtensions({
      onChange: onDocChange,
      follow: followAt,
      path: () => currentPath,
      openNote: navigate,
      linkSearch: (q) => api.search(q, false, currentPath, 30),
      vault: () => meta?.vault ?? null,
      vaultName: (id) => settings?.vaults.find((v) => v.id === id)?.name ?? id,
      paste: smartPaste,
      imageError: (message) => toaster.show(message, "error"),
    });
    view = new EditorView({ parent: editorEl, state: EditorState.create({ doc: "", extensions }) });
    darkQuery.addEventListener("change", applyDark);
    // Lets UI tests drive the editor in `npm run dev`; stripped from builds.
    if (import.meta.env.DEV) (window as unknown as { __view: () => EditorView }).__view = () => view;
    propertiesHooks.current = {
      statusView: (st) => statusInfo(st, settings?.statuses ?? []),
      onStatus: () => pickStatus(),
      onTags: () => pickTags(),
      onTag: showTag,
      tagColor: (t) => settings?.tagColors[t] ?? settings?.tagColors[t.split("/")[0]] ?? null,
      onTicket: openTicket,
      onLink: openLink,
      ticket: () => (ticketKey ? systemForKey(noteVault(), ticketKey) : null),
      linkTypes: () => noteVault()?.links ?? [],
    };
    window.addEventListener("keydown", onKeydown, true);
    window.addEventListener("contextmenu", onContextMenu);
    api.pinnedPaths().then((p) => (pinnedPaths = p), () => {});

    const unlisten: Promise<() => void>[] = [
      on<null>("summoned", onSummoned),
      on<string[]>("vault-changed", onVaultChanged),
      on<{ path: string; dirty: boolean; meta: NoteMeta | null }>("buffer-state", onBufferState),
      on<{ path: string; deleted: boolean }>("buffer-conflict", onBufferConflict),
      on<{ from: string; to: string }>("note-moved", (m) => moved(m.from, m.to)),
      on<string>("note-deleted", deleted),
      on<NoteMeta>("note-created", noteCreated),
      // `::notes#tag` (from a pinned window's tag chip) opens the Notes view on that tag.
      on<string>("open-note", (p) => (p.startsWith(`${NOTES}#`) ? showTag(p.slice(NOTES.length + 1)) : navigate(p))),
      on<string[]>("pins-changed", (p) => (pinnedPaths = p)),
      on<null>("open-settings", () => toggleSettings(true)),
      on<null>("index-ready", () => {
        indexing = false;
        refreshSide();
        listVersion++;
      }),
      on<null>("open-daily", openDaily),
      on<string>("switch-vault", (id) => id !== currentVault && switchVault(id)),
      on<null>("quit-requested", quit),
      on<string>("toast", (t) => toast(t, undefined, "warn")),
      on<Settings>("settings-changed", (s) => {
        const folders = (x: Settings) => x.vaults.map((v) => `${v.id}=${v.path}`).join("\n");
        if (settings && folders(s) !== folders(settings)) {
          indexing = true;
          api.getVaultPaths().then((p) => (vaultPaths = p), () => {});
        }
        // The current vault was removed: go to the first one.
        if (!s.vaults.some((v) => v.id === currentVault) && s.vaults.length) switchVault(s.vaults[0].id);
        const fromScreen = screen === "settings";
        applySettings(s);
        if (!fromScreen) toast("Settings reloaded");
      }),
      onWindowFocus((focused) => {
        if (!focused) onBlur();
      }),
    ];

    (async () => {
      try {
        const s = await api.startup();
        applySettings(s.settings);
        if (typeof s.ui.sidebarExpanded === "boolean") sideExpanded = s.ui.sidebarExpanded;
        if (typeof s.ui.sidebarWidth === "number") sideWidth = Math.max(180, Math.min(360, s.ui.sidebarWidth));
        notesState = {
          query: "",
          sort: s.settings.notesView.sort,
          group: s.settings.notesView.groupBy,
          showDaily: s.settings.notesView.showDaily,
          showPutAway: s.settings.notesView.showClosed,
        };
        vaultPaths = s.vaultPaths;
        currentVault = s.currentVault;
        settingsPath = s.settingsPath;
        s.warnings.forEach((w) => toast(w, undefined, "warn"));
        let path = s.lastPath;
        const today = isoDate();
        const rel = path ? path.slice(path.indexOf(":") + 1) : "";
        const staleDaily = rel.startsWith("daily/") && rel < `daily/${today}.md`;
        if (!path || s.settings.onSummon === "daily" || staleDaily) {
          path = await api.dailyNote(today, currentVault);
        }
        const entry = hist.push(path);
        if (path === s.lastPath && s.lastAnchor !== null) entry.anchor = s.lastAnchor;
        try {
          await show(path, entry, entry.anchor >= 0);
        } catch {
          hist.entries = [];
          hist.index = -1;
          await openDaily();
        }
        refreshSide();
        if (await api.smokeEnabled()) runSmokeTest();
      } catch (e) {
        fail(e);
      }
    })();

    return () => {
      window.removeEventListener("keydown", onKeydown, true);
      window.removeEventListener("contextmenu", onContextMenu);
      darkQuery.removeEventListener("change", applyDark);
      unlisten.forEach((p) => p.then((f) => f()));
      view.destroy();
    };
  });
</script>

<svelte:window bind:innerWidth={winWidth} />

<main class="app">
  {#if settings}
    <Sidebar
      bind:this={sidebar}
      vaults={settings.vaults}
      {currentVault}
      onVault={pickVault}
      active={side.active}
      more={side.more}
      recent={side.recent}
      current={currentPath}
      open={screen === "editor" ? meta : null}
      {screen}
      expanded={sideExpanded}
      width={sideWidth}
      overlay={sideOverlay}
      statuses={settings.statuses}
      keys={settings.keys}
      onToggle={() => setSideExpanded(!sideExpanded)}
      onResize={setSideWidth}
      onToday={openDaily}
      onNotes={(q) => openNotesView(q)}
      onSettings={() => toggleSettings()}
      onOpen={(p) => navigate(p)}
      onActions={noteActions}
      onCurrentActions={() => noteActions()}
      onLeave={focusMain}
    />
  {/if}
  <div class="content">
  <div class="pane" class:hidden={screen !== "editor"}>
  {#if settings}
    <Header
      {meta}
      {dirty}
      canBack={nav.canBack}
      canForward={nav.canForward}
      ticket={ticketKey ? systemForKey(noteVault(), ticketKey) : null}
      linkTypes={noteVault()?.links ?? []}
      keys={settings.keys}
      statuses={settings.statuses}
      onStatus={() => pickStatus()}
      onTags={() => pickTags()}
      onBack={back}
      onForward={forward}
      onTicket={openTicket}
      onLink={openLink}
      onAddLink={addLink}
      onSwitcher={openSwitcher}
      pinned={!!currentPath && pinnedPaths.includes(currentPath)}
      onPin={togglePin}
      onArchive={toggleArchive}
      onActions={() => noteActions()}
      onImage={() => pickImages(view, { path: () => currentPath, imageError: (message) => toaster.show(message, "error") })}
    />
  {/if}

  {#if conflict}
    <div class="conflict" role="alert">
      {#if conflict.deleted}
        <span>This note was deleted or moved outside the app.</span>
        <button onclick={() => resolveConflict(false)}>Keep editing (recreates it)</button>
        <button onclick={back}>Go back</button>
      {:else}
        <span>This note changed on disk while you had unsaved edits.</span>
        <button onclick={() => resolveConflict(true)}>Reload from disk</button>
        <button onclick={() => resolveConflict(false)}>Keep mine</button>
      {/if}
    </div>
  {/if}

  <div class="editor" bind:this={editorEl}></div>

  {#if settings}
    <Backlinks
      items={backlinkItems}
      expanded={backlinksOpen}
      keys={settings.keys}
      statuses={settings.statuses}
      onOpen={(p) => {
        backlinksOpen = false;
        navigate(p);
      }}
      onToggle={() => actions.backlinks()}
    />
  {/if}
  </div>

  {#if screen === "settings" && settings}
    <SettingsView
      {settings}
      {settingsPath}
      {indexing}
      sidebarExpanded={sideExpanded}
      onSidebar={setSideExpanded}
      {currentVault}
      {vaultPaths}
    />
  {/if}

  {#if screen === "notes" && settings}
    <NotesView
      bind:this={notesView}
      bind:view={notesState}
      statuses={settings.statuses}
      tagColors={settings.tagColors}
      vault={currentVault}
      vaults={settings.vaults}
      keys={settings.keys}
      version={listVersion}
      isTicket={startsWithTicket}
      {pasteKey}
      onOpen={(p) => navigate(p)}
      onCreate={async (q) => {
        try {
          navigate(await api.createNote(q, currentVault));
        } catch (e) {
          fail(e);
        }
      }}
    />
  {/if}
  </div>

  {#if overlay?.type === "noteActions"}
    {@const target = overlay.target}
    <NoteActions title={target.title} canRename={target.kind === "note"} canStatus={target.kind !== "daily"}
      onRename={() => startRename(target)}
      onMove={() => (overlay = { type: "file", mode: "move", target })}
      onDelete={() => (overlay = { type: "file", mode: "delete", target })}
      onStatus={() => pickStatus(target.path, target.status, target.key ?? target.title)} onClose={closeOverlay} />
  {:else if overlay?.type === "file"}
    {@const o = overlay}
    <FileDialog mode={o.mode} title={o.target.title} path={o.target.path} vaults={settings?.vaults ?? []}
      onMove={(vault) => moveNote(o.target, vault)} onArchive={() => archiveInstead(o.target)} onDelete={() => deleteNote(o.target)} onClose={closeOverlay} />
  {:else if overlay?.type === "switcher"}
    <Switcher
      mode={overlay.mode}
      current={currentPath}
      isTicket={startsWithTicket}
      {pasteKey}
      statuses={settings?.statuses ?? []}
      tagColors={settings?.tagColors ?? {}}
      vaults={settings?.vaults ?? []}
      {currentVault}
      onChoose={switcherChoose}
      onCreate={switcherCreate}
      onClose={closeOverlay}
    />
  {:else if overlay?.type === "help" && settings}
    <Help keys={settings.keys} hotkey={settings.hotkey} switcherHotkey={settings.switcherHotkey} {settingsPath} vaults={settings.vaults} {vaultPaths} onClose={closeOverlay} />
  {:else if overlay?.type === "prompt"}
    <Prompt title={overlay.title} value={overlay.value} placeholder={overlay.placeholder} onSubmit={overlay.submit} onClose={closeOverlay} />
  {:else if overlay?.type === "status" && settings}
    {@const o = overlay}
    <StatusPicker statuses={settings.statuses} current={o.current} title={o.title} onChoose={(st) => applyStatus(o.path, st)} onClose={closeOverlay} />
  {:else if overlay?.type === "tags" && settings}
    {@const o = overlay}
    <TagPicker
      title={o.title}
      tags={o.tags}
      all={o.all}
      colors={settings.tagColors}
      onChange={(t) => applyTags(o.path, t)}
      onClose={closeOverlay}
    />
  {:else if overlay?.type === "picker"}
    <Picker title={overlay.title} items={overlay.items} onChoose={overlay.choose} onClose={closeOverlay} />
  {/if}

  {#if chordPending && settings}
    <div class="chord">
      <kbd>{formatKey(settings.keys.pinNote.split(/\s+/)[0])}</kbd> then
      <kbd>{settings.keys.pinNote.split(/\s+/)[1] ?? "P"}</kbd> pin or unpin this note
    </div>
  {/if}

  <Toasts />
</main>

<style>
  .app {
    position: relative;
    display: flex;
    height: 100vh;
  }
  .content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .pane {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .pane.hidden {
    display: none;
  }
  .editor {
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .editor :global(.cm-editor) {
    flex: 1;
    min-width: 0;
  }
  .conflict {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 12px;
    background: var(--warn-bg);
    color: var(--fg);
    border-bottom: 1px solid var(--border);
    font-size: 13px;
  }
  .conflict button {
    font: inherit;
    font-size: 12.5px;
    color: var(--fg);
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 2px 8px;
    cursor: pointer;
  }
  .chord {
    position: fixed;
    left: 50%;
    bottom: 34px;
    transform: translateX(-50%);
    padding: 5px 12px;
    font-size: 12.5px;
    background: var(--toast-bg);
    color: var(--toast-fg);
    border-radius: 7px;
    box-shadow: var(--shadow);
    z-index: 41;
  }
  .chord kbd {
    font: inherit;
    font-weight: 600;
  }
</style>
