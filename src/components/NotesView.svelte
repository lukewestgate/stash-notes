<script lang="ts" module>
  export interface NotesViewState {
    query: string;
    sort: import("../lib/api").SortBy;
    group: "none" | "status" | "tag";
    showDaily: boolean;
    showPutAway: boolean;
  }
</script>

<script lang="ts">
  import { onMount } from "svelte";
  import { api, type Keys, type SearchHit, type SortBy, type StatusDef, type TagCount, type VaultDef } from "../lib/api";
  import VaultTag from "./VaultTag.svelte";
  import { formatKey } from "../lib/keys";
  import { anchorAt, anchorOf, type Anchor } from "../lib/popup";
  import { completeTag, relTime, tagSuggestions, typingTag } from "../lib/text";
  import TagChip from "./TagChip.svelte";
  import EmptyState from "./EmptyState.svelte";
  import Highlight from "./Highlight.svelte";
  import Icon from "./Icon.svelte";
  import StatusIcon from "./StatusIcon.svelte";

  interface Props {
    view: NotesViewState;
    statuses: StatusDef[];
    tagColors: Record<string, string>;
    /** The vault listed (unless the query says `in:` others). */
    vault: string;
    vaults: VaultDef[];
    keys: Keys;
    /** Bumped by the app when notes change, to reload the list. */
    version: number;
    isTicket: (q: string) => boolean;
    /** A pasted ticket URL becomes its key. */
    pasteKey: (text: string) => string | null;
    onOpen: (path: string) => void;
    onCreate: (query: string) => void;
    /** Open the note menu for a row (right-click, Shift+F10 or the menu key). */
    onMenu: (hit: SearchHit, anchor: Anchor) => void;
  }
  let { view = $bindable(), statuses, tagColors, vault, vaults, keys, version, isTicket, pasteKey, onOpen, onCreate, onMenu }: Props = $props();

  let allTags = $state<TagCount[]>([]);
  $effect(() => {
    void version;
    api.listTags().then((t) => (allTags = t), () => {});
  });
  const suggestions = $derived(tagSuggestions(view.query, allTags));
  /** What's searched: a tag still being typed (with suggestions showing) doesn't filter yet. */
  const searched = $derived(
    suggestions.length && !allTags.some((t) => t.name === typingTag(view.query)) ? view.query.replace(/#[\p{L}\p{N}_/-]*$/u, "") : view.query,
  );

  function complete(tag: string) {
    view.query = completeTag(view.query, tag);
    input?.focus();
  }

  function paste(e: ClipboardEvent) {
    const key = pasteKey(e.clipboardData?.getData("text/plain") ?? "");
    if (key) {
      e.preventDefault();
      // Leave room to type the ticket's name: `PROJ-1234 Do this thing`.
      view.query = `${key} `;
    }
  }

  const ROW = 44;
  const SORTS: SortBy[] = ["edited", "opened", "status", "title"];
  const SORT_LABEL: Record<SortBy, string> = {
    edited: "last edited",
    opened: "last opened",
    status: "status",
    title: "title",
  };

  type Row =
    | { type: "note"; hit: SearchHit; group?: string }
    | { type: "header"; key: string; label: string; count: number; status: string | null; tag: string | null }
    | { type: "create"; query: string };

  let hits = $state<SearchHit[]>([]);
  let exact = $state(true);
  let sel = $state(0);
  let collapsed = $state(new Set<string>());
  let scrollTop = $state(0);
  let height = $state(600);
  let input: HTMLInputElement;
  let list: HTMLDivElement;
  let seq = 0;

  const isFilter = (w: string) => w.length > 1 && (w.startsWith("@") || w.startsWith("#") || w.startsWith("in:"));
  const text = $derived(view.query.split(/\s+/).filter((w) => w && !isFilter(w)).join(" "));
  const filters = $derived(view.query.split(/\s+/).filter(isFilter));

  async function load(query: string, daily: boolean, putAway: boolean, sort: SortBy, vault: string) {
    const my = ++seq;
    try {
      const r = await api.listNotes(query, daily, putAway, sort, vault);
      if (my !== seq) return;
      hits = r.hits;
      exact = r.exact;
    } catch {
      // keep the previous rows
    }
  }

  $effect(() => {
    void version;
    load(searched, view.showDaily, view.showPutAway, view.sort, vault);
  });

  // Reset the selection when the query changes, not when the list reloads.
  $effect(() => {
    void view.query;
    sel = 0;
  });

  /** The groups a note belongs to: its status, or each of its tags. */
  function groupKeys(h: SearchHit): string[] {
    if (view.group === "tag") return h.tags.length ? h.tags : [""];
    return [h.status ?? ""];
  }

  function groupOrder(key: string, counts: Map<string, number>): number {
    if (view.group === "tag") return key ? -(counts.get(key) ?? 0) : 1e9;
    if (!key) return 1000;
    const i = statuses.findIndex((s) => s.name === key);
    if (i < 0) return 900;
    const d = statuses[i];
    return d.active ? (key === "open" ? 0 : key === "blocked" ? 1 : 2 + i) : 100 + i;
  }

  const rows = $derived.by<Row[]>(() => {
    const out: Row[] = [];
    if (view.group !== "none" && !text) {
      const groups = new Map<string, SearchHit[]>();
      for (const h of hits) {
        for (const k of groupKeys(h)) {
          if (!groups.has(k)) groups.set(k, []);
          groups.get(k)!.push(h);
        }
      }
      const counts = new Map([...groups].map(([k, v]) => [k, v.length]));
      const byTag = view.group === "tag";
      const keys = [...groups.keys()].sort((a, b) => groupOrder(a, counts) - groupOrder(b, counts) || a.localeCompare(b));
      for (const k of keys) {
        const items = groups.get(k)!;
        out.push({
          type: "header",
          key: k,
          label: k ? (byTag ? `#${k}` : k) : byTag ? "no tags" : "no status",
          count: items.length,
          status: !byTag && k ? k : null,
          tag: byTag && k ? k : null,
        });
        if (!collapsed.has(k)) for (const h of items) out.push({ type: "note", hit: h, group: k });
      }
    } else {
      for (const h of hits) out.push({ type: "note", hit: h });
    }
    if (text.trim() && !exact) out.push({ type: "create", query: view.query.trim() });
    // (Rust leaves filter words out of the new note's title and gives it the #tags.)
    return out;
  });

  const first = $derived(Math.max(0, Math.floor(scrollTop / ROW) - 8));
  const last = $derived(Math.min(rows.length, Math.ceil((scrollTop + height) / ROW) + 8));

  $effect(() => {
    if (sel >= rows.length) sel = Math.max(0, rows.length - 1);
  });

  function reveal(i: number) {
    if (!list) return;
    const top = i * ROW;
    if (top < list.scrollTop) list.scrollTop = top;
    else if (top + ROW > list.scrollTop + list.clientHeight) list.scrollTop = top + ROW - list.clientHeight;
  }

  onMount(() => input.focus());

  /** The row the status shortcut and Enter act on. */
  export function selected(): SearchHit | null {
    const r = rows[sel];
    return r?.type === "note" ? r.hit : null;
  }

  export function focus() {
    input?.focus();
  }

  function activate(i: number) {
    const r = rows[i];
    if (!r) return;
    if (r.type === "note") onOpen(r.hit.path);
    else if (r.type === "create") onCreate(r.query);
    else toggleGroup(r.key);
  }

  function toggleGroup(key: string, open?: boolean) {
    const next = new Set(collapsed);
    const isOpen = !next.has(key);
    if (open ?? !isOpen) next.delete(key);
    else next.add(key);
    collapsed = next;
  }

  const GROUPS = ["none", "status", "tag"] as const;
  const GROUP_LABEL = { none: "Not grouped", status: "Grouped by status", tag: "Grouped by tag" };

  function cycleGroup() {
    view.group = GROUPS[(GROUPS.indexOf(view.group) + 1) % GROUPS.length];
  }

  function cycleSort() {
    view.sort = SORTS[(SORTS.indexOf(view.sort) + 1) % SORTS.length];
  }

  function keydown(e: KeyboardEvent) {
    const k = e.key;
    if (k === "ArrowDown") {
      e.preventDefault();
      sel = Math.min(sel + 1, rows.length - 1);
      reveal(sel);
    } else if (k === "ArrowUp") {
      e.preventDefault();
      sel = Math.max(sel - 1, 0);
      reveal(sel);
    } else if (k === "PageDown" || k === "PageUp") {
      e.preventDefault();
      const step = Math.max(1, Math.floor(height / ROW) - 1);
      sel = Math.max(0, Math.min(rows.length - 1, sel + (k === "PageDown" ? step : -step)));
      reveal(sel);
    } else if (k === "Enter") {
      e.preventDefault();
      activate(sel);
    } else if (k === "ContextMenu" || (e.shiftKey && k === "F10")) {
      const row = rows[sel];
      const el = list?.querySelector<HTMLElement>(".row.selected");
      if (row?.type === "note" && el) {
        e.preventDefault();
        onMenu(row.hit, anchorOf(el));
      }
    } else if (k === "Tab" && suggestions.length) {
      e.preventDefault();
      complete(suggestions[0]);
    } else if (k === "Tab") {
      e.preventDefault();
      view.showPutAway = !view.showPutAway;
    } else if (e.altKey && !e.ctrlKey && (k === "s" || k === "S" || e.code === "KeyS")) {
      e.preventDefault();
      cycleSort();
    } else if (e.altKey && !e.ctrlKey && (k === "g" || k === "G" || e.code === "KeyG")) {
      e.preventDefault();
      cycleGroup();
    } else if (e.altKey && !e.ctrlKey && (k === "d" || k === "D" || e.code === "KeyD")) {
      e.preventDefault();
      view.showDaily = !view.showDaily;
    } else if ((k === "ArrowLeft" || k === "ArrowRight") && rows[sel]?.type === "header") {
      e.preventDefault();
      toggleGroup((rows[sel] as { key: string }).key, k === "ArrowRight");
    }
  }

  function name(h: SearchHit) {
    return h.kind === "ticket" && h.key ? h.key : h.title;
  }
</script>

<section class="notes" aria-label="Notes">
  <div class="bar">
    <span class="lead"><Icon name="search" size={16} /></span>
    <input
      bind:this={input}
      bind:value={view.query}
      onkeydown={keydown}
      onpaste={paste}
      placeholder="Search notes — key, title, date, text or @status"
      spellcheck="false"
      autocomplete="off"
    />
  </div>
  {#if suggestions.length}
    <div class="suggest" aria-label="Tag suggestions">
      <span class="shint"><kbd>Tab</kbd></span>
      {#each suggestions as t}<TagChip tag={t} colors={tagColors} onclick={() => complete(t)} />{/each}
    </div>
  {/if}
  <div class="chips">
    {#if vaults.length > 1 && !filters.some((f) => f.startsWith("in:"))}
      <span class="chip vault" title="Notes in this vault; type in:name for another">
        <VaultTag {vault} {vaults} />
      </span>
    {/if}
    {#each filters as f}<span class="chip on">{f}</span>{/each}
    <button class="chip" class:on={view.showDaily} onclick={() => (view.showDaily = !view.showDaily)} title="Alt+D">
      {view.showDaily ? "Daily notes shown" : "Daily notes hidden"}
    </button>
    <button class="chip" class:on={view.showPutAway} onclick={() => (view.showPutAway = !view.showPutAway)} title="Tab">
      {view.showPutAway ? "Closed & archived shown" : "Closed hidden"}
    </button>
    <button class="chip" onclick={cycleSort} title="Alt+S">
      {text ? "Sorted by relevance" : `Sorted by ${SORT_LABEL[view.sort]}`}
    </button>
    <button class="chip" class:on={view.group !== "none"} onclick={cycleGroup} title="Alt+G">
      {GROUP_LABEL[view.group]}
    </button>
    <span class="count">{hits.length} {hits.length === 1 ? "note" : "notes"}</span>
  </div>
  <div
    class="list"
    bind:this={list}
    bind:clientHeight={height}
    onscroll={() => (scrollTop = list.scrollTop)}
    role="listbox"
    tabindex="-1"
  >
    <div class="spacer" style="height: {rows.length * ROW}px">
      {#each rows.slice(first, last) as row, j (row.type === "note" ? `${row.group ?? ""}:${row.hit.path}` : row.type === "header" ? `h:${row.key}` : "create")}
        {@const i = first + j}
        <div
          class="row {row.type}"
          class:selected={i === sel}
          class:dim={row.type === "note" && (row.hit.archived || row.hit.status === "closed")}
          style="top: {i * ROW}px"
          role="option"
          aria-selected={i === sel}
          tabindex="-1"
          onmousemove={() => (sel = i)}
          onclick={() => activate(i)}
          oncontextmenu={(e) => {
            if (row.type !== "note") return;
            e.preventDefault();
            sel = i;
            onMenu(row.hit, anchorAt(e));
          }}
          onkeydown={() => {}}
        >
          {#if row.type === "note"}
            <span class="sym"><StatusIcon status={row.hit.status} {statuses} /></span>
            <span class="main">
              <span class="line1">
                <span class="name"><Highlight text={name(row.hit)} query={view.query} /></span>
                {#if row.hit.kind === "ticket" && row.hit.label}<span class="label"><Highlight text={row.hit.label} query={view.query} /></span>{/if}
                {#each row.hit.tags.filter((t) => t !== row.group) as t}<TagChip tag={t} colors={tagColors} />{/each}
                {#if row.hit.kind === "daily"}<span class="tag">daily</span>{/if}
                {#if row.hit.archived}<span class="tag">archived</span>{/if}
                {#if row.hit.vault !== vault}<VaultTag vault={row.hit.vault} {vaults} />{/if}
              </span>
              <span class="snippet"><Highlight text={row.hit.snippet ?? ""} query={view.query} /></span>
            </span>
            <span class="time" title={new Date(row.hit.mtime).toLocaleString()}>{relTime(row.hit.mtime)}</span>
          {:else if row.type === "header"}
            <span class="caret" class:closed={collapsed.has(row.key)}><Icon name="chevron-down" size={14} /></span>
            {#if row.status}<StatusIcon status={row.status} {statuses} />{/if}
            {#if row.tag}<TagChip tag={row.tag} colors={tagColors} />{:else}<span class="hname">{row.label}</span>{/if}
            <span class="hcount">{row.count}</span>
          {:else}
            <span class="sym new"><Icon name="plus" size={14} /></span>
            <span class="main"><span class="name">Create "{row.query}"</span></span>
            <span class="label">{isTicket(row.query) ? "ticket note" : "note"}</span>
          {/if}
        </div>
      {/each}
    </div>
    {#if rows.length === 0}
      {#if view.query}
        <EmptyState icon="search-x" title="No notes match “{view.query.trim()}”">
          {#if !view.showPutAway || !view.showDaily}
            Closed, archived or daily notes may be hidden —
            {#if !view.showPutAway}<kbd>Tab</kbd> shows closed{/if}{#if !view.showPutAway && !view.showDaily}, {/if}{#if !view.showDaily}<kbd>Alt+D</kbd> shows daily{/if}.
          {/if}
        </EmptyState>
      {:else}
        <EmptyState icon="inbox" title="No notes to show">
          Press <kbd>{formatKey(keys.quickSwitcher)}</kbd> and type a name to create one, or <kbd>{formatKey(keys.dailyNote)}</kbd> for
          today's note.
        </EmptyState>
      {/if}
    {/if}
  </div>
  <div class="hints">
    <span><kbd>↑</kbd><kbd>↓</kbd> move</span>
    <span><kbd>Enter</kbd> open</span>
    <span><kbd>{formatKey(keys.setStatus)}</kbd> status</span>
    <span><kbd>Alt+S</kbd> sort</span>
    <span><kbd>Alt+G</kbd> group</span>
    <span><kbd>Alt+D</kbd> daily</span>
    <span><kbd>Tab</kbd> closed</span>
    <span><kbd>Esc</kbd> back</span>
  </div>
</section>

<style>
  .notes {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg);
  }
  .bar {
    position: relative;
    padding: 12px 16px 6px;
  }
  .lead {
    position: absolute;
    left: 28px;
    top: 21px;
    display: flex;
    color: var(--fg-muted);
    pointer-events: none;
  }
  input {
    width: 100%;
    font: inherit;
    font-size: 15px;
    color: var(--fg);
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px 12px 8px 36px;
    outline: none;
  }
  input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 20%, transparent);
  }
  .suggest {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    padding: 0 16px 4px;
  }
  .shint {
    font-size: 11px;
    color: var(--fg-muted);
  }
  .shint kbd {
    font: inherit;
    padding: 0 4px;
    border: 1px solid var(--border);
    border-radius: 3px;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    padding: 2px 16px 8px;
    font-size: 12px;
  }
  .chip {
    font: inherit;
    color: var(--fg-muted);
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 1px 9px;
    cursor: pointer;
  }
  .chip.on {
    color: var(--fg);
    background: var(--selected);
    border-color: transparent;
  }
  .count {
    margin-left: auto;
    color: var(--fg-muted);
  }
  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    outline: none;
    border-top: 1px solid var(--border);
  }
  .spacer {
    position: relative;
  }
  .row {
    position: absolute;
    left: 0;
    right: 0;
    height: 44px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 16px;
    cursor: pointer;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 50%, transparent);
  }
  .row.selected {
    background: var(--selected);
  }
  .row.dim {
    opacity: 0.55;
  }
  .row.header {
    background: var(--bg-raised);
    font-size: 12px;
    color: var(--fg-muted);
  }
  .hname {
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .row.header.selected {
    background: var(--selected);
  }
  .sym {
    width: 1.2em;
    flex-shrink: 0;
    text-align: center;
  }
  .sym.new {
    color: var(--ok);
    font-weight: 700;
  }
  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    line-height: 1.3;
  }
  .line1 {
    display: flex;
    gap: 8px;
    align-items: center;
    white-space: nowrap;
    overflow: hidden;
  }
  .name {
    font-weight: 600;
  }
  .label {
    color: var(--fg-muted);
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tag {
    font-size: 10.5px;
    color: var(--fg-muted);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0 4px;
  }
  .snippet {
    font-size: 12.5px;
    color: var(--fg-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .time {
    flex-shrink: 0;
    font-size: 12px;
    color: var(--fg-muted);
    font-variant-numeric: tabular-nums;
  }
  .caret {
    display: inline-flex;
    color: var(--fg-muted);
    transition: transform var(--motion-fast) ease;
  }
  .caret.closed {
    transform: rotate(-90deg);
  }
  .hname {
    font-weight: 600;
  }
  .hcount {
    color: var(--fg-muted);
  }
  .hints {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    padding: 6px 16px;
    border-top: 1px solid var(--border);
    font-size: 11.5px;
    color: var(--fg-muted);
  }
  kbd {
    font: inherit;
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 0 4px;
    margin-right: 2px;
  }
</style>
