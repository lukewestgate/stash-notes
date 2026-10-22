<script lang="ts">
  import { api, type SearchHit, type StatusDef } from "../lib/api";
  import StatusIcon from "./StatusIcon.svelte";
  import { onMount } from "svelte";

  interface Props {
    /** "open" navigates; "link" turns the editor selection into a link. */
    mode: "open" | "link";
    current: string | null;
    isTicket: (q: string) => boolean;
    statuses: StatusDef[];
    onChoose: (hit: SearchHit, peek: boolean) => void;
    onCreate: (query: string) => void;
  }
  let { mode, current, isTicket, statuses, onChoose, onCreate }: Props = $props();

  let query = $state("");
  let hits = $state<SearchHit[]>([]);
  let exact = $state(true);
  let sel = $state(0);
  let includeArchived = $state(false);
  let input: HTMLInputElement;
  let list: HTMLDivElement;
  let seq = 0;

  const showCreate = $derived(query.trim() !== "" && !exact);
  const rowCount = $derived(hits.length + (showCreate ? 1 : 0));

  async function run(q: string, archived: boolean) {
    const my = ++seq;
    try {
      const r = await api.search(q, archived, current);
      if (my !== seq) return;
      hits = r.hits;
      exact = r.exact;
      sel = 0;
    } catch {
      // keep previous results
    }
  }

  $effect(() => {
    run(query, includeArchived);
  });

  $effect(() => {
    list?.querySelector(`[data-row="${sel}"]`)?.scrollIntoView({ block: "nearest" });
  });

  onMount(() => input.focus());

  function choose(i: number, peek: boolean) {
    if (i < hits.length) onChoose(hits[i], peek);
    else if (showCreate) onCreate(query.trim());
  }

  function keydown(e: KeyboardEvent) {
    if (e.key === "ArrowDown" || (e.key === "n" && e.ctrlKey)) {
      e.preventDefault();
      sel = rowCount ? (sel + 1) % rowCount : 0;
    } else if (e.key === "ArrowUp" || (e.key === "p" && e.ctrlKey)) {
      e.preventDefault();
      sel = rowCount ? (sel - 1 + rowCount) % rowCount : 0;
    } else if (e.key === "Enter") {
      e.preventDefault();
      choose(sel, e.altKey);
    } else if (e.key === "Tab") {
      e.preventDefault();
      includeArchived = !includeArchived;
    }
  }
</script>

<div class="backdrop" role="presentation">
  <div class="switcher" role="dialog" aria-label={mode === "link" ? "Link to note" : "Quick switcher"}>
    <div class="inputrow">
      <input
        bind:this={input}
        bind:value={query}
        onkeydown={keydown}
        placeholder={mode === "link" ? "Link selection to…" : "Find or create a note — key, title, date, text or @status"}
        spellcheck="false"
        autocomplete="off"
      />
      {#if includeArchived}<span class="flag">+ archived</span>{/if}
    </div>
    <div class="results" bind:this={list}>
      {#each hits as hit, i (hit.path)}
        <button
          class="row"
          class:selected={i === sel}
          class:archived={hit.archived || hit.status === "closed"}
          data-row={i}
          onmousemove={() => (sel = i)}
          onclick={(e) => choose(i, e.altKey)}
        >
          <span class="line">
            <span class="kind {hit.kind}">{hit.kind}</span>
            <span class="sym"><StatusIcon status={hit.status} {statuses} /></span>
            <span class="name">{hit.title}</span>
            {#if hit.label}<span class="label">{hit.label}</span>{/if}
            {#if hit.archived}<span class="tag">archived</span>{/if}
          </span>
          {#if hit.snippet}<span class="snippet">{hit.snippet}</span>{/if}
        </button>
      {/each}
      {#if showCreate}
        <button
          class="row create"
          class:selected={sel === hits.length}
          data-row={hits.length}
          onmousemove={() => (sel = hits.length)}
          onclick={() => choose(hits.length, false)}
        >
          <span class="line">
            <span class="kind new">new</span>
            <span class="sym"></span>
            <span class="name">Create "{query.trim()}"</span>
            <span class="label">{isTicket(query.trim()) ? "ticket note" : "note"}</span>
          </span>
        </button>
      {/if}
      {#if rowCount === 0}
        <div class="empty">{query ? "No matches" : "No notes yet — type a name to create one"}</div>
      {/if}
    </div>
    <div class="hints">
      <span><kbd>↑</kbd><kbd>↓</kbd> move</span>
      <span><kbd>Enter</kbd> {mode === "link" ? "link" : "open"}</span>
      {#if mode === "open"}<span><kbd>Alt+Enter</kbd> peek</span>{/if}
      <span><kbd>Tab</kbd> {includeArchived ? "hide" : "include"} closed & archived</span>
      <span><kbd>Esc</kbd> close</span>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 9vh;
    background: var(--scrim);
    z-index: 20;
  }
  .switcher {
    width: min(640px, calc(100vw - 32px));
    max-height: 72vh;
    display: flex;
    flex-direction: column;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: var(--shadow);
    overflow: hidden;
  }
  .inputrow {
    display: flex;
    align-items: center;
    border-bottom: 1px solid var(--border);
  }
  input {
    flex: 1;
    font: inherit;
    font-size: 15px;
    padding: 12px 14px;
    border: none;
    outline: none;
    background: transparent;
    color: var(--fg);
  }
  .flag {
    font-size: 12px;
    color: var(--fg-muted);
    padding-right: 12px;
  }
  .results {
    overflow-y: auto;
    padding: 4px;
  }
  .row {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 100%;
    text-align: left;
    font: inherit;
    color: var(--fg);
    background: transparent;
    border: none;
    border-radius: 6px;
    padding: 6px 10px;
    cursor: pointer;
  }
  .row.selected {
    background: var(--selected);
  }
  .row.archived {
    opacity: 0.55;
  }
  .line {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
    white-space: nowrap;
  }
  .name {
    font-weight: 600;
  }
  .label {
    color: var(--fg-muted);
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .kind {
    font-size: 10.5px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    width: 44px;
    flex-shrink: 0;
    color: var(--fg-muted);
  }
  .kind.ticket {
    color: var(--ticket);
  }
  .kind.daily {
    color: var(--accent);
  }
  .kind.new {
    color: var(--ok);
  }
  .tag {
    font-size: 11px;
    color: var(--fg-muted);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0 5px;
  }
  .sym {
    width: 1.1em;
    flex-shrink: 0;
    margin-left: -4px;
  }
  .snippet {
    padding-left: 70px;
    font-size: 12.5px;
    color: var(--fg-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .empty {
    padding: 14px;
    color: var(--fg-muted);
  }
  .hints {
    display: flex;
    flex-wrap: wrap;
    gap: 14px;
    padding: 7px 12px;
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
