<script lang="ts" module>
  export interface NotesViewState {
    query: string;
    sort: import("../lib/api").SortBy;
    grouped: boolean;
    showDaily: boolean;
    showPutAway: boolean;
  }
</script>

<script lang="ts">
  import { onMount } from "svelte";
  import { api, type Keys, type SearchHit, type SortBy, type StatusDef } from "../lib/api";
  import { formatKey } from "../lib/keys";
  import { relTime } from "../lib/text";
  import StatusIcon from "./StatusIcon.svelte";

  interface Props {
    view: NotesViewState;
    statuses: StatusDef[];
    keys: Keys;
    /** Bumped by the app when notes change, to reload the list. */
    version: number;
    isTicket: (q: string) => boolean;
    /** A pasted Jira URL becomes its key. */
    pasteKey: (text: string) => string | null;
    onOpen: (path: string) => void;
    onCreate: (query: string) => void;
  }
  let { view = $bindable(), statuses, keys, version, isTicket, pasteKey, onOpen, onCreate }: Props = $props();

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
    | { type: "note"; hit: SearchHit }
    | { type: "header"; key: string; label: string; count: number; status: string | null }
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

  const text = $derived(view.query.split(/\s+/).filter((w) => w && !w.startsWith("@")).join(" "));
  const filters = $derived(view.query.split(/\s+/).filter((w) => w.length > 1 && w.startsWith("@")));

  async function load(query: string, daily: boolean, putAway: boolean, sort: SortBy) {
    const my = ++seq;
    try {
      const r = await api.listNotes(query, daily, putAway, sort);
      if (my !== seq) return;
      hits = r.hits;
      exact = r.exact;
    } catch {
      // keep the previous rows
    }
  }

  $effect(() => {
    void version;
    load(view.query, view.showDaily, view.showPutAway, view.sort);
  });

  // Reset the selection when the query changes, not when the list reloads.
  $effect(() => {
    void view.query;
    sel = 0;
  });

  function groupKey(h: SearchHit): string {
    return h.status ?? "";
  }

  function groupOrder(key: string): number {
    if (!key) return 1000;
    const i = statuses.findIndex((s) => s.name === key);
    if (i < 0) return 900;
    const d = statuses[i];
    return d.active ? (key === "open" ? 0 : key === "blocked" ? 1 : 2 + i) : 100 + i;
  }

  const rows = $derived.by<Row[]>(() => {
    const out: Row[] = [];
    if (view.grouped && !text) {
      const groups = new Map<string, SearchHit[]>();
      for (const h of hits) {
        const k = groupKey(h);
        if (!groups.has(k)) groups.set(k, []);
        groups.get(k)!.push(h);
      }
      for (const k of [...groups.keys()].sort((a, b) => groupOrder(a) - groupOrder(b))) {
        const items = groups.get(k)!;
        out.push({ type: "header", key: k, label: k || "no status", count: items.length, status: k || null });
        if (!collapsed.has(k)) for (const h of items) out.push({ type: "note", hit: h });
      }
    } else {
      for (const h of hits) out.push({ type: "note", hit: h });
    }
    if (text.trim() && !exact) out.push({ type: "create", query: view.query.trim() });
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
    } else if (k === "Tab") {
      e.preventDefault();
      view.showPutAway = !view.showPutAway;
    } else if (e.altKey && !e.ctrlKey && (k === "s" || k === "S" || e.code === "KeyS")) {
      e.preventDefault();
      cycleSort();
    } else if (e.altKey && !e.ctrlKey && (k === "g" || k === "G" || e.code === "KeyG")) {
      e.preventDefault();
      view.grouped = !view.grouped;
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
  <div class="chips">
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
    <button class="chip" class:on={view.grouped} onclick={() => (view.grouped = !view.grouped)} title="Alt+G">
      {view.grouped ? "Grouped by status" : "Not grouped"}
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
      {#each rows.slice(first, last) as row, j (row.type === "note" ? row.hit.path : row.type === "header" ? `h:${row.key}` : "create")}
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
          onkeydown={() => {}}
        >
          {#if row.type === "note"}
            <span class="sym"><StatusIcon status={row.hit.status} {statuses} /></span>
            <span class="main">
              <span class="line1">
                <span class="name">{name(row.hit)}</span>
                {#if row.hit.kind === "ticket" && row.hit.label}<span class="label">{row.hit.label}</span>{/if}
                {#if row.hit.kind === "daily"}<span class="tag">daily</span>{/if}
                {#if row.hit.archived}<span class="tag">archived</span>{/if}
              </span>
              <span class="snippet">{row.hit.snippet ?? ""}</span>
            </span>
            <span class="time" title={new Date(row.hit.mtime).toLocaleString()}>{relTime(row.hit.mtime)}</span>
          {:else if row.type === "header"}
            <span class="caret">{collapsed.has(row.key) ? "▸" : "▾"}</span>
            {#if row.status}<StatusIcon status={row.status} {statuses} />{/if}
            <span class="hname">{row.label}</span>
            <span class="hcount">{row.count}</span>
          {:else}
            <span class="sym new">+</span>
            <span class="main"><span class="name">Create "{row.query}"</span></span>
            <span class="label">{isTicket(row.query) ? "ticket note" : "note"}</span>
          {/if}
        </div>
      {/each}
    </div>
    {#if rows.length === 0}
      <div class="empty">
        {view.query ? "No matching notes" : "No notes to show"} — {formatKey(keys.quickSwitcher)} to create one
      </div>
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
    padding: 12px 16px 6px;
  }
  input {
    width: 100%;
    font: inherit;
    font-size: 15px;
    color: var(--fg);
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px 12px;
    outline: none;
  }
  input:focus {
    border-color: var(--accent);
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
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--fg-muted);
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
    align-items: baseline;
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
    width: 1em;
  }
  .hname {
    font-weight: 600;
  }
  .hcount {
    color: var(--fg-muted);
  }
  .empty {
    padding: 24px 16px;
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
