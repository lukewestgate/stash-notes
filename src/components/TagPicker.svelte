<script lang="ts">
  // Add and remove a note's tags (Ctrl+Shift+T). Every change is saved
  // straight away; Esc closes.
  import { onMount } from "svelte";
  import type { TagCount } from "../lib/api";
  import { normalizeTag } from "../lib/text";
  import { dim, pop } from "../lib/motion";
  import { dismissOnBackdrop, placeAt, type Anchor } from "../lib/popup";
  import Icon from "./Icon.svelte";
  import TagChip from "./TagChip.svelte";

  interface Props {
    title: string;
    /** The note's tags. */
    tags: string[];
    /** Every tag in use, most used first. */
    all: TagCount[];
    colors: Record<string, string>;
    onChange: (tags: string[]) => void;
    onClose: () => void;
    /** Open beside this (e.g. the tag chips) instead of centered. */
    anchor?: Anchor | null;
  }
  let { title, tags, all, colors, onChange, onClose, anchor = null }: Props = $props();

  // The picker owns the list while it's open; the note catches up as each change saves.
  // svelte-ignore state_referenced_locally
  let current = $state<string[]>([...tags]);
  let filter = $state("");
  let sel = $state(0);
  let input: HTMLInputElement;

  type Item = { tag: string; count: number; isNew: boolean };
  const typed = $derived(normalizeTag(filter));
  const items = $derived.by<Item[]>(() => {
    const f = filter.trim().replace(/^#/, "").toLowerCase();
    const known = new Map(all.map((t) => [t.name, t.count]));
    for (const t of current) if (!known.has(t)) known.set(t, 0);
    const list: Item[] = [...known]
      .filter(([t]) => !f || t.includes(f))
      .map(([tag, count]) => ({ tag, count, isNew: false }));
    // The note's own tags first, then by use.
    list.sort((a, b) => Number(current.includes(b.tag)) - Number(current.includes(a.tag)) || b.count - a.count);
    // Existing tags come first, so Enter picks `bug` while typing `bu`.
    if (typed && !known.has(typed)) list.push({ tag: typed, count: 0, isNew: true });
    return list.slice(0, 50);
  });

  onMount(() => input.focus());

  $effect(() => {
    void filter;
    sel = 0;
  });

  function toggle(tag: string) {
    current = current.includes(tag) ? current.filter((t) => t !== tag) : [...current, tag];
    onChange(current);
  }

  function keydown(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      sel = Math.min(sel + 1, items.length - 1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      sel = Math.max(sel - 1, 0);
    } else if (e.key === "Enter") {
      e.preventDefault();
      const it = items[sel];
      if (it) {
        toggle(it.tag);
        filter = "";
      } else if (!filter.trim()) onClose();
    } else if (e.key === "Backspace" && !filter && current.length) {
      e.preventDefault();
      toggle(current[current.length - 1]);
    } else if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      onClose();
    }
  }
</script>

<div class="popup-backdrop" class:anchored={!!anchor} role="presentation" in:dim use:dismissOnBackdrop={onClose}>
  <div class="picker popup-panel" use:placeAt={anchor} role="dialog" aria-label={title} in:pop>
    <div class="title">{title}</div>
    <div class="field">
      {#each current as t (t)}
        <span class="chosen">
          <TagChip tag={t} {colors} />
          <button class="remove" onclick={() => toggle(t)} aria-label="Remove #{t}" title="Remove"><Icon name="x" size={12} /></button>
        </span>
      {/each}
      <input
        bind:this={input}
        bind:value={filter}
        onkeydown={keydown}
        placeholder={current.length ? "Add a tag…" : "Type a tag, e.g. bug"}
        spellcheck="false"
        autocomplete="off"
      />
    </div>
    <div class="list">
      {#each items as it, i (it.tag)}
        <button class="row" class:selected={i === sel} onmousemove={() => (sel = i)} onclick={() => toggle(it.tag)}>
          <span class="check" class:on={current.includes(it.tag)}>
            {#if it.isNew}<Icon name="plus" size={14} />{:else if current.includes(it.tag)}<Icon name="check" size={14} />{/if}
          </span>
          {#if it.isNew}
            <span class="label">Create <TagChip tag={it.tag} {colors} /></span>
          {:else}
            <span class="label"><TagChip tag={it.tag} {colors} /></span>
            <span class="count">{it.count || ""}</span>
          {/if}
        </button>
      {:else}
        <div class="empty">
          {filter.trim() ? "Tags use letters, digits, - _ and / (not digits alone)" : "No tags yet — type one to create it"}
        </div>
      {/each}
    </div>
    <div class="hints"><kbd>Enter</kbd> add or remove · <kbd>Backspace</kbd> remove last · <kbd>Esc</kbd> done</div>
  </div>
</div>

<style>
  .picker {
    width: min(380px, calc(100vw - 32px));
    padding: 8px;
  }
  .title {
    font-size: 12px;
    color: var(--fg-muted);
    padding: 2px 4px 6px;
  }
  .field {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
    background: var(--bg);
    border: 1px solid var(--accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 20%, transparent);
    border-radius: 6px;
    padding: 4px 6px;
    margin-bottom: 6px;
  }
  .chosen {
    display: inline-flex;
    align-items: center;
  }
  .remove {
    display: grid;
    place-items: center;
    width: 16px;
    height: 16px;
    padding: 0;
    color: var(--fg-muted);
    background: transparent;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }
  .remove:hover {
    color: var(--fg);
    background: var(--hover);
  }
  input {
    flex: 1;
    min-width: 100px;
    font: inherit;
    color: var(--fg);
    background: transparent;
    border: none;
    outline: none;
    padding: 2px;
  }
  .list {
    max-height: 280px;
    overflow-y: auto;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    font: inherit;
    color: var(--fg);
    background: transparent;
    border: none;
    border-radius: 6px;
    padding: 4px 8px;
    cursor: pointer;
    text-align: left;
  }
  .row.selected {
    background: var(--selected);
  }
  .check {
    display: grid;
    place-items: center;
    width: 16px;
    color: var(--accent);
  }
  .label {
    flex: 1;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .count {
    font-size: 11px;
    color: var(--fg-muted);
  }
  .empty {
    font-size: 12.5px;
    color: var(--fg-muted);
    padding: 8px;
  }
  .hints {
    font-size: 11px;
    color: var(--fg-muted);
    padding: 6px 4px 0;
  }
  .hints kbd {
    font: inherit;
    font-weight: 600;
  }
</style>
