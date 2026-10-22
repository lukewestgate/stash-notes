<script lang="ts">
  import { onMount } from "svelte";
  import { EditorState, type Extension } from "@codemirror/state";
  import { EditorView } from "@codemirror/view";
  import { completionStatus } from "@codemirror/autocomplete";
  import Header from "./components/Header.svelte";
  import Switcher from "./components/Switcher.svelte";
  import Backlinks from "./components/Backlinks.svelte";
  import Help from "./components/Help.svelte";
  import Prompt from "./components/Prompt.svelte";
  import Picker from "./components/Picker.svelte";
  import StatusPicker from "./components/StatusPicker.svelte";
  import NotesView, { type NotesViewState } from "./components/NotesView.svelte";
  import { api, on, onWindowFocus, type Keys, type NoteData, type NoteMeta, type SearchHit, type Settings } from "./lib/api";
  import {
    decoConfig,
    editorExtensions,
    editorState,
    refreshDecorations,
    setEditorDark,
    setLiveFormatting,
  } from "./lib/editor";
  import { liveConfig, propertiesHooks } from "./lib/live";
  import { statusInfo } from "./lib/status";
  import { History, type Entry } from "./lib/history";
  import { formatKey, matches, parseKey, type KeySpec } from "./lib/keys";
  import {
    checkboxToggle,
    firstTicket,
    gerritUrl,
    isoDate,
    jiraUrl,
    looksLikeGerrit,
    refAt,
    ticketRegex,
    timestamp,
    todoCursor,
  } from "./lib/text";

  type Overlay =
    | { type: "switcher"; mode: "open" | "link"; sel?: { from: number; to: number; text: string } }
    | { type: "help" }
    | { type: "prompt"; title: string; value: string; placeholder?: string; submit: (v: string) => void }
    | { type: "picker"; title: string; items: { label: string; detail?: string }[]; choose: (i: number) => void }
    | { type: "status"; path: string; current: string | null; title: string };

  interface Toast {
    id: number;
    text: string;
    action?: { label: string; run: () => void };
  }

  const AUTOSAVE_MS = 300;

  let settings = $state<Settings | null>(null);
  let vaultPath = $state("");
  let settingsPath = $state("");
  let meta = $state<NoteMeta | null>(null);
  let dirty = $state(false);
  let hasKey = $state(false);
  let backlinkItems = $state<SearchHit[]>([]);
  let backlinksOpen = $state(false);
  let overlay = $state<Overlay | null>(null);
  let toasts = $state<Toast[]>([]);
  /** Set when the open note changed on disk while we had unsaved edits (or it vanished). */
  let conflict = $state<{ disk: string | null } | null>(null);
  let nav = $state({ canBack: false, canForward: false });
  /** What fills the main area. The editor stays mounted underneath full-window views. */
  let screen = $state<"editor" | "notes">("editor");
  let notesState = $state<NotesViewState>({ query: "", sort: "edited", grouped: false, showDaily: false, showPutAway: false });
  /** Bumped when notes change, so lists reload. */
  let listVersion = $state(0);
  let notesView = $state<NotesView>();

  /** History path standing for the Notes view. */
  const NOTES = "::notes";

  let editorEl: HTMLDivElement;
  let view: EditorView;
  let extensions: Extension[];
  const hist = new History();
  let currentPath = $state<string | null>(null);
  let savedContent = "";
  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  let saveChain: Promise<void> = Promise.resolve();
  /** Write even if the text matches what we last loaded ("Keep mine" after a conflict). */
  let forceSave = false;
  let tickets = ticketRegex("[A-Z][A-Z0-9]+-\\d+");
  let keySpecs: Partial<Record<keyof Keys, KeySpec | null>> = {};
  let toastId = 0;

  // ---------- small helpers ----------

  function toast(text: string, action?: Toast["action"], ms = action ? 5000 : 3500) {
    const id = ++toastId;
    toasts = [...toasts, { id, text, action }];
    setTimeout(() => (toasts = toasts.filter((t) => t.id !== id)), ms);
  }

  function fail(e: unknown) {
    toast(String(e instanceof Error ? e.message : e));
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

  function isTicketKey(q: string) {
    return new RegExp(`^(?:${settings?.ticketPattern ?? "[A-Z][A-Z0-9]+-\\d+"})$`).test(q.trim().toUpperCase());
  }

  function applySettings(s: Settings) {
    settings = s;
    keySpecs = Object.fromEntries(Object.entries(s.keys).map(([k, v]) => [k, parseKey(v)]));
    tickets = ticketRegex(s.ticketPattern);
    decoConfig.tickets = tickets;
    const root = document.documentElement;
    if (s.theme === "light" || s.theme === "dark") root.dataset.theme = s.theme;
    else delete root.dataset.theme;
    root.style.setProperty("--font-size", `${s.fontSize || 14}px`);
    liveConfig.enabled = s.liveFormatting;
    applyDark();
    if (view) {
      setLiveFormatting(view, s.liveFormatting);
      hasKey = computeHasKey();
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

  function computeHasKey() {
    return meta?.kind === "ticket" || firstTicket(docText(), tickets) !== null;
  }

  // ---------- saving ----------

  function save(): Promise<void> {
    saveChain = saveChain.then(async () => {
      if (!currentPath || conflict) return;
      const content = docText();
      if (content === savedContent && !forceSave) {
        dirty = false;
        return;
      }
      const path = currentPath;
      try {
        const m = await api.saveNote(path, content);
        forceSave = false;
        if (path === currentPath) {
          savedContent = content;
          meta = m;
          dirty = docText() !== content;
          hasKey = computeHasKey();
        }
      } catch (e) {
        toast(`Could not save: ${e}`);
      }
    });
    return saveChain;
  }

  function flush(): Promise<void> {
    clearTimeout(saveTimer);
    return save();
  }

  function onDocChange() {
    dirty = true;
    clearTimeout(saveTimer);
    saveTimer = setTimeout(save, AUTOSAVE_MS);
  }

  // ---------- opening notes ----------

  /** Where the cursor goes when a note is opened fresh. */
  function defaultCursor(data: NoteData): number {
    if (data.meta.kind === "daily") return todoCursor(data.content) ?? data.content.length;
    // A ticket with no name yet: put the caret after `# PROJ-123 ` so the name can be typed.
    if (data.meta.kind === "ticket" && !data.meta.label) {
      const m = /^# .*$/m.exec(data.content);
      if (m) return m.index + m[0].length;
    }
    return data.content.length;
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
    const data = await api.openNote(path);
    screen = "editor";
    currentPath = path;
    savedContent = data.content;
    meta = data.meta;
    dirty = false;
    conflict = null;
    view.setState(EditorState.create({ doc: data.content, extensions }));
    const len = data.content.length;
    const anchor = restore && entry && entry.anchor >= 0 ? Math.min(entry.anchor, len) : defaultCursor(data);
    view.dispatch({ selection: { anchor }, effects: EditorView.scrollIntoView(anchor, { y: "center" }) });
    if (restore && entry) {
      const top = entry.scroll;
      requestAnimationFrame(() => (view.scrollDOM.scrollTop = top));
    }
    hasKey = computeHasKey();
    updateNav();
    refreshBacklinks();
    focusEditor();
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

  /** Leave the Notes view: back to where you came from, else today's note. */
  async function leaveNotesView() {
    if (hist.canBack) await back();
    else await openDaily();
  }

  async function openDaily() {
    try {
      navigate(await api.dailyNote(isoDate()));
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
    else api.resolveLink(ref.target).then((p) => navigate(p), fail);
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

  function openJira() {
    if (!meta || !settings) return;
    const template = settings.jira.urlTemplate;
    if (meta.kind === "ticket" && meta.key) {
      openExternal(meta.jira ?? jiraUrl(template, meta.key));
      return;
    }
    const pos = head();
    const line = view.state.doc.lineAt(pos);
    const ref = refAt(line.text, pos - line.from, tickets);
    let key: string | null = null;
    if (ref?.kind === "ticket") key = ref.target;
    else if (ref?.kind === "wiki" && isTicketKey(ref.target)) key = ref.target.toUpperCase();
    key ??= firstTicket(docText(), tickets);
    if (key) openExternal(jiraUrl(template, key));
    else toast("No ticket key in this note");
  }

  async function openGerrit() {
    if (!settings) return;
    await flush();
    const template = settings.gerrit.urlTemplate;
    const list = meta?.gerrit ?? [];
    if (list.length === 0) {
      toast(`No Gerrit links yet — add one with ${formatKey(settings.keys.addGerrit)}`);
    } else if (list.length === 1) {
      openExternal(gerritUrl(template, list[0]));
    } else {
      overlay = {
        type: "picker",
        title: "Open Gerrit change",
        items: list.map((g) => ({ label: g, detail: gerritUrl(template, g) })),
        choose: (i) => {
          closeOverlay();
          openExternal(gerritUrl(template, list[i]));
        },
      };
    }
  }

  async function addGerrit() {
    if (!settings || !currentPath) return;
    const clip = await api.readClipboard().catch(() => "");
    overlay = {
      type: "prompt",
      title: "Add Gerrit change — paste a URL or type a change number",
      placeholder: "https://gerrit…/c/project/+/12345 or 12345",
      value: looksLikeGerrit(clip, settings.gerrit.urlTemplate) ? clip.trim() : "",
      submit: async (v) => {
        closeOverlay();
        try {
          const edit = await api.gerritEdit(docText(), v);
          view.dispatch({ changes: edit });
          await flush();
          toast(`Gerrit link added (${meta?.gerrit.length ?? 1} total)`);
        } catch (e) {
          fail(e);
        }
      },
    };
  }

  function insertTimestamp() {
    view.dispatch(view.state.replaceSelection(timestamp()));
    view.focus();
  }

  // ---------- archive / rename ----------

  function moved(from: string, to: string) {
    hist.rename(from, to);
    if (currentPath === from) {
      currentPath = to;
      if (meta) meta = { ...meta, path: to, archived: to.startsWith("archive/") };
    }
    refreshSide();
  }

  async function toggleArchive() {
    if (!currentPath) return;
    try {
      await flush();
      const from = currentPath;
      const to = await api.archiveNote(from);
      moved(from, to);
      const archived = to.startsWith("archive/");
      toast(archived ? "Archived" : "Unarchived", {
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
    } catch (e) {
      fail(e);
    }
  }

  function startRename() {
    if (!meta || !currentPath) return;
    if (meta.kind !== "note") {
      toast(
        meta.kind === "daily"
          ? "Daily notes are named by their date"
          : "Ticket notes are named by their key — edit the name after the key in the first heading instead",
      );
      return;
    }
    const from = currentPath;
    overlay = {
      type: "prompt",
      title: "Rename note (links to it are updated)",
      value: meta.title,
      submit: async (title) => {
        closeOverlay();
        try {
          await flush();
          remember();
          const r = await api.renameNote(from, title);
          hist.rename(from, r.newPath);
          await show(r.newPath, hist.current, true);
          const links = r.linksUpdated === 1 ? "1 link" : `${r.linksUpdated} links`;
          const notes = r.filesChanged === 1 ? "1 other note" : `${r.filesChanged} other notes`;
          toast(`Renamed — updated ${links} in ${notes}`, {
            label: "Undo",
            run: async () => {
              try {
                await flush();
                remember();
                const old = await api.undoRename(r.undoId);
                hist.rename(r.newPath, old);
                if (currentPath === r.newPath) await show(old, hist.current, true);
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
      if (path === currentPath) {
        if (r.edit) view.dispatch({ changes: r.edit });
        savedContent = docText();
        dirty = false;
        meta = r.meta;
      }
      if (r.path !== path) {
        moved(path, r.path);
        toast(r.meta.archived ? "Closed — moved to the archive" : "Reopened — moved out of the archive");
      }
      listVersion++;
      refreshSide();
    } catch (e) {
      fail(e);
    }
  }

  // ---------- overlays ----------

  function openSwitcher() {
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
    requestAnimationFrame(() => view.focus());
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

  async function switcherCreate(query: string) {
    const o = overlay;
    closeOverlay();
    try {
      const path = await api.createNote(query);
      if (o?.type === "switcher" && o.mode === "link" && o.sel) {
        insertLink(isTicketKey(query) ? query.toUpperCase() : query, o.sel);
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
      try {
        decoConfig.archived = new Set(await api.archivedNames());
        refreshDecorations(view);
      } catch {
        // ignore
      }
    }, 150);
  }

  async function onVaultChanged(paths: string[]) {
    refreshSide();
    listVersion++;
    const path = currentPath;
    if (!path || !paths.includes(path)) return;
    const disk = await api.readNote(path).catch(() => undefined);
    if (disk === undefined || path !== currentPath || disk === savedContent) return;
    const current = docText();
    if (disk === current) {
      savedContent = disk;
      dirty = false;
      return;
    }
    if (disk !== null && current === savedContent && !dirty) {
      reloadFromDisk(disk);
    } else {
      conflict = { disk };
    }
  }

  async function reloadFromDisk(text: string) {
    const anchor = Math.min(head(), text.length);
    const scroll = view.scrollDOM.scrollTop;
    savedContent = text;
    view.setState(EditorState.create({ doc: text, extensions, selection: { anchor } }));
    requestAnimationFrame(() => (view.scrollDOM.scrollTop = scroll));
    dirty = false;
    conflict = null;
    if (currentPath) {
      try {
        meta = (await api.openNote(currentPath)).meta;
      } catch {
        // keep old meta
      }
    }
    hasKey = computeHasKey();
    focusEditor();
  }

  function keepMine() {
    conflict = null;
    forceSave = true;
    flush();
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
    openJira,
    openGerrit,
    addGerrit,
    timestamp: insertTimestamp,
    backlinks: () => {
      backlinksOpen = !backlinksOpen;
      if (!backlinksOpen) focusEditor();
    },
    archive: toggleArchive,
    help: () => (overlay = { type: "help" }),
    quit,
    rename: startRename,
    openSettings: () => api.openSettingsFile().catch(fail),
    setStatus: () => {
      if (screen === "notes") {
        const h = notesView?.selected();
        if (h) pickStatus(h.path, h.status, h.key ?? h.title);
      } else pickStatus();
    },
    notesView: () => openNotesView(),
  };

  /** Actions that only make sense with a note in the editor. */
  const EDITOR_ONLY = new Set<keyof Keys>([
    "followLink",
    "openJira",
    "openGerrit",
    "addGerrit",
    "timestamp",
    "backlinks",
    "archive",
    "rename",
  ]);

  function onKeydown(e: KeyboardEvent) {
    if (e.isComposing || !settings) return;
    const is = (a: keyof Keys) => matches(e, keySpecs[a] ?? null);
    const stop = () => {
      e.preventDefault();
      e.stopPropagation();
    };

    if (overlay) {
      if (is("hide") || (overlay.type === "help" && is("help"))) {
        stop();
        closeOverlay();
      }
      return;
    }
    if (is("hide")) {
      // Let the link picker close itself first.
      if (view && completionStatus(view.state) !== null) return;
      stop();
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

  // ---------- startup ----------

  onMount(() => {
    extensions = editorExtensions({
      onChange: onDocChange,
      follow: followAt,
      linkSearch: (q) => api.search(q, false, currentPath, 30),
    });
    view = new EditorView({ parent: editorEl, state: EditorState.create({ doc: "", extensions }) });
    darkQuery.addEventListener("change", applyDark);
    // Lets UI tests drive the editor in `npm run dev`; stripped from builds.
    if (import.meta.env.DEV) (window as unknown as { __view: () => EditorView }).__view = () => view;
    propertiesHooks.current = {
      statusView: (st) => statusInfo(st, settings?.statuses ?? []),
      onStatus: () => pickStatus(),
      onJira: openJira,
      onGerrit: openGerrit,
      hasJira: () => hasKey,
    };
    window.addEventListener("keydown", onKeydown, true);
    window.addEventListener("contextmenu", onContextMenu);

    const unlisten: Promise<() => void>[] = [
      on<null>("summoned", onSummoned),
      on<string[]>("vault-changed", onVaultChanged),
      on<null>("index-ready", refreshSide),
      on<null>("open-daily", openDaily),
      on<null>("quit-requested", quit),
      on<string>("toast", (t) => toast(t)),
      on<Settings>("settings-changed", (s) => {
        applySettings(s);
        toast("Settings reloaded");
      }),
      onWindowFocus((focused) => {
        if (!focused) onBlur();
      }),
    ];

    (async () => {
      try {
        const s = await api.startup();
        applySettings(s.settings);
        notesState = {
          query: "",
          sort: s.settings.notesView.sort,
          grouped: s.settings.notesView.groupByStatus,
          showDaily: s.settings.notesView.showDaily,
          showPutAway: s.settings.notesView.showClosed,
        };
        vaultPath = s.vaultPath;
        settingsPath = s.settingsPath;
        s.warnings.forEach((w) => toast(w, undefined, 8000));
        let path = s.lastPath;
        const today = isoDate();
        const staleDaily = path?.startsWith("daily/") && path < `daily/${today}.md`;
        if (!path || s.settings.onSummon === "daily" || staleDaily) {
          path = await api.dailyNote(today);
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

<main class="app">
  <div class="pane" class:hidden={screen !== "editor"}>
  {#if settings}
    <Header
      {meta}
      {dirty}
      canBack={nav.canBack}
      canForward={nav.canForward}
      {hasKey}
      keys={settings.keys}
      statuses={settings.statuses}
      onStatus={() => pickStatus()}
      onBack={back}
      onForward={forward}
      onJira={openJira}
      onGerrit={openGerrit}
      onSwitcher={openSwitcher}
    />
  {/if}

  {#if conflict}
    <div class="conflict" role="alert">
      {#if conflict.disk === null}
        <span>This note was deleted or moved outside the app.</span>
        <button onclick={keepMine}>Keep editing (recreates it)</button>
        <button onclick={back}>Go back</button>
      {:else}
        <span>This note changed on disk while you had unsaved edits.</span>
        <button onclick={() => reloadFromDisk(conflict!.disk!)}>Reload from disk</button>
        <button onclick={keepMine}>Keep mine</button>
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

  {#if screen === "notes" && settings}
    <NotesView
      bind:this={notesView}
      bind:view={notesState}
      statuses={settings.statuses}
      keys={settings.keys}
      version={listVersion}
      isTicket={isTicketKey}
      onOpen={(p) => navigate(p)}
      onCreate={async (q) => {
        try {
          navigate(await api.createNote(q.replace(/@\S+/g, "").trim()));
        } catch (e) {
          fail(e);
        }
      }}
    />
  {/if}

  {#if overlay?.type === "switcher"}
    <Switcher
      mode={overlay.mode}
      current={currentPath}
      isTicket={isTicketKey}
      statuses={settings?.statuses ?? []}
      onChoose={switcherChoose}
      onCreate={switcherCreate}
    />
  {:else if overlay?.type === "help" && settings}
    <Help keys={settings.keys} hotkey={settings.hotkey} {settingsPath} {vaultPath} />
  {:else if overlay?.type === "prompt"}
    <Prompt title={overlay.title} value={overlay.value} placeholder={overlay.placeholder} onSubmit={overlay.submit} />
  {:else if overlay?.type === "status" && settings}
    {@const o = overlay}
    <StatusPicker statuses={settings.statuses} current={o.current} title={o.title} onChoose={(st) => applyStatus(o.path, st)} />
  {:else if overlay?.type === "picker"}
    <Picker title={overlay.title} items={overlay.items} onChoose={overlay.choose} />
  {/if}

  <div class="toasts" aria-live="polite">
    {#each toasts as t (t.id)}
      <div class="toast">
        <span>{t.text}</span>
        {#if t.action}
          <button
            onclick={() => {
              t.action!.run();
              toasts = toasts.filter((x) => x.id !== t.id);
            }}>{t.action.label}</button
          >
        {/if}
      </div>
    {/each}
  </div>
</main>

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
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
  .conflict button,
  .toast button {
    font: inherit;
    font-size: 12.5px;
    color: var(--fg);
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 2px 8px;
    cursor: pointer;
  }
  .toasts {
    position: fixed;
    right: 12px;
    bottom: 36px;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 6px;
    z-index: 40;
    pointer-events: none;
  }
  .toast {
    display: flex;
    align-items: center;
    gap: 10px;
    max-width: min(520px, calc(100vw - 24px));
    padding: 7px 12px;
    background: var(--toast-bg);
    color: var(--toast-fg);
    border-radius: 7px;
    box-shadow: var(--shadow);
    font-size: 13px;
    pointer-events: auto;
  }
</style>
