<script lang="ts">
  import type { Keys, SearchHit } from "../lib/api";
  import { formatKey } from "../lib/keys";

  interface Props {
    items: SearchHit[];
    expanded: boolean;
    keys: Keys;
    onOpen: (path: string) => void;
    onToggle: () => void;
  }
  let { items, expanded, keys, onOpen, onToggle }: Props = $props();

  let sel = $state(0);
  let list = $state<HTMLDivElement>();

  const name = (h: SearchHit) => (h.kind === "daily" ? `${h.title} (daily)` : h.label ? `${h.title} ${h.label}` : h.title);

  $effect(() => {
    if (expanded && list) {
      sel = 0;
      list.focus();
    }
  });

  $effect(() => {
    list?.querySelector(`[data-row="${sel}"]`)?.scrollIntoView({ block: "nearest" });
  });

  function keydown(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      sel = Math.min(sel + 1, items.length - 1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      sel = Math.max(sel - 1, 0);
    } else if (e.key === "Enter" && items[sel]) {
      e.preventDefault();
      onOpen(items[sel].path);
    }
  }
</script>

<footer class="backlinks" class:expanded>
  {#if expanded}
    <div class="list" bind:this={list} tabindex="-1" role="listbox" onkeydown={keydown}>
      {#each items as h, i (h.path)}
        <button
          class="item"
          class:selected={i === sel}
          class:archived={h.archived}
          data-row={i}
          role="option"
          aria-selected={i === sel}
          onclick={() => onOpen(h.path)}
        >
          <span class="name">{name(h)}</span>
          {#if h.snippet}<span class="snippet">{h.snippet}</span>{/if}
        </button>
      {:else}
        <div class="none">Nothing links to or mentions this note yet.</div>
      {/each}
    </div>
  {/if}
  <button class="strip" onclick={onToggle} title="Backlinks ({formatKey(keys.backlinks)})">
    {#if items.length}
      <span class="lead">Mentioned in:</span>
      {#each items.slice(0, 8) as h, i (h.path)}
        {#if i > 0}<span class="sep">·</span>{/if}
        <span class="ref">{name(h)}</span>
      {/each}
      {#if items.length > 8}<span class="sep">+{items.length - 8} more</span>{/if}
    {:else}
      <span class="lead">No backlinks</span>
    {/if}
  </button>
</footer>

<style>
  .backlinks {
    border-top: 1px solid var(--border);
    background: var(--bg-raised);
    flex-shrink: 0;
    font-size: 12.5px;
  }
  .strip {
    display: flex;
    align-items: baseline;
    gap: 6px;
    width: 100%;
    height: 26px;
    padding: 0 12px;
    font: inherit;
    color: var(--fg-muted);
    background: transparent;
    border: none;
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    cursor: pointer;
    line-height: 26px;
  }
  .ref {
    color: var(--fg);
  }
  .list {
    max-height: 32vh;
    overflow-y: auto;
    padding: 4px;
    outline: none;
    border-bottom: 1px solid var(--border);
  }
  .item {
    display: flex;
    flex-direction: column;
    width: 100%;
    font: inherit;
    text-align: left;
    color: var(--fg);
    background: transparent;
    border: none;
    border-radius: 5px;
    padding: 4px 8px;
    cursor: pointer;
  }
  .item.selected {
    background: var(--selected);
  }
  .item.archived {
    opacity: 0.55;
  }
  .name {
    font-weight: 600;
  }
  .snippet {
    color: var(--fg-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .none {
    padding: 6px 8px;
    color: var(--fg-muted);
  }
</style>
