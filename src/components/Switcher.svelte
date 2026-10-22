<script lang="ts">
  import { dim, pop } from "../lib/motion";
  import { api, type SearchHit, type StatusDef, type TagCount } from "../lib/api";
  import { completeTag, tagSuggestions, typingTag } from "../lib/text";
  import TagChip from "./TagChip.svelte";
  import EmptyState from "./EmptyState.svelte";
  import Highlight from "./Highlight.svelte";
  import Icon from "./Icon.svelte";
  import StatusIcon from "./StatusIcon.svelte";
  import { onMount } from "svelte";

  interface Props {
    /** "open" navigates; "link" turns the editor selection into a link. */
    mode: "open" | "link";
    current: string | null;
    isTicket: (q: string) => boolean;
    statuses: StatusDef[];
    tagColors: Record<string, string>;
    /** A pasted Jira URL becomes its key. */
    pasteKey: (text: string) => string | null;
    onChoose: (hit: SearchHit, peek: boolean) => void;
    onCreate: (query: string) => void;
    /** Fill a window of its own (the Alt+J pop-up) instead of floating over the app. */
    embedded?: boolean;
    /** Embedded only: Esc was pressed. */
    onClose?: () => void;
    /** Embedded only: the content's height changed. */
    onSize?: (height: number) => void;
  }
  let { mode, current, isTicket, statuses, tagColors, pasteKey, onChoose, onCreate, embedded = false, onClose, onSize }: Props =
    $props();
  let box: HTMLDivElement;

  $effect(() => {
    if (!embedded || !onSize || !box) return;
    const ro = new ResizeObserver(() => onSize(Math.ceil(box.getBoundingClientRect().height)));
    ro.observe(box);
    return () => ro.disconnect();
  });

  function paste(e: ClipboardEvent) {
    const key = pasteKey(e.clipboardData?.getData("text/plain") ?? "");
    if (key) {
      e.preventDefault();
      // Leave room to type the ticket's name: `PROJ-1234 Do this thing`.
      query = `${key} `;
    }
  }

  let query = $state("");
  let hits = $state<SearchHit[]>([]);
  let exact = $state(true);
  let sel = $state(0);
  let includeArchived = $state(false);
  let input: HTMLInputElement;
  let list: HTMLDivElement;
  let seq = 0;
  let allTags = $state<TagCount[]>([]);
  api.listTags().then((t) => (allTags = t), () => {});
  const suggestions = $derived(tagSuggestions(query, allTags));
  /** What's searched: a tag still being typed (with suggestions showing) doesn't filter yet. */
  const searched = $derived(
    suggestions.length && !allTags.some((t) => t.name === typingTag(query)) ? query.replace(/#[\p{L}\p{N}_/-]*$/u, "") : query,
  );
  const isFilter = (w: string) => w.length > 1 && (w.startsWith("@") || w.startsWith("#") || w.startsWith("in:"));
  /** The query without filter words: what a new note would be called. */
  const title = $derived(query.split(/\s+/).filter((w) => w && !isFilter(w)).join(" "));

  const showCreate = $derived(title !== "" && !exact);
  const newTags = $derived(query.split(/\s+/).filter((w) => w.length > 1 && w.startsWith("#")).map((w) => w.slice(1).toLowerCase()));
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
    run(searched, includeArchived);
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
    if (e.key === "Escape" && embedded) {
      e.preventDefault();
      onClose?.();
    } else if (e.key === "ArrowDown" || (e.key === "n" && e.ctrlKey)) {
      e.preventDefault();
      sel = rowCount ? (sel + 1) % rowCount : 0;
    } else if (e.key === "ArrowUp" || (e.key === "p" && e.ctrlKey)) {
      e.preventDefault();
      sel = rowCount ? (sel - 1 + rowCount) % rowCount : 0;
    } else if (e.key === "Enter") {
      e.preventDefault();
      choose(sel, e.altKey);
    } else if (e.key === "Tab" && suggestions.length) {
      e.preventDefault();
      query = completeTag(query, suggestions[0]);
    } else if (e.key === "Tab") {
      e.preventDefault();
      includeArchived = !includeArchived;
    }
  }
</script>

<div class="backdrop" class:embedded role="presentation" in:dim>
  <div class="switcher" in:pop bind:this={box} role="dialog" aria-label={mode === "link" ? "Link to note" : "Quick switcher"}>
    <div class="inputrow">
      <span class="lead"><Icon name="search" size={16} /></span>
      <input
        bind:this={input}
        bind:value={query}
        onkeydown={keydown}
        onpaste={paste}
        placeholder={mode === "link" ? "Link selection to…" : "Find or create a note — key, title, date, text or @status"}
        spellcheck="false"
        autocomplete="off"
      />
      {#if includeArchived}<span class="flag">+ archived</span>{/if}
    </div>
    {#if suggestions.length}
      <div class="suggest">
        <span class="shint"><kbd>Tab</kbd></span>
        {#each suggestions as t}<TagChip tag={t} colors={tagColors} onclick={() => { query = completeTag(query, t); input.focus(); }} />{/each}
      </div>
    {/if}
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
            <span class="name"><Highlight text={hit.title} {query} /></span>
            {#if hit.label}<span class="label"><Highlight text={hit.label} {query} /></span>{/if}
            {#each hit.tags as t}<TagChip tag={t} colors={tagColors} />{/each}
            {#if hit.archived}<span class="tag">archived</span>{/if}
          </span>
          {#if hit.snippet}<span class="snippet"><Highlight text={hit.snippet} {query} /></span>{/if}
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
            <span class="sym new"><Icon name="plus" size={14} /></span>
            {#if isTicket(title)}
              {@const [key, ...name] = title.split(/\s+/)}
              <span class="name">Create {key.toUpperCase()}</span>
              <span class="label">{name.length ? `"${name.join(" ")}"` : "ticket note"}</span>
            {:else}
              <span class="name">Create "{title}"</span>
              <span class="label">note</span>
            {/if}
            {#each newTags as t}<TagChip tag={t} colors={tagColors} />{/each}
          </span>
        </button>
      {/if}
      {#if rowCount === 0}
        {#if query}
          <EmptyState icon="search-x" title="No matches" />
        {:else}
          <EmptyState icon="file-plus" title="No notes yet">Type a name and press <kbd>Enter</kbd> to create one.</EmptyState>
        {/if}
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
  .backdrop.embedded {
    position: static;
    padding: 0;
    background: none;
    display: block;
  }
  .embedded .switcher {
    width: 100%;
    max-height: none;
    border-radius: 0;
    border: none;
    box-shadow: none;
  }
  .embedded .results {
    max-height: 380px;
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
  .lead {
    display: flex;
    padding-left: 14px;
    color: var(--fg-muted);
  }
  input {
    flex: 1;
    font: inherit;
    font-size: 15px;
    padding: 12px 14px 12px 10px;
    border: none;
    outline: none;
    background: transparent;
    color: var(--fg);
  }
  .suggest {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    padding: 6px 14px;
    border-bottom: 1px solid var(--border);
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
    display: inline-flex;
    width: 16px;
    flex-shrink: 0;
    margin-left: -4px;
  }
  .sym.new {
    color: var(--ok);
  }
  .snippet {
    padding-left: 70px;
    font-size: 12.5px;
    color: var(--fg-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
