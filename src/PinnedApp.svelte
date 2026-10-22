<script lang="ts">
  // A pinned note (spec section 15): a small always-on-top window fixed to one
  // note. It shares the note's buffer with the main window, so both show the
  // same live text. Links open in the main window; Esc does nothing here.
  import { onMount } from "svelte";
  import { EditorState, type Extension } from "@codemirror/state";
  import { EditorView } from "@codemirror/view";
  import StatusIcon from "./components/StatusIcon.svelte";
  import StatusPicker from "./components/StatusPicker.svelte";
  import Picker from "./components/Picker.svelte";
  import { api, on, type NoteMeta, type Settings } from "./lib/api";
  import { NoteSync } from "./lib/buffer";
  import { decoConfig, editorExtensions, editorState, setEditorDark, setSpellCheck } from "./lib/editor";
  import { applyAppearance } from "./lib/appearance";
  import { liveConfig, propertiesHooks } from "./lib/live";
  import { matches, matchesChordTail, parseChord, parseKey } from "./lib/keys";
  import { statusInfo } from "./lib/status";
  import { firstTicket, gerritUrl, jiraUrl, refAt, ticketRegex, timestamp, checkboxToggle } from "./lib/text";

  let settings = $state<Settings | null>(null);
  let path = $state<string | null>(null);
  let meta = $state<NoteMeta | null>(null);
  let conflict = $state<{ deleted: boolean } | null>(null);
  let onTop = $state(true);
  let message = $state("");
  let overlay = $state<
    | { type: "status" }
    | { type: "gerrit"; items: { label: string; detail?: string }[]; urls: string[] }
    | null
  >(null);
  let chordPending = false;

  let editorEl: HTMLDivElement;
  let view: EditorView;
  let extensions: Extension[];
  let sync: NoteSync | null = null;
  let tickets = ticketRegex("[A-Z][A-Z0-9]+-\\d+");

  const darkQuery = window.matchMedia("(prefers-color-scheme: dark)");

  function say(text: string) {
    message = text;
    setTimeout(() => (message === text ? (message = "") : null), 3000);
  }

  function applySettings(s: Settings) {
    settings = s;
    tickets = ticketRegex(s.ticketPattern);
    decoConfig.tickets = tickets;
    liveConfig.enabled = s.liveFormatting;
    applyAppearance(s);
    editorState.spellCheck = s.spellCheck;
    if (view) setSpellCheck(view, s.spellCheck);
    applyDark();
  }

  function applyDark() {
    const theme = settings?.theme ?? "system";
    editorState.dark = theme === "dark" || (theme === "system" && darkQuery.matches);
    if (view) setEditorDark(view, editorState.dark);
  }

  async function load() {
    if (!path) return;
    const data = await api.bufferOpen(path);
    sync?.close();
    meta = data.meta;
    conflict = data.deleted ? { deleted: true } : null;
    view.setState(EditorState.create({ doc: data.text, extensions: [extensions, NoteSync.extension(data)] }));
    sync = new NoteSync(path, view, load);
    view.dispatch({ selection: { anchor: data.text.length }, scrollIntoView: true });
    view.focus();
  }

  function followAt(pos: number): boolean {
    const line = view.state.doc.lineAt(pos);
    const ref = refAt(line.text, pos - line.from, tickets);
    if (!ref) return false;
    if (ref.kind === "url") api.openUrl(ref.target).catch((e) => say(String(e)));
    else api.resolveLink(ref.target).then((p) => api.openInMain(p), (e) => say(String(e)));
    return true;
  }

  function openJira() {
    if (!meta || !settings) return;
    const key = meta.kind === "ticket" ? meta.key : firstTicket(view.state.doc.toString(), tickets);
    if (meta.kind === "ticket" && meta.jira) api.openUrl(meta.jira);
    else if (key) api.openUrl(jiraUrl(settings.jira.urlTemplate, key));
    else say("No ticket key in this note");
  }

  async function openGerrit() {
    if (!meta || !settings) return;
    await sync?.drain();
    const t = settings.gerrit.urlTemplate;
    const urls = meta.gerrit.map((g) => gerritUrl(t, g));
    if (!urls.length) say("No Gerrit links in this note");
    else if (urls.length === 1) api.openUrl(urls[0]);
    else overlay = { type: "gerrit", items: meta.gerrit.map((g, i) => ({ label: g, detail: urls[i] })), urls };
  }

  async function setStatus(status: string | null) {
    overlay = null;
    if (!path) return;
    try {
      await sync?.drain();
      const r = await api.setStatus(path, status);
      meta = r.meta;
    } catch (e) {
      say(String(e));
    }
    view.focus();
  }

  async function toggleOnTop() {
    onTop = !onTop;
    await api.pinSetOnTop(onTop);
  }

  function onKeydown(e: KeyboardEvent) {
    if (!settings || e.isComposing) return;
    const k = settings.keys;
    const stop = () => {
      e.preventDefault();
      e.stopPropagation();
    };
    if (overlay) {
      if (e.key === "Escape") {
        stop();
        overlay = null;
        view.focus();
      }
      return;
    }
    const chord = parseChord(k.pinNote);
    if (chordPending) {
      chordPending = false;
      if (matchesChordTail(e, chord)) {
        stop();
        api.pinClose();
        return;
      }
    }
    if (chord.length === 2 && matches(e, chord[0])) {
      stop();
      chordPending = true;
      setTimeout(() => (chordPending = false), 1500);
      return;
    }
    const is = (spec: string) => matches(e, parseKey(spec));
    if (is(k.timestamp)) {
      stop();
      view.dispatch(view.state.replaceSelection(timestamp(new Date(), settings.timestampFormat)));
    } else if (is(k.openJira)) {
      stop();
      openJira();
    } else if (is(k.openGerrit)) {
      stop();
      openGerrit();
    } else if (is(k.setStatus)) {
      stop();
      if (meta?.kind !== "daily") overlay = { type: "status" };
    } else if (is(k.followLink)) {
      stop();
      const pos = view.state.selection.main.head;
      if (!followAt(pos)) {
        const line = view.state.doc.lineAt(pos);
        const box = checkboxToggle(line.text);
        if (box) view.dispatch({ changes: { from: line.from + box.at, to: line.from + box.at + 1, insert: box.insert } });
      }
    } else if (e.key === "Escape") {
      // Never closed by accident.
      stop();
    } else if (
      ["F3", "F5", "F7"].includes(e.key) ||
      (e.ctrlKey && !e.altKey && ["r", "p", "f", "g", "h", "j", "n", "o", "t", "u", "w", "e", "l"].includes(e.key.toLowerCase()))
    ) {
      e.preventDefault();
    }
  }

  onMount(() => {
    extensions = editorExtensions({
      onChange: () => sync?.push(),
      follow: followAt,
      linkSearch: (q) => api.search(q, false, path, 30),
      paste: () => false,
    });
    view = new EditorView({ parent: editorEl, state: EditorState.create({ doc: "", extensions }) });
    propertiesHooks.current = {
      statusView: (st) => statusInfo(st, settings?.statuses ?? []),
      onStatus: () => (overlay = { type: "status" }),
      onJira: openJira,
      onGerrit: openGerrit,
      hasJira: () => meta?.kind === "ticket" || firstTicket(view.state.doc.toString(), tickets) !== null,
    };
    window.addEventListener("keydown", onKeydown, true);
    darkQuery.addEventListener("change", applyDark);
    const unlisten = [
      on<{ path: string; dirty: boolean; meta: NoteMeta | null }>("buffer-state", (ev) => {
        if (ev.path !== path) return;
        if (ev.meta) meta = ev.meta;
        if (!ev.dirty && conflict && !conflict.deleted) conflict = null;
      }),
      on<{ path: string; deleted: boolean }>("buffer-conflict", (ev) => {
        if (ev.path === path) conflict = { deleted: ev.deleted };
      }),
      on<{ from: string; to: string }>("note-moved", (m) => {
        if (m.from !== path) return;
        path = m.to;
        if (sync) sync.path = m.to;
        if (meta) meta = { ...meta, path: m.to, archived: m.to.startsWith("archive/") };
      }),
      on<Settings>("settings-changed", applySettings),
    ];
    (async () => {
      try {
        applySettings(await api.getSettings());
        path = await api.pinPath();
        if (!path) return say("This pinned window lost its note");
        await load();
        if (await api.smokeEnabled()) {
          // CI self-test: report that this window works, then close it like the × button.
          await api.smokeStep("pin-loaded");
          setTimeout(() => api.pinClose(), 300);
        }
      } catch (e) {
        say(String(e));
      }
    })();
    return () => {
      window.removeEventListener("keydown", onKeydown, true);
      darkQuery.removeEventListener("change", applyDark);
      unlisten.forEach((p) => p.then((f) => f()));
      sync?.close();
    };
  });
