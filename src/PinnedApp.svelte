<script lang="ts">
  // A pinned note (spec section 15): a small always-on-top window fixed to one
  // note. It shares the note's buffer with the main window, so both show the
  // same live text. Links open in the main window; Esc does nothing here.
  import { onMount } from "svelte";
  import { EditorState, type Extension } from "@codemirror/state";
  import { EditorView } from "@codemirror/view";
  import StatusIcon from "./components/StatusIcon.svelte";
  import StatusPicker from "./components/StatusPicker.svelte";
  import TagPicker from "./components/TagPicker.svelte";
  import Picker from "./components/Picker.svelte";
  import Prompt from "./components/Prompt.svelte";
  import LinksPanel from "./components/LinksPanel.svelte";
  import { api, on, type LinkSource, type MetaEdit, type NoteLink, type NoteMeta, type Settings, type TagCount } from "./lib/api";
  import { NoteSync } from "./lib/buffer";
  import { pickImages } from "./lib/images";
  import { decoConfig, editorExtensions, editorState, refreshDecorations, setEditorDark, setSpellCheck } from "./lib/editor";
  import { applyAppearance } from "./lib/appearance";
  import Icon from "./components/Icon.svelte";
  import { liveConfig, propertiesHooks } from "./lib/live";
  import { matches, matchesChordTail, parseChord, parseKey } from "./lib/keys";
  import { statusInfo } from "./lib/status";
  import { refAt, ticketRegex, timestamp, checkboxToggle } from "./lib/text";
  import { isUrl, keyRegex } from "./lib/links";

  let settings = $state<Settings | null>(null);
  let path = $state<string | null>(null);
  let meta = $state<NoteMeta | null>(null);
  let conflict = $state<{ deleted: boolean } | null>(null);
  let onTop = $state(true);
  let message = $state("");
  let overlay = $state<
    | { type: "status" }
    | { type: "tags"; all: TagCount[] }
    | { type: "links"; items: { label: string; detail?: string }[]; urls: string[] }
    | { type: "actions"; title: string; items: { label: string; detail?: string; run: () => void }[] }
    | { type: "prompt"; title: string; value: string; submit: (v: string) => void }
    | null
  >(null);
  /** The Links panel, over the editor (this window is small). */
  let linksOpen = $state(false);
  let chordPending = false;
  /** The note's links, as its text is now. */
  let noteLinks = $state<NoteLink[]>([]);
  /** Set by Ctrl+Shift+V: the next paste goes in as typed. */
  let rawPaste = false;

  let editorEl: HTMLDivElement;
  let view: EditorView;
  let extensions: Extension[];
  let sync: NoteSync | null = null;
  let tickets = /(?!)/g;

  const darkQuery = window.matchMedia("(prefers-color-scheme: dark)");

  function say(text: string) {
    message = text;
    setTimeout(() => (message === text ? (message = "") : null), 3000);
  }

  function applySettings(s: Settings) {
    settings = s;
    applyTickets();
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
    applyTickets();
    conflict = data.deleted ? { deleted: true } : null;
    view.setState(EditorState.create({ doc: data.text, extensions: [extensions, NoteSync.extension(data)] }));
    sync = new NoteSync(path, view, load);
    view.dispatch({ selection: { anchor: data.text.length }, scrollIntoView: true });
    view.focus();
    refreshLinks(true);
  }

  let linksTimer: ReturnType<typeof setTimeout> | undefined;
  function refreshLinks(now = false) {
    clearTimeout(linksTimer);
    const run = async () => {
      if (!path) return;
      const text = view.state.doc.toString();
      try {
        const links = await api.noteLinks(path, text);
        if (text !== view.state.doc.toString()) return;
        noteLinks = links;
        refreshDecorations(view);
      } catch {
        // keep the previous list
      }
    };
    if (now) run();
    else linksTimer = setTimeout(run, 150);
  }

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

  function followAt(pos: number): boolean {
    const line = view.state.doc.lineAt(pos);
    const ref = refAt(line.text, pos - line.from, tickets);
    if (!ref) return false;
    if (ref.kind === "url") api.openUrl(ref.target).catch((e) => say(String(e)));
    else api.resolveLink(ref.target, path).then((p) => api.openInMain(p), (e) => say(String(e)));
    return true;
  }

  /** Key highlighting and following: the keys link rules read (and this vault's 0.6 ticket keys). */
  function applyTickets() {
    const re = keyRegex(settings, meta?.vault ?? null);
    tickets = re ? ticketRegex(re.source) : /(?!)/g;
    decoConfig.tickets = tickets;
  }

  async function currentLinks(): Promise<NoteLink[]> {
    if (path) noteLinks = await api.noteLinks(path, view.state.doc.toString()).catch(() => noteLinks);
    return noteLinks;
  }

  function pickLink(links: NoteLink[]) {
    overlay = { type: "links", items: links.map((l) => ({ label: l.primary ? `${l.label} · primary` : l.label, detail: l.url })), urls: links.map((l) => l.url) };
  }

  /** The note's primary link (a 0.6 ticket's ticket), else its only link, else a choice. */
  async function openPrimary() {
    const links = await currentLinks();
    const primary = links.find((l) => l.primary);
    if (primary) api.openUrl(primary.url);
    else if (links.length === 1) api.openUrl(links[0].url);
    else if (links.length) pickLink(links);
    else say("No links in this note yet");
  }

  async function openLink() {
    const links = await currentLinks();
    if (links.length === 1) api.openUrl(links[0].url);
    else if (links.length) pickLink(links);
    else say("No links in this note yet");
  }

  // ---------- the Links panel ----------

  function toggleLinks() {
    linksOpen = !linksOpen;
    if (linksOpen) refreshLinks(true);
    else view.focus();
  }

  function closeLinks() {
    linksOpen = false;
    view.focus();
  }

  async function editLinks(edit: MetaEdit): Promise<boolean> {
    if (!path) return false;
    const change = await api.linkMetaEdit(path, view.state.doc.toString(), edit);
    if (!change) return false;
    view.dispatch({ changes: change, userEvent: "input.links" });
    await sync?.drain();
    refreshLinks(true);
    return true;
  }

  function goToSource(s: LinkSource) {
    linksOpen = false;
    const len = view.state.doc.length;
    view.dispatch({ selection: { anchor: Math.min(s.from, len), head: Math.min(s.to, len) }, effects: EditorView.scrollIntoView(Math.min(s.from, len), { y: "center" }) });
    view.focus();
  }

  function copyLink(url: string) {
    api.writeClipboard(url).then(() => say("Link copied"), (e) => say(String(e)));
  }

  function addLink() {
    overlay = {
      type: "prompt",
      title: "Add a link to this note",
      value: "",
      submit: (value) => {
        overlay = null;
        if (!isUrl(value)) return say("That isn't a link (https://…)");
        editLinks({ op: "add", url: value.trim() }).then((added) => say(added ? "Link added" : "Already there"), (e) => say(String(e)));
      },
    };
  }

  function editAttached(link: NoteLink) {
    overlay = {
      type: "prompt",
      title: "Edit attached link",
      value: link.url,
      submit: (value) => {
        overlay = null;
        if (!isUrl(value)) return say("That isn't a link (https://…)");
        editLinks({ op: "replace", url: [link.url, value.trim()] }).catch((e) => say(String(e)));
      },
    };
  }

  function linkActions(link: NoteLink) {
    const run = (edit: MetaEdit) => () => {
      overlay = null;
      editLinks(edit).then((changed) => changed && say("Done — Ctrl+Z undoes it"), (e) => say(String(e)));
    };
    const items: { label: string; detail?: string; run: () => void }[] = [];
    if (!link.primary) items.push({ label: "Make it the primary link", run: run({ op: "primary", url: link.url }) });
    else if (link.sources.some((s) => s.kind === "primary")) items.push({ label: "Stop using it as the primary link", run: run({ op: "clearPrimary" }) });
    if (link.sources.every((s) => s.kind === "attached")) items.push({ label: "Remove from this note", run: run({ op: "remove", url: link.url }) });
    for (const s of link.sources) items.push({ label: `Go to line ${s.line}`, detail: s.field ?? s.kind, run: () => ((overlay = null), goToSource(s)) });
    overlay = { type: "actions", title: link.label, items };
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

  async function pickTags() {
    if (!meta || meta.kind === "daily") return;
    overlay = { type: "tags", all: await api.listTags().catch(() => []) };
  }

  async function setTags(tags: string[]) {
    if (!path) return;
    try {
      await sync?.drain();
      meta = (await api.setTags(path, tags)).meta;
    } catch (e) {
      say(String(e));
    }
  }

  async function toggleOnTop() {
    onTop = !onTop;
    await api.pinSetOnTop(onTop);
  }

  function closeOverlay() {
    overlay = null;
    view.focus();
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
        closeOverlay();
      }
      return;
    }
    if (linksOpen && e.key === "Escape") {
      stop();
      closeLinks();
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
    if (e.ctrlKey && e.shiftKey && !e.altKey && (e.key === "V" || e.key === "v" || e.code === "KeyV")) {
      // Paste as typed; the browser still performs the paste.
      rawPaste = true;
      setTimeout(() => (rawPaste = false), 1000);
      return;
    }
    const is = (spec: string) => matches(e, parseKey(spec));
    if (is(k.timestamp)) {
      stop();
      view.dispatch(view.state.replaceSelection(timestamp(new Date(), settings.timestampFormat)));
    } else if (is(k.openTicket)) {
      stop();
      openPrimary();
    } else if (is(k.openLink)) {
      stop();
      openLink();
    } else if (is(k.linksPanel)) {
      stop();
      toggleLinks();
    } else if (is(k.addLink)) {
      stop();
      addLink();
    } else if (is(k.setStatus)) {
      stop();
      if (meta?.kind !== "daily") overlay = { type: "status" };
    } else if (is(k.setTags)) {
      stop();
      pickTags();
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
      onChange: () => {
        sync?.push();
        refreshLinks();
      },
      follow: followAt,
      path: () => path,
      openNote: (target) => { api.openInMain(target).catch((e) => say(String(e))); },
      linkSearch: (q) => api.search(q, false, path, 30, meta?.vault ?? null),
      vault: () => meta?.vault ?? null,
      vaultName: (id) => settings?.vaults.find((v) => v.id === id)?.name ?? id,
      paste: smartPaste,
      imageError: say,
    });
    view = new EditorView({ parent: editorEl, state: EditorState.create({ doc: "", extensions }) });
    propertiesHooks.current = {
      statusView: (st) => statusInfo(st, settings?.statuses ?? []),
      onStatus: () => (overlay = { type: "status" }),
      onTags: pickTags,
      // Tag searches happen in the main window.
      onTag: (t) => api.openInMain(`::notes#${t}`),
      tagColor: (t) => settings?.tagColors[t] ?? settings?.tagColors[t.split("/")[0]] ?? null,
      onPrimary: openPrimary,
      onLinks: () => {
        linksOpen = true;
        refreshLinks(true);
      },
      links: () => {
        const p = noteLinks.find((l) => l.primary);
        return { count: noteLinks.length, primary: p ? { label: p.label, icon: p.rule?.icon ?? "external-link" } : null };
      },
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
        if (meta) meta = { ...meta, path: m.to, vault: m.to.slice(0, m.to.indexOf(":")), archived: m.to.slice(m.to.indexOf(":") + 1).startsWith("archive/") };
        conflict = null;
        applyTickets();
        refreshDecorations(view);
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
      view.destroy();
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
    <button class="links-btn" class:on={linksOpen} onclick={toggleLinks} title="This note's links" aria-label="Links ({noteLinks.length})" aria-pressed={linksOpen} disabled={!path}>
      <Icon name="link" size={13} />{noteLinks.length}
    </button>
    <button class="icon" onclick={() => pickImages(view, { path: () => path, imageError: say })} title="Insert image (or paste / drop an image)" aria-label="Insert image" disabled={!path}>
      <Icon name="image-plus" size={14} />
    </button>
    <button class="icon" class:off={!onTop} onclick={toggleOnTop} title={onTop ? "Always on top (click to release)" : "Keep on top"} aria-label="Pin on top">
      <Icon name="pin" size={14} />
    </button>
    <button class="icon" onclick={() => api.pinClose()} title="Close pinned note" aria-label="Close">
      <Icon name="x" size={14} />
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

  {#if linksOpen && settings}
    <div class="panel">
      <LinksPanel
        compact
        scope="note"
        links={noteLinks}
        noteTitle={meta?.title ?? null}
        vaults={settings.vaults}
        keys={settings.keys}
        version={0}
        onScope={() => {}}
        onOpenUrl={(url) => api.openUrl(url).catch((e) => say(String(e)))}
        onCopy={copyLink}
        onGoTo={goToSource}
        onEdit={editAttached}
        onMore={linkActions}
        onAdd={addLink}
        onOpenNote={(p) => api.openInMain(p)}
        onClose={closeLinks}
      />
    </div>
  {/if}

  {#if message}<div class="message">{message}</div>{/if}

  {#if overlay?.type === "status" && settings}
    <StatusPicker statuses={settings.statuses} current={meta?.status ?? null} title="Status" onChoose={setStatus} onClose={closeOverlay} />
  {:else if overlay?.type === "tags" && settings && meta}
    {@const o = overlay}
    <TagPicker
      title="Tags"
      tags={meta.tags}
      all={o.all}
      colors={settings.tagColors}
      onChange={setTags}
      onClose={closeOverlay}
    />
  {:else if overlay?.type === "links"}
    {@const o = overlay}
    <Picker title="Open link" items={o.items} onChoose={(i) => { overlay = null; api.openUrl(o.urls[i]); }} onClose={closeOverlay} />
  {:else if overlay?.type === "actions"}
    {@const o = overlay}
    <Picker title={o.title} items={o.items} onChoose={(i) => o.items[i].run()} onClose={closeOverlay} />
  {:else if overlay?.type === "prompt"}
    {@const o = overlay}
    <Prompt title={o.title} value={o.value} placeholder="https://…" onSubmit={o.submit} onClose={closeOverlay} />
  {/if}
</main>

<style>
  .pinned {
    position: relative;
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
  .links-btn {
    display: flex;
    align-items: center;
    gap: 3px;
    height: 22px;
    padding: 0 5px;
    font: inherit;
    font-size: 11.5px;
    color: var(--fg-muted);
    background: transparent;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }
  .links-btn:hover,
  .links-btn.on {
    background: var(--hover);
    color: var(--fg);
  }
  .panel {
    position: absolute;
    top: 28px;
    left: 0;
    right: 0;
    bottom: 0;
    display: flex;
    z-index: 5;
    background: var(--bg-raised);
  }
  .panel > :global(.links) {
    flex: 1;
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
