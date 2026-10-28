<script lang="ts">
  import { onMount } from "svelte";
  import Toasts from "./components/Toasts.svelte";
  import { toaster, type ToastAction, type ToastKind } from "./lib/toasts.svelte";
  import { EditorState, type Extension } from "@codemirror/state";
  import { EditorView } from "@codemirror/view";
  import { completionStatus } from "@codemirror/autocomplete";
  import { undo } from "@codemirror/commands";
  import Header from "./components/Header.svelte";
  import Switcher from "./components/Switcher.svelte";
  import Backlinks from "./components/Backlinks.svelte";
  import LinksPanel from "./components/LinksPanel.svelte";
  import LinkCreate from "./components/LinkCreate.svelte";
  import { dim, pop } from "./lib/motion";
  import { dismissOnBackdrop } from "./lib/popup";
  import Help from "./components/Help.svelte";
  import Prompt from "./components/Prompt.svelte";
  import Menu, { type MenuEntry } from "./components/Menu.svelte";
  import FileDialog from "./components/FileDialog.svelte";
  import TidyDialog from "./components/TidyDialog.svelte";
  import Picker from "./components/Picker.svelte";
  import StatusPicker from "./components/StatusPicker.svelte";
  import TagPicker from "./components/TagPicker.svelte";
  import NotesView, { type NotesViewState } from "./components/NotesView.svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import SettingsView from "./components/SettingsView.svelte";
  import { applyAppearance } from "./lib/appearance";
  import { api, on, onWindowFocus, type Keys, type LinkSource, type MetaEdit, type NoteLink, type NoteMeta, type Opened, type SearchHit, type Settings, type StatusResult, type TagCount, type TidyPreview } from "./lib/api";
  import { NoteSync } from "./lib/buffer";
  import { pickImages } from "./lib/images";
  import type { Anchor } from "./lib/popup";
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
  import { daily, noteName } from "./lib/daily.svelte";
  import { isKey, isLinkInput, isUrl, keyRegex } from "./lib/links";
  import { statusInfo } from "./lib/status";
  import { History, type Entry } from "./lib/history";
  import { formatKey, keyCapture, matches, matchesChordTail, parseChord, parseKey, type KeySpec } from "./lib/keys";
  import {
    checkboxToggle,
    isoDate,
    refAt,
    ticketRegex,
    timestamp,
    todoCursor,
  } from "./lib/text";

  /** A note an action applies to: the open note, or a row in a list. */
  type NoteTarget = Pick<NoteMeta, "path" | "title" | "kind" | "status" | "key" | "tags" | "archived">;
  type Overlay =
    | { type: "menu"; label: string; items: MenuEntry[]; anchor: Anchor }
    | { type: "file"; mode: "delete"; target: NoteTarget }
    | { type: "tidy"; vault: string; preview: TidyPreview }
    | { type: "switcher"; mode: "open" | "link"; sel?: { from: number; to: number; text: string } }
    | { type: "help" }
    | { type: "prompt"; title: string; value: string; placeholder?: string; submit: (v: string) => void; anchor?: Anchor | null }
    | { type: "picker"; title: string; items: { label: string; detail?: string; color?: string }[]; choose: (i: number) => void; anchor?: Anchor | null }
    | { type: "linkCreate"; input: string; vault: string }
    | { type: "status"; path: string; current: string | null; title: string; anchor?: Anchor | null }
    | { type: "tags"; path: string; tags: string[]; all: TagCount[]; title: string; anchor?: Anchor | null };


  let settings = $state<Settings | null>(null);
  /** Each vault's folder, by id. */
  let vaultPaths = $state<Record<string, string>>({});
  /** The vault the sidebar, Notes view, Today and new notes belong to. */
  let currentVault = $state("");
  let settingsPath = $state("");
  let meta = $state<NoteMeta | null>(null);
  let dirty = $state(false);
  /** The open note's links, as its text is now (see refreshLinks). */
  let noteLinks = $state<NoteLink[]>([]);
  /** The Links panel beside the editor, and which links it shows. */
  let linksOpen = $state(false);
  let linksScope = $state<"note" | "all">("note");
  let linksPanel = $state<LinksPanel>();
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
  /** Vaults still laid out the pre-0.8 way (offered a tidy into notes/). */
  let untidy = $state<string[]>([]);
  /** Vaults whose tidy was put off with "Not now" (not offered at startup again). */
  let tidyDismissed: string[] = [];
  /** The notes in `pinnedPaths`, for the sidebar's Pinned group. */
  let pinnedHits = $state<SearchHit[]>([]);
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

  /** A key a link rule reads (`PROJ-123`). */
  function isTicketKey(q: string) {
    return isKey(settings, q);
  }

  /** Highlight and follow the keys link rules read (and the note's vault's 0.6 ticket keys). */
  function applyTickets() {
    const re = keyRegex(settings, meta?.vault ?? currentVault);
    tickets = re ? ticketRegex(re.source) : /(?!)/g;
    decoConfig.tickets = tickets;
    if (view) refreshDecorations(view);
  }

  /** `PROJ-1234 Do this thing` or `https://… Title`: creates a note from the link. */
  function isLink(q: string) {
    return isLinkInput(settings, q);
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
    daily.format = s.dailyDateFormat;
    applyDark();
    if (view) {
      setLiveFormatting(view, s.liveFormatting);
      setSpellCheck(view, s.spellCheck);
      refreshLinks(true);
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

  let linksTimer: ReturnType<typeof setTimeout> | undefined;
  /** Re-read the open note's links from its text (shortly after typing stops, or `now`). */
  function refreshLinks(now = false) {
    clearTimeout(linksTimer);
    const run = async () => {
      const path = currentPath;
      if (!path || !view) return;
      const text = docText();
      try {
        const links = await api.noteLinks(path, text);
        if (path !== currentPath || text !== docText()) return;
        noteLinks = links;
        refreshDecorations(view);
      } catch {
        // keep the previous list
      }
    };
    if (now) run();
    else linksTimer = setTimeout(run, 150);
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
    refreshLinks();
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
    noteLinks = [];
    // The window follows the note into its vault (Rust records it too).
    if (data.meta.vault && data.meta.vault !== currentVault) enterVault(data.meta.vault);
    applyTickets();
    serverDirty = data.dirty;
    conflict = data.deleted ? { deleted: true } : null;
    decoConfig.todoAges = new Map();
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
    refreshLinks(true);
    refreshAges();
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

  /** Switch vault: a list from the keyboard, a dropdown from the sidebar's vault row. */
  function pickVault(anchor: Anchor | null = null) {
    const vs = settings?.vaults ?? [];
    if (vs.length < 2) {
      toast("Add another vault in Settings → Vaults to switch between them");
      return;
    }
    if (anchor) {
      overlay = {
        type: "menu",
        label: "Switch vault",
        anchor,
        items: [
          ...vs.map((v) => ({
            label: v.name,
            badge: { text: v.name.slice(0, 1).toUpperCase(), color: v.color },
            checked: v.id === currentVault,
            run: () => v.id !== currentVault && switchVault(v.id),
          })),
          "-",
          { label: "Open vault folder", icon: "folder", run: () => api.openVaultFolder(currentVault).catch(fail) },
          { label: "Manage vaults…", icon: "settings", run: () => toggleSettings(true) },
        ],
      };
      return;
    }
    overlay = {
      type: "picker",
      title: "Switch vault",
      anchor,
      items: vs.map((v) => ({ label: v.name, detail: v.id === currentVault ? "current" : `in:${v.id}`, color: v.color })),
      choose: (i) => {
        closeOverlay();
        if (vs[i].id !== currentVault) switchVault(vs[i].id);
      },
    };
  }

  async function refreshSidebar() {
    try {
      [side, pinnedHits] = await Promise.all([api.sidebar(currentVault), api.noteHits(pinnedPaths)]);
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

  /** How long the open daily note's carried todos have been open (the `5d` badges). */
  async function refreshAges() {
    const path = currentPath;
    if (!path || meta?.kind !== "daily") return;
    const ages = await api.todoAges(path).catch(() => []);
    if (path !== currentPath) return;
    decoConfig.todoAges = new Map(ages.map((a) => [a.text, a]));
    refreshDecorations(view);
  }

  /** The daily note before (or after) the open one; from another note, the latest one. */
  async function stepDaily(forward: boolean) {
    const from = screen === "editor" ? currentPath : null;
    try {
      const path = from ? await api.adjacentDaily(from, isoDate(), forward) : await api.dailyNote(isoDate(), currentVault);
      if (path) navigate(path);
      else toast(forward ? "No later daily note" : "No earlier daily note");
    } catch (e) {
      fail(e);
    }
  }

  /** `+ text` in the switcher: a todo for today's daily note. */
  async function addTodo(text: string, vault: string | null) {
    closeOverlay();
    try {
      await flush();
      done(await api.addTodo(text, isoDate(), vault ?? currentVault));
      refreshAges();
    } catch (e) {
      fail(e);
    }
  }

  /** Pull the previous daily note's open todos into the open daily note. */
  async function carryTodos() {
    if (screen !== "editor" || !currentPath || meta?.kind !== "daily") {
      toast("Open a daily note to bring todos into it");
      return;
    }
    try {
      await flush();
      const message = await api.carryTodos(currentPath);
      if (/^(Moved|Copied)/.test(message)) done(message);
      else toast(message);
      refreshAges();
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
    else if (ref.kind === "ticket") followKey(ref.target);
    else api.resolveLink(ref.target, currentPath).then((p) => navigate(p), fail);
    return true;
  }

  /** A bare key: its note, if there is one, else offer to create it from the key's link. */
  async function followKey(key: string) {
    try {
      const p = await api.linkPreview(key, meta?.vault ?? currentVault);
      if (p.existing.length === 1) return navigate(p.existing[0].path);
      if (!p.url && !p.choices.length) return navigate(await api.resolveLink(key, currentPath));
      overlay = { type: "linkCreate", input: key, vault: currentVault };
    } catch (e) {
      fail(e);
    }
  }

  /** A note was created from a link: open it, or link the selection to it. */
  function createdFromLink(path: string, sel?: { from: number; to: number; text: string }) {
    closeOverlay();
    if (sel) insertLink(path.slice(path.lastIndexOf("/") + 1).replace(/\.md$/, ""), sel);
    else navigate(path);
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

  /** The open note's links now (not waiting for the next refresh). */
  async function currentLinks(): Promise<NoteLink[]> {
    if (!currentPath) return [];
    try {
      noteLinks = await api.noteLinks(currentPath, docText());
    } catch {
      // the last list will do
    }
    return noteLinks;
  }

  function pickLink(title: string, links: NoteLink[]) {
    overlay = {
      type: "picker",
      title,
      items: links.map((l) => ({ label: l.primary ? `${l.label} · primary` : l.label, detail: l.url })),
      choose: (i) => {
        closeOverlay();
        openExternal(links[i].url);
      },
    };
  }

  /** Open the link this note is about: its primary link (a 0.6 ticket's ticket), else its only link, else a choice. */
  async function openPrimary() {
    if (!meta || !settings) return;
    const links = await currentLinks();
    const primary = links.find((l) => l.primary);
    if (primary) openExternal(primary.url);
    else if (links.length === 1) openExternal(links[0].url);
    else if (links.length) pickLink("Open which link?", links);
    else toast(`No links in this note yet — paste one, or add one with ${formatKey(settings.keys.addLink)}`);
  }

  /** Open one of the note's links (a picker when there are several). */
  async function openLink() {
    if (!meta || !settings) return;
    const links = await currentLinks();
    if (links.length === 1) openExternal(links[0].url);
    else if (links.length) pickLink("Open link", links);
    else toast(`No links in this note yet — paste one, or add one with ${formatKey(settings.keys.addLink)}`);
  }

  /** Change the note's link metadata through the editor, so it joins undo and every window's copy. */
  async function editLinks(edit: MetaEdit): Promise<boolean> {
    const path = currentPath;
    if (!path) return false;
    const change = await api.linkMetaEdit(path, docText(), edit);
    if (!change || path !== currentPath) return false;
    view.dispatch({ changes: change, userEvent: "input.links" });
    await flush();
    refreshLinks(true);
    return true;
  }

  /** Ask for any link and attach it to the note (front matter `links`), beside `anchor` if given. */
  async function addLink(anchor: Anchor | null = null) {
    if (!settings || !currentPath) return;
    const clip = (await api.readClipboard().catch(() => "")).trim();
    overlay = {
      type: "prompt",
      title: "Add a link to this note",
      placeholder: "https://…",
      value: isUrl(clip) ? clip : "",
      anchor,
      submit: async (value) => {
        closeOverlay();
        const url = value.trim();
        if (!isUrl(url)) return toast("That isn't a link — it needs to start with https:// or http://", undefined, "warn");
        try {
          if (await editLinks({ op: "add", url })) done("Link added");
          else toast("This note already has that link");
        } catch (e) {
          fail(e);
        }
      },
    };
  }

  /** The properties row's + Add: tags or a link. */
  function addMenu(anchor: Anchor) {
    if (!settings || !meta || meta.kind === "daily") return;
    overlay = {
      type: "menu",
      label: "Add to note",
      anchor,
      items: [
        { label: "Tags…", icon: "tag", hint: formatKey(settings.keys.setTags), run: () => pickTags(undefined, undefined, undefined, anchor) },
        { label: "Link…", icon: "link", hint: formatKey(settings.keys.addLink), run: () => addLink(anchor) },
      ],
    };
  }

  // ---------- the Links panel ----------

  /** Show the panel on `scope`; again (from the header or its shortcut) hides it. */
  function toggleLinks(scope: "note" | "all", toggle = true) {
    if (toggle && linksOpen && linksScope === scope) return closeLinks();
    linksScope = scope;
    linksOpen = true;
    if (scope === "note") refreshLinks(true);
    requestAnimationFrame(() => linksPanel?.focusFirst());
  }

  function closeLinks() {
    linksOpen = false;
    requestAnimationFrame(() => !overlay && focusMain());
  }

  /** Put the caret on a link where it appears in the note. */
  function goToSource(s: LinkSource) {
    if (!currentPath) return;
    screen = "editor";
    if (sideOverlay) linksOpen = false;
    const len = view.state.doc.length;
    view.dispatch({ selection: { anchor: Math.min(s.from, len), head: Math.min(s.to, len) }, effects: EditorView.scrollIntoView(Math.min(s.from, len), { y: "center" }) });
    view.focus();
  }

  async function copyLink(url: string) {
    try {
      await api.writeClipboard(url);
      done("Link copied");
    } catch (e) {
      fail(e);
    }
  }

  /** Change an attached (front matter only) link. */
  function editAttached(link: NoteLink) {
    overlay = {
      type: "prompt",
      title: "Edit attached link",
      value: link.url,
      submit: async (value) => {
        closeOverlay();
        if (!isUrl(value)) return toast("That isn't a link — it needs to start with https:// or http://", undefined, "warn");
        try {
          await editLinks({ op: "replace", url: [link.url, value.trim()] });
        } catch (e) {
          fail(e);
        }
      },
    };
  }

  /** Primary link, removal and every place a link appears. */
  function linkActions(link: NoteLink) {
    const run = (edit: MetaEdit, said: string) => async () => {
      closeOverlay();
      const path = currentPath;
      try {
        if (!(await editLinks(edit))) return;
        done(said, { label: "Undo", run: () => path === currentPath && (undo(view), flush()) });
      } catch (e) {
        fail(e);
      }
    };
    const items: { label: string; detail?: string; run: () => void }[] = [];
    if (!link.primary) items.push({ label: "Make it the primary link", detail: `${formatKey(settings?.keys.openTicket ?? "")} opens it`, run: run({ op: "primary", url: link.url }, "Primary link set") });
    else if (link.sources.some((s) => s.kind === "primary")) items.push({ label: "Stop using it as the primary link", run: run({ op: "clearPrimary" }, "No primary link") });
    if (link.sources.every((s) => s.kind === "attached")) {
      items.push({ label: "Edit attached link", run: () => editAttached(link) });
      items.push({ label: "Remove from this note", run: run({ op: "remove", url: link.url }, "Link removed") });
    }
    for (const s of link.sources) {
      const kind = { body: "in the text", key: "key", attached: "attached link", primary: "primary link", legacy: "front matter", ticket: "the ticket's own key" }[s.kind];
      items.push({ label: `Go to line ${s.line}`, detail: s.field ? `${kind} · ${s.field}` : kind, run: () => (closeOverlay(), goToSource(s)) });
    }
    items.push({ label: "Copy link", detail: link.url, run: () => (closeOverlay(), copyLink(link.url)) });
    overlay = { type: "picker", title: link.label, items, choose: (i) => items[i].run() };
  }

  // ---------- paste ----------

  /** Set by Ctrl+Shift+V: the next paste goes in as typed. */
  let rawPaste = false;

  /** A single pasted link whose rule formats pastes goes in as `[label](link)`. */
  function smartPaste(text: string): boolean {
    if (rawPaste) {
      rawPaste = false;
      return false;
    }
    if (!settings?.smartPaste || !isUrl(text) || !settings.linkRules.some((r) => r.enabled && r.pasteFormat)) return false;
    const doc = view.state.doc;
    const sel = view.state.selection;
    api
      .pasteFormat(text)
      .catch(() => null)
      .then((formatted) => {
        if (view.state.doc === doc) view.dispatch({ selection: sel });
        view.dispatch(view.state.replaceSelection(formatted ?? text), { userEvent: "input.paste", scrollIntoView: true });
      });
    return true;
  }

  function insertTimestamp() {
    view.dispatch(view.state.replaceSelection(timestamp(new Date(), settings?.timestampFormat)));
    view.focus();
  }

  // ---------- tidying a vault into 0.8's layout ----------

  const vaultName = (id: string) => settings?.vaults.find((v) => v.id === id)?.name ?? id;

  async function refreshUntidy() {
    try {
      untidy = await api.tidyNeeded();
    } catch {
      // keep the last answer
    }
  }

  /** After an upgrade: offer to tidy the first vault still using the old folders. */
  async function offerTidy() {
    await refreshUntidy();
    const id = untidy.find((v) => !tidyDismissed.includes(v));
    if (!id) return;
    toaster.show(`Stash Notes now keeps every note in one notes folder. Tidy ${vaultName(id)}'s folders?`, "info", { label: "Review…", run: () => openTidy(id) }, 20000);
  }

  async function openTidy(vault: string) {
    try {
      overlay = { type: "tidy", vault, preview: await api.tidyPreview(vault) };
    } catch (e) {
      fail(e);
    }
  }

  async function runTidy(vault: string) {
    await flush();
    const r = await api.tidyVault(vault);
    closeOverlay();
    listVersion++;
    refreshSide();
    const links = r.links ? `, links updated in ${r.links === 1 ? "1 note" : `${r.links} notes`}` : "";
    done(`Tidied ${vaultName(vault)}: ${r.moved === 1 ? "1 note" : `${r.moved} notes`} moved into notes${links}`, {
      label: "Undo",
      run: async () => {
        try {
          await flush();
          await api.tidyUndo(r.undoId);
          done(`Put ${vaultName(vault)}'s folders back`);
        } catch (e) {
          fail(e);
        }
        listVersion++;
        refreshSide();
        refreshUntidy();
      },
    });
    await refreshUntidy();
  }

  function laterTidy(vault: string) {
    tidyDismissed = [...new Set([...tidyDismissed, vault])];
    api.setUi("tidyDismissed", tidyDismissed).catch(() => {});
    closeOverlay();
    toast("You can tidy it any time in Settings → Vaults");
  }

  // ---------- archive / rename ----------

  function moved(from: string, to: string) {
    hist.rename(from, to);
    if (recentCreation?.target.path === from) recentCreation.target.path = to;
    if (sync?.path === from) sync.path = to;
    if (currentPath === from) {
      currentPath = to;
      const vault = to.slice(0, to.indexOf(":"));
      if (meta) meta = { ...meta, path: to, vault };
      if (vault !== currentVault) enterVault(vault);
      applyTickets();
      refreshLinks(true);
      conflict = null;
      refreshDecorations(view);
      refreshBacklinks();
    }
    listVersion++;
    refreshSide();
  }

  /** The note menu (right-click, or …) for `target`, opened at `anchor`. */
  function openNoteMenu(target: NoteTarget | null, anchor: Anchor) {
    if (!target || !settings) return;
    const s = settings;
    const isOpen = screen === "editor" && target.path === currentPath;
    const vault = target.path.slice(0, target.path.indexOf(":"));
    const name = target.kind === "ticket" && target.key ? target.key : target.title;
    const pinned = pinnedPaths.includes(target.path);
    // The primary link is known for the open note (from its text).
    const primary = isOpen ? noteLinks.find((l) => l.primary) : undefined;
    const link = `[[${vault !== currentVault ? `${vault}:` : ""}${name}]]`;
    const items: MenuEntry[] = [];
    if (!isOpen) items.push({ label: "Open", icon: "file-text", hint: "Enter", run: () => navigate(target.path) });
    items.push({
      label: pinned ? "Unpin" : "Pin beside your work",
      icon: pinned ? "pin-off" : "pin",
      hint: s.keys.pinNote.split(/\s+/).map(formatKey).join(", "),
      run: () => pinPath(target.path),
    });
    items.push("-");
    if (target.kind !== "daily") {
      items.push({
        label: "Status",
        status: target.status ?? undefined,
        icon: "circle-dashed",
        hint: target.status ?? "none",
        numbered: true,
        items: [
          ...s.statuses.map((st) => ({ label: st.name, status: st.name, checked: st.name === target.status, run: () => applyStatus(target.path, st.name) })),
          "-",
          { label: "No status", icon: "x", checked: !target.status, run: () => applyStatus(target.path, null) },
        ],
      });
      items.push({
        label: "Tags…",
        icon: target.tags.length > 1 ? "tags" : "tag",
        hint: target.tags.map((t) => `#${t}`).join(" ") || formatKey(s.keys.setTags),
        run: () => pickTags(target.path, target.tags, name, anchor),
      });
    }
    if (primary) items.push({ label: `Open ${primary.label}`, icon: primary.rule?.icon ?? "external-link", hint: formatKey(s.keys.openTicket), run: openPrimary });
    if (isOpen) items.push({ label: "Links…", icon: "link", hint: formatKey(s.keys.linksPanel), run: () => toggleLinks("note", false) });
    items.push({ label: "Copy link", icon: "copy", hint: link, run: () => copyLink(link) });
    if (isOpen) {
      items.push({
        label: "Insert image…",
        icon: "image-plus",
        run: () => pickImages(view, { path: () => currentPath, imageError: (message) => toaster.show(message, "error") }),
      });
    }
    items.push("-");
    items.push({
      label: "Rename…",
      icon: "pencil",
      hint: formatKey(s.keys.rename),
      disabled: target.kind !== "note",
      title: target.kind === "daily" ? "Daily notes are named by their date" : target.kind === "ticket" ? "Ticket notes are named by their key" : undefined,
      run: () => startRename(target),
    });
    const others = s.vaults.filter((v) => v.id !== vault);
    if (others.length) {
      items.push({
        label: "Move to",
        icon: "folder-input",
        items: others.map((v) => ({ label: v.name, badge: { text: v.name.slice(0, 1).toUpperCase(), color: v.color }, run: () => moveNote(target, v.id) })),
      });
    }
    items.push({
      label: target.archived ? "Unarchive" : "Archive",
      icon: target.archived ? "archive-restore" : "archive",
      hint: formatKey(s.keys.archive),
      run: () => archivePath(target.path).catch(fail),
    });
    items.push("-");
    items.push({ label: "Delete…", icon: "trash", danger: true, run: () => (overlay = { type: "file", mode: "delete", target }) });
    overlay = { type: "menu", label: `Note menu for ${name}`, items, anchor };
  }

  async function moveNote(target: NoteTarget, vault: string) {
    try {
      await flush();
      const to = await api.moveNote(target.path, vault);
      moved(target.path, to);
      const name = settings?.vaults.find((v) => v.id === vault)?.name ?? vault;
      done(`Moved to ${name}`);
    } catch (e) {
      fail(e);
    }
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
    creation.toast = toaster.show(`Created ${noteName(target)}`, "done", { label: "Delete", run: async () => {
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
      noteLinks = [];
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
    done(`Deleted ${noteName(target)}`, { label: "Undo", run: async () => {
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
    const r = await api.archiveNote(from);
    afterArchive(from, r);
    clearCreation(r.path);
    done(r.meta.archived ? "Archived" : "Unarchived", {
      label: "Undo",
      run: async () => {
        try {
          await flush();
          afterArchive(r.path, await api.archiveNote(r.path));
        } catch (e) {
          fail(e);
        }
      },
    });
  }

  /** A note was archived or unarchived (in place, or out of a pre-0.8 archive/ folder). */
  function afterArchive(from: string, r: StatusResult) {
    if (r.path !== from) moved(from, r.path);
    // An open note receives the edit through its buffer.
    if (currentPath === r.path) meta = r.meta;
    listVersion++;
    refreshSide();
  }

  async function archiveInstead(target: NoteTarget) {
    if (!target.archived) await archivePath(target.path);
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
  function pickStatus(path = currentPath, current = meta?.status ?? null, title = meta?.title ?? "", anchor: Anchor | null = null) {
    if (!path) return;
    if (path === currentPath && meta?.kind === "daily") {
      toast("Daily notes don't have a status");
      return;
    }
    overlay = { type: "status", path, current, title: `Status for ${title}`, anchor };
  }

  async function applyStatus(path: string, status: string | null) {
    closeOverlay();
    try {
      if (path === currentPath) await flush();
      const r = await api.setStatus(path, status);
      // An open note receives the edit through its buffer.
      if (path === currentPath) meta = r.meta;
      if (r.path !== path) moved(path, r.path);
      if (status === "closed" && r.meta.archived) done("Closed — archived");
      listVersion++;
      refreshSide();
    } catch (e) {
      fail(e);
    }
  }

  // ---------- tags ----------

  /** Open the tag picker for `path` (default: the current note). */
  async function pickTags(path = currentPath, tags = meta?.tags ?? [], title = meta?.title ?? "", anchor: Anchor | null = null) {
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
    overlay = { type: "tags", path, tags, all, title: `Tags for ${title}`, anchor };
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
    if (path) await pinPath(path);
  }

  async function pinPath(path: string) {
    try {
      await flush();
      const pinned = await api.pinToggle(path);
      done(pinned ? "Pinned — it stays on top beside your work" : "Unpinned");
    } catch (e) {
      fail(e);
    }
  }

  // ---------- overlays ----------

  /** Ctrl+K: search, or link the selected text. `search`: always search (the sidebar's box). */
  function openSwitcher(search = false) {
    if (overlay?.type === "switcher") {
      closeOverlay();
      return;
    }
    const sel = view.state.selection.main;
    if (!sel.empty && !search && screen === "editor") {
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
        insertLink(isTicketKey(title.split(/\s+/)[0] ?? "") ? title.split(/\s+/)[0].toUpperCase() : title, o.sel);
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
    if (ev.meta) meta = ev.meta;
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
    quickSwitcher: () => openSwitcher(),
    dailyNote: openDaily,
    carryTodos,
    prevDaily: () => stepDaily(false),
    nextDaily: () => stepDaily(true),
    followLink: followOrToggle,
    back,
    forward,
    openTicket: openPrimary,
    openLink,
    addLink: () => addLink(),
    linksPanel: () => toggleLinks("note"),
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
    switchVault: () => pickVault(),
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
      if (e.key === "Tab" && (overlay.type === "file" || overlay.type === "tidy")) {
        const dialog = document.querySelector<HTMLElement>('[role="dialog"][aria-modal="true"]');
        const controls = dialog ? [...dialog.querySelectorAll<HTMLElement>('button:not(:disabled), input:not(:disabled), select:not(:disabled)')] : [];
        const first = controls[0], last = controls[controls.length - 1];
        if (first && last && (e.shiftKey ? document.activeElement === first : document.activeElement === last)) {
          stop();
          (e.shiftKey ? last : first).focus();
        }
      }
      // Menus handle Esc themselves: it closes a submenu before the menu.
      if (overlay.type === "menu") return;
      // The switcher's link form goes back to the results first.
      if (overlay.type === "switcher" && is("hide") && document.activeElement?.closest(".create")) return;
      if (is("hide") || (overlay.type === "help" && is("help")) || (overlay.type === "switcher" && is("quickSwitcher"))) {
        stop();
        if (!e.repeat || is("hide")) closeOverlay();
      }
      return;
    }
    if (is("hide")) {
      // Let the link picker close itself first.
      if (view && completionStatus(view.state) !== null) return;
      if (linksOpen && document.activeElement?.closest(".links")) {
        stop();
        closeLinks();
        return;
      }
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
      if (linksOpen) {
        closeLinks();
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
      onStatus: (anchor) => pickStatus(undefined, undefined, undefined, anchor),
      onAdd: addMenu,
      onTag: showTag,
      tagColor: (t) => settings?.tagColors[t] ?? settings?.tagColors[t.split("/")[0]] ?? null,
      onPrimary: openPrimary,
      onLinks: () => toggleLinks("note", false),
      links: () => {
        const p = noteLinks.find((l) => l.primary);
        return { count: noteLinks.length, primary: p ? { label: p.rule || p.key ? p.label : p.domain, icon: p.rule?.icon ?? "external-link" } : null };
      },
      showEmpty: () => !!meta && meta.kind !== "daily",
    };
    window.addEventListener("keydown", onKeydown, true);
    window.addEventListener("contextmenu", onContextMenu);
    api.pinnedPaths().then((p) => {
      pinnedPaths = p;
      refreshSidebar();
    }, () => {});

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
      on<string[]>("pins-changed", (p) => {
        pinnedPaths = p;
        refreshSidebar();
      }),
      on<null>("open-settings", () => toggleSettings(true)),
      on<null>("index-ready", () => {
        indexing = false;
        refreshSide();
        refreshUntidy();
        listVersion++;
      }),
      on<null>("open-daily", openDaily),
      on<string>("switch-vault", (id) => id !== currentVault && switchVault(id)),
      on<null>("quit-requested", quit),
      on<string>("toast", (t) => toast(t, undefined, "warn")),
      on<string>("todos-carried", (t) => done(t)),
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
        if (Array.isArray(s.ui.tidyDismissed)) tidyDismissed = s.ui.tidyDismissed.filter((v): v is string => typeof v === "string");
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
        s.notices.forEach((n) => toaster.show(n, "info", undefined, 15000));
        let path = s.lastPath;
        const today = isoDate();
        // A daily note is named by its date, in any folder.
        const name = path ? (path.split("/").pop() ?? "").replace(/\.md$/, "") : "";
        const staleDaily = /^\d{4}-\d{2}-\d{2}$/.test(name) && name < today;
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
        offerTidy();
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
      pinned={pinnedHits}
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
      linksOn={linksOpen && linksScope === "all"}
      onLinks={() => toggleLinks("all")}
      onSettings={() => toggleSettings()}
      onHelp={() => (overlay = { type: "help" })}
      onSearch={() => openSwitcher(true)}
      onOpen={(p) => navigate(p)}
      onMenu={openNoteMenu}
      onCurrentMenu={(anchor) => openNoteMenu(meta, anchor)}
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
      keys={settings.keys}
      vaults={settings.vaults}
      onBack={back}
      onForward={forward}
      pinned={!!currentPath && pinnedPaths.includes(currentPath)}
      onPin={() => currentPath && pinPath(currentPath)}
      onActions={(anchor) => openNoteMenu(meta, anchor)}
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
      onMenu={openNoteMenu}
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
      {untidy}
      onTidy={openTidy}
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
      isLink={isLink}
      onCreateLink={(input) => (overlay = { type: "linkCreate", input, vault: currentVault })}
      onOpen={(p) => navigate(p)}
      onMenu={openNoteMenu}
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

  {#if linksOpen && settings}
    <div class="linkside" class:float={sideOverlay}>
      <LinksPanel
        bind:this={linksPanel}
        scope={linksScope}
        links={noteLinks}
        noteTitle={currentPath && meta ? (meta.label ? `${meta.title} ${meta.label}` : noteName(meta)) : null}
        vaults={settings.vaults}
        keys={settings.keys}
        version={listVersion}
        onScope={(sc) => toggleLinks(sc, false)}
        onOpenUrl={openExternal}
        onCopy={copyLink}
        onGoTo={goToSource}
        onEdit={editAttached}
        onMore={linkActions}
        onAdd={() => addLink()}
        onOpenNote={(p) => {
          if (sideOverlay) linksOpen = false;
          navigate(p);
        }}
        onClose={closeLinks}
      />
    </div>
  {/if}

  {#if overlay?.type === "menu"}
    <Menu label={overlay.label} items={overlay.items} anchor={overlay.anchor} statuses={settings?.statuses ?? []} onClose={closeOverlay} />
  {:else if overlay?.type === "file"}
    {@const o = overlay}
    <FileDialog title={noteName(o.target)} archived={o.target.archived}
      onArchive={() => archiveInstead(o.target)} onDelete={() => deleteNote(o.target)} onClose={closeOverlay} />
  {:else if overlay?.type === "tidy"}
    {@const o = overlay}
    <TidyDialog name={vaultName(o.vault)} preview={o.preview} onTidy={() => runTidy(o.vault)} onLater={() => laterTidy(o.vault)} onClose={closeOverlay} />
  {:else if overlay?.type === "switcher"}
    <Switcher
      mode={overlay.mode}
      current={currentPath}
      {isLink}
      createLink={(input, vault, title, rule) => api.createFromLink(input, vault, title, rule)}
      onCreatedLink={(path) => {
        const o = overlay;
        createdFromLink(path, o?.type === "switcher" && o.mode === "link" ? o.sel : undefined);
      }}
      statuses={settings?.statuses ?? []}
      tagColors={settings?.tagColors ?? {}}
      vaults={settings?.vaults ?? []}
      {currentVault}
      onChoose={switcherChoose}
      onCreate={switcherCreate}
      onMenu={overlay.mode === "open" ? openNoteMenu : undefined}
      onAddTodo={addTodo}
      onShowAll={(q) => {
        closeOverlay();
        openNotesView(q);
      }}
      onClose={closeOverlay}
    />
  {:else if overlay?.type === "help" && settings}
    <Help keys={settings.keys} hotkey={settings.hotkey} switcherHotkey={settings.switcherHotkey} {settingsPath} vaults={settings.vaults} {vaultPaths} onClose={closeOverlay} />
  {:else if overlay?.type === "prompt"}
    <Prompt title={overlay.title} value={overlay.value} placeholder={overlay.placeholder} anchor={overlay.anchor} onSubmit={overlay.submit} onClose={closeOverlay} />
  {:else if overlay?.type === "status" && settings}
    {@const o = overlay}
    <StatusPicker statuses={settings.statuses} current={o.current} title={o.title} anchor={o.anchor} onChoose={(st) => applyStatus(o.path, st)} onClose={closeOverlay} />
  {:else if overlay?.type === "tags" && settings}
    {@const o = overlay}
    <TagPicker
      title={o.title}
      tags={o.tags}
      all={o.all}
      colors={settings.tagColors}
      anchor={o.anchor}
      onChange={(t) => applyTags(o.path, t)}
      onClose={closeOverlay}
    />
  {:else if overlay?.type === "linkCreate" && settings}
    {@const o = overlay}
    <div class="backdrop" role="presentation" in:dim use:dismissOnBackdrop={closeOverlay}>
      <div class="dialog" in:pop role="dialog" aria-label="Create note from link">
        <LinkCreate
          input={o.input}
          vaults={settings.vaults}
          vault={o.vault}
          create={(input, vault, title, rule) => api.createFromLink(input, vault, title, rule)}
          onCreated={(path) => createdFromLink(path)}
          onOpen={(h) => {
            closeOverlay();
            navigate(h.path);
          }}
          onCancel={closeOverlay}
        />
      </div>
    </div>
  {:else if overlay?.type === "picker"}
    <Picker title={overlay.title} items={overlay.items} anchor={overlay.anchor} onChoose={overlay.choose} onClose={closeOverlay} />
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
  .backdrop {
    position: fixed;
    inset: 0;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 12vh;
    background: var(--scrim);
    z-index: 25;
  }
  .dialog {
    width: min(560px, calc(100vw - 32px));
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: var(--shadow);
  }
  .linkside {
    display: flex;
    min-height: 0;
  }
  .linkside.float {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    z-index: 16;
    box-shadow: var(--shadow);
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