</script>

<main class="pinned">
  <header class="bar" data-tauri-drag-region>
    {#if meta && settings && meta.status}<StatusIcon status={meta.status} statuses={settings.statuses} />{/if}
    <span class="title" data-tauri-drag-region>
      {#if meta}
        <b data-tauri-drag-region>{meta.title}</b>{#if meta.label}<span class="label" data-tauri-drag-region> {meta.label}</span>{/if}
      {/if}
    </span>
    <button class="icon" class:off={!onTop} onclick={toggleOnTop} title={onTop ? "Always on top (click to release)" : "Keep on top"} aria-label="Pin on top">
      <svg viewBox="0 0 20 20" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M7 3h6l-1 5 3 3H5l3-3zM10 11v6" /></svg>
    </button>
    <button class="icon" onclick={() => api.pinClose()} title="Close pinned note" aria-label="Close">
      <svg viewBox="0 0 20 20" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true"><path d="M5 5l10 10M15 5L5 15" /></svg>
    </button>
  </header>

  {#if conflict}
    <div class="conflict">
      {#if conflict.deleted}
        This note was deleted outside the app.
        <button onclick={() => path && api.bufferResolve(path, false).then(() => (conflict = null))}>Save as new note</button>
        <button onclick={() => api.pinClose()}>Close</button>
      {:else}
        Changed on disk while you were editing.
        <button onclick={() => path && api.bufferResolve(path, true)}>Reload</button>
        <button onclick={() => path && api.bufferResolve(path, false)}>Keep mine</button>
      {/if}
    </div>
  {/if}

  <div class="editor" bind:this={editorEl}></div>

  {#if message}<div class="message">{message}</div>{/if}

  {#if overlay?.type === "status" && settings}
    <StatusPicker statuses={settings.statuses} current={meta?.status ?? null} title="Status" onChoose={setStatus} />
  {:else if overlay?.type === "gerrit"}
    {@const o = overlay}
    <Picker title="Open Gerrit change" items={o.items} onChoose={(i) => { overlay = null; api.openUrl(o.urls[i]); }} />
  {/if}
</main>

<style>
  .pinned {
    height: 100vh;
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    box-sizing: border-box;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 4px 0 10px;
    background: var(--bg-raised);
    border-bottom: 1px solid var(--border);
    font-size: 12.5px;
    user-select: none;
    flex-shrink: 0;
  }
  .title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .label {
    color: var(--fg-muted);
  }
  .icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 22px;
    color: var(--fg-muted);
    background: transparent;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }
  .icon:hover {
    background: var(--hover);
    color: var(--fg);
  }
  .icon.off {
    opacity: 0.45;
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
  .editor :global(.cm-content) {
    padding: 0 10px !important;
  }
  .editor :global(.cm-scroller) {
    padding-top: 4px !important;
  }
  .conflict {
    padding: 5px 8px;
    font-size: 12px;
    background: var(--warn-bg);
    border-bottom: 1px solid var(--border);
  }
  .conflict button {
    font: inherit;
    color: var(--fg);
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 1px 6px;
    margin-left: 4px;
    cursor: pointer;
  }
  .message {
    position: fixed;
    left: 8px;
    right: 8px;
    bottom: 8px;
    padding: 5px 8px;
    font-size: 12px;
    background: var(--toast-bg);
    color: var(--toast-fg);
    border-radius: 6px;
  }
</style>
