<script lang="ts">
  // The Links panel: every external link of the open note (This note), or of
  // every vault (All notes). Both are views of the notes themselves; nothing
  // here is stored apart from them.
  import { onMount } from "svelte";
  import { api, type Keys, type LinkSource, type LinkUse, type NoteLink, type VaultDef } from "../lib/api";
  import { formatKey } from "../lib/keys";
  import EmptyState from "./EmptyState.svelte";
  import Icon from "./Icon.svelte";
  import VaultTag from "./VaultTag.svelte";

  interface Props {
    scope: "note" | "all";
    /** This note's links (primary first). */
    links: NoteLink[];
    /** The open note's title, or null when no note is open. */
    noteTitle: string | null;
    vaults: VaultDef[];
    keys: Keys;
    /** Bumped when notes change, to reload All notes. */
    version: number;
    /** A pinned window: This note only. */
    compact?: boolean;
    onScope: (scope: "note" | "all") => void;
    onOpenUrl: (url: string) => void;
    onCopy: (url: string) => void;
    onGoTo: (source: LinkSource) => void;
    onEdit: (link: NoteLink) => void;
    /** More actions for a link: primary, remove, each source. */
    onMore: (link: NoteLink) => void;
    onAdd: () => void;
    onOpenNote: (path: string) => void;
    onClose: () => void;
  }
  let { scope, links, noteTitle, vaults, keys, version, compact = false, onScope, onOpenUrl, onCopy, onGoTo, onEdit, onMore, onAdd, onOpenNote, onClose }: Props = $props();

  let query = $state("");
  let vault = $state("");
  let includeArchived = $state(false);
  let all = $state<LinkUse[]>([]);
  let loading = $state(false);
  let root: HTMLElement;
  let seq = 0;

  async function load(q: string, v: string, archived: boolean) {
    const my = ++seq;
    loading = true;
    try {
      const r = await api.allLinks(q, v || null, archived);
      if (my === seq) all = r;
    } catch {
      // keep the previous list
    } finally {
      if (my === seq) loading = false;
    }
  }

  $effect(() => {
    void version;
    if (scope === "all") load(query, vault, includeArchived);
  });

  onMount(() => focusFirst());

  /** Focus the panel: the first row, else its first control. */
  export function focusFirst() {
    requestAnimationFrame(() => (root?.querySelector<HTMLElement>(".row .open") ?? root?.querySelector<HTMLElement>("input, .add, [role=tab]"))?.focus());
  }

  const metadataOnly = (l: NoteLink) => l.sources.every((s) => s.kind === "attached");
  /** Where "Go to source" goes: the first place in the text, else the first at all. */
  const firstSource = (l: NoteLink) => l.sources.find((s) => s.kind === "body" || s.kind === "key") ?? l.sources[0];
  const KIND: Record<LinkSource["kind"], string> = {
    body: "text",
    key: "key",
    attached: "attached",
    primary: "primary link",
    legacy: "front matter",
    ticket: "ticket note",
  };
  const where = (s: LinkSource) => `Line ${s.line} · ${s.field ? `${KIND[s.kind]} (${s.field})` : KIND[s.kind]}`;

  /** Arrow keys move between rows; Esc closes. */
  function keydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      onClose();
      return;
    }
    if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
    const target = e.target as HTMLElement;
    if (target.matches("input[type=text], input:not([type])")) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        root.querySelector<HTMLElement>(".row .open")?.focus();
      }
      return;
    }
    const rows = [...root.querySelectorAll<HTMLElement>(".row")];
    const i = rows.findIndex((r) => r.contains(target));
    if (i < 0) return;
    e.preventDefault();
    const next = rows[Math.max(0, Math.min(rows.length - 1, i + (e.key === "ArrowDown" ? 1 : -1)))];
    next.querySelector<HTMLElement>(".open")?.focus();
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<aside class="links" class:compact bind:this={root} aria-label="Links" onkeydown={keydown}>
  <div class="top">
    {#if compact}
      <span class="heading">Links</span>
    {:else}
      <div class="tabs" role="tablist" aria-label="Which links">
        <button role="tab" aria-selected={scope === "note"} class:on={scope === "note"} onclick={() => onScope("note")}>This note</button>
        <button role="tab" aria-selected={scope === "all"} class:on={scope === "all"} onclick={() => onScope("all")}>All notes</button>
      </div>
    {/if}
    <span class="grow"></span>
    <button class="icon" onclick={onClose} title="Close (Esc)" aria-label="Close the Links panel"><Icon name="x" size={15} /></button>
  </div>

  {#if scope === "note" || compact}
    {#if noteTitle !== null}
      <div class="sub">
        <span class="title" title={noteTitle}>{noteTitle}</span>
        <button class="add" onclick={onAdd} title="Add any link ({formatKey(keys.addLink)})"><Icon name="plus" size={14} />Add link</button>
      </div>
    {/if}
    <ul class="list" aria-label="This note's links">
      {#each links as l (l.identity)}
        <li class="row" class:primary={l.primary}>
          <div class="line1">
            <span class="ico"><Icon name={l.rule?.icon ?? (l.primary ? "star" : "link")} size={14} /></span>
            <span class="label" title={l.label}>{l.label}</span>
            {#if l.primary}<span class="tag">primary</span>{/if}
            {#if l.sources.length > 1}<span class="count" title="Appears {l.sources.length} times">×{l.sources.length}</span>{/if}
          </div>
          <div class="dest" title={l.url}><b>{l.domain}</b> <span class="url">{l.url}</span></div>
          <div class="acts">
            <button class="open" onclick={() => onOpenUrl(l.url)} aria-label="Open {l.label} in browser" title="Open in browser"><Icon name="external-link" size={14} />Open</button>
            <button onclick={() => onCopy(l.url)} aria-label="Copy link {l.url}" title="Copy link"><Icon name="copy" size={14} /></button>
            {#if metadataOnly(l)}
              <button onclick={() => onEdit(l)} aria-label="Edit attached link {l.label}" title="Edit attached link"><Icon name="pencil" size={14} /></button>
            {:else}
              <button onclick={() => onGoTo(firstSource(l))} aria-label="Go to source of {l.label}" title="Go to source — {where(firstSource(l))}"><Icon name="corner-down-right" size={14} /></button>
            {/if}
            <button onclick={() => onMore(l)} aria-label="More actions for {l.label}" title={l.sources.length > 1 ? "Every place it appears, and more" : "More"}><Icon name="ellipsis" size={14} /></button>
          </div>
        </li>
      {/each}
    </ul>
    {#if !links.length}
      <EmptyState icon="link" title="Links you paste into this note appear here.">
        <button class="add big" onclick={onAdd}><Icon name="plus" size={14} />Add link</button>
      </EmptyState>
    {/if}
  {:else}
    <div class="filters">
      <input type="text" placeholder="Filter links and notes" bind:value={query} aria-label="Filter links" spellcheck="false" />
      <span class="frow">
        {#if vaults.length > 1}
          <select bind:value={vault} aria-label="Vault">
            <option value="">All vaults</option>
            {#each vaults as v}<option value={v.id}>{v.name}</option>{/each}
          </select>
        {/if}
        <label><input type="checkbox" bind:checked={includeArchived} /> Include archived</label>
      </span>
    </div>
    <ul class="list" aria-label="Links in all notes" aria-busy={loading}>
      {#each all as l (l.identity)}
        <li class="row">
          <div class="line1">
            <span class="ico"><Icon name={l.rule?.icon ?? "link"} size={14} /></span>
            <span class="label" title={l.label}>{l.label}</span>
            <span class="count" title="Referenced by {l.notes.length} {l.notes.length === 1 ? 'note' : 'notes'}">{l.notes.length}</span>
          </div>
          <div class="dest" title={l.url}><b>{l.domain}</b> <span class="url">{l.url}</span></div>
          <div class="acts">
            <button class="open" onclick={() => onOpenUrl(l.url)} aria-label="Open {l.label} in browser" title="Open in browser"><Icon name="external-link" size={14} />Open</button>
            <button onclick={() => onCopy(l.url)} aria-label="Copy link {l.url}" title="Copy link"><Icon name="copy" size={14} /></button>
          </div>
          <div class="notes">
            {#each l.notes as n (n.path)}
              {@const name = n.label ? `${n.title} ${n.label}` : n.title}
              {@const inVault = vaults.length > 1 ? ` in ${vaults.find((v) => v.id === n.vault)?.name ?? n.vault}` : ""}
              <button class="note" class:archived={n.archived} onclick={() => onOpenNote(n.path)} aria-label="Open note {name}{inVault}" title="Open {name}">
                <Icon name="file-text" size={12} /><span class="nt">{name}</span>
                {#if vaults.length > 1}<VaultTag vault={n.vault} {vaults} />{/if}
              </button>
            {/each}
          </div>
        </li>
      {/each}
    </ul>
    {#if !all.length && !loading}
      <EmptyState icon="link" title={query || vault ? "No links match" : "No links in your notes yet"}>
        {#if !includeArchived}Archived notes are hidden.{/if}
      </EmptyState>
    {/if}
  {/if}
</aside>

<style>
  .links {
    width: 320px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--bg-raised);
    border-left: 1px solid var(--border);
    font-size: 13px;
  }
  .links.compact {
    width: auto;
    border-left: none;
  }
  .top {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 36px;
    padding: 0 6px 0 8px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .heading {
    font-weight: 600;
    padding-left: 4px;
  }
  .tabs {
    display: flex;
    gap: 2px;
  }
  .tabs button {
    font: inherit;
    font-size: 12.5px;
    color: var(--fg-muted);
    background: transparent;
    border: 1px solid transparent;
    border-radius: 6px;
    padding: 3px 9px;
    cursor: pointer;
  }
  .tabs button.on {
    color: var(--fg);
    background: var(--selected);
    font-weight: 600;
  }
  .grow {
    flex: 1;
  }
  button {
    font: inherit;
  }
  .icon,
  .acts button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    height: 24px;
    min-width: 26px;
    padding: 0 6px;
    color: var(--fg-muted);
    background: transparent;
    border: 1px solid transparent;
    border-radius: 5px;
    cursor: pointer;
  }
  .icon:hover,
  .acts button:hover {
    background: var(--hover);
    color: var(--fg);
  }
  button:focus-visible,
  input:focus-visible,
  select:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -1px;
  }
  .sub {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px 4px 12px;
  }
  .sub .title {
    flex: 1;
    min-width: 0;
    color: var(--fg-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .add {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 12.5px;
    color: var(--fg);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 2px 8px;
    cursor: pointer;
    white-space: nowrap;
  }
  .add:hover {
    background: var(--hover);
  }
  .add.big {
    margin-top: 6px;
  }
  .filters {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px 10px;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 60%, transparent);
  }
  .filters input[type="text"],
  .filters select {
    font: inherit;
    font-size: 12.5px;
    color: var(--fg);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 4px 8px;
  }
  .frow {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 12px;
    color: var(--fg-muted);
  }
  .frow label {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 4px;
    overflow-y: auto;
    flex: 1;
    min-height: 0;
  }
  .list:empty {
    flex: 0;
  }
  .row {
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding: 6px 8px;
    border-radius: 7px;
  }
  .row:hover,
  .row:focus-within {
    background: var(--hover);
  }
  .line1 {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .ico {
    display: flex;
    color: var(--fg-muted);
  }
  .row.primary .ico {
    color: var(--accent);
  }
  .label {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
  .tag {
    font-size: 10.5px;
    color: var(--accent);
    border: 1px solid color-mix(in srgb, var(--accent) 45%, transparent);
    border-radius: 4px;
    padding: 0 4px;
    flex-shrink: 0;
  }
  .count {
    margin-left: auto;
    font-size: 11.5px;
    color: var(--fg-muted);
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }
  .dest {
    font-size: 11.5px;
    color: var(--fg-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding-left: 20px;
  }
  .dest b {
    font-weight: 500;
    color: var(--fg);
  }
  .acts {
    display: flex;
    gap: 2px;
    padding-left: 16px;
  }
  .notes {
    display: flex;
    flex-direction: column;
    padding-left: 18px;
  }
  .note {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    color: var(--fg);
    background: transparent;
    border: none;
    border-radius: 5px;
    padding: 2px 4px;
    text-align: left;
    cursor: pointer;
    min-width: 0;
  }
  .note:hover {
    background: var(--selected);
  }
  .note.archived {
    opacity: 0.6;
  }
  .nt {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
</style>
