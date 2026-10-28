<script lang="ts">
  // Where you are (back, forward and a breadcrumb) and what you can do with
  // the window's note (pin it, or the note menu). A note's status, tags,
  // ticket and links live in its properties row, not here.
  import type { Keys, NoteMeta, VaultDef } from "../lib/api";
  import Icon from "./Icon.svelte";
  import { formatKey } from "../lib/keys";
  import { weekday } from "../lib/text";
  import { noteName, showsWeekday } from "../lib/daily.svelte";
  import { anchorOf, type Anchor } from "../lib/popup";

  interface Props {
    meta: NoteMeta | null;
    dirty: boolean;
    canBack: boolean;
    canForward: boolean;
    keys: Keys;
    vaults: VaultDef[];
    onBack: () => void;
    onForward: () => void;
    /** The current note is open in a pinned window. */
    pinned: boolean;
    onPin: () => void;
    /** Open the note menu beside the … button. */
    onActions: (anchor: Anchor) => void;
  }
  let { meta, dirty, canBack, canForward, keys, vaults, onBack, onForward, pinned, onPin, onActions }: Props = $props();

  const pinKeys = $derived(keys.pinNote.split(/\s+/).map(formatKey).join(" then "));
  const KIND = { daily: "Daily", ticket: "Tickets", note: "Notes" } as const;
  const crumbs = $derived.by(() => {
    if (!meta) return [];
    const out: string[] = [];
    if (vaults.length > 1) out.push(vaults.find((v) => v.id === meta.vault)?.name ?? meta.vault);
    if (meta.archived) out.push("Archive");
    out.push(KIND[meta.kind]);
    return out;
  });
</script>

<header class="header">
  <div class="nav">
    <button disabled={!canBack} onclick={onBack} title="Back ({formatKey(keys.back)})" aria-label="Back"
      ><Icon name="chevron-left" size={16} /></button
    >
    <button
      disabled={!canForward}
      onclick={onForward}
      title="Forward ({formatKey(keys.forward)})"
      aria-label="Forward"><Icon name="chevron-right" size={16} /></button
    >
  </div>

  <nav class="title" aria-label="Breadcrumb">
    {#each crumbs as c}<span class="crumb">{c}</span><span class="slash" aria-hidden="true">/</span>{/each}
    {#if meta}
      <span class="main" class:archived={meta.archived} aria-current="page">{noteName(meta)}</span>
      {#if meta.kind === "ticket" && meta.label}
        <span class="sub">{meta.label}</span>
      {:else if meta.kind === "daily" && meta.date && !showsWeekday()}
        <span class="sub">{weekday(meta.date)}</span>
      {/if}
      {#if meta.archived}<span class="tag">archived</span>{/if}
    {/if}
    <span class="dot" class:visible={dirty} title="Unsaved changes"></span>
  </nav>

  {#if meta}
    <div class="actions">
      <button
        class:on={pinned}
        onclick={onPin}
        title="{pinned ? 'Unpin' : 'Pin beside your work'} ({pinKeys})"
        aria-label={pinned ? "Unpin" : "Pin"}
        aria-pressed={pinned}
      >
        <Icon name={pinned ? "pin-off" : "pin"} size={15} />
      </button>
      <button onclick={(e) => onActions(anchorOf(e.currentTarget))} title="Note menu" aria-label="Note menu" aria-haspopup="menu">
        <Icon name="ellipsis" size={15} />
      </button>
    </div>
  {/if}
</header>

<style>
  .header {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    padding: 0 8px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-raised);
    user-select: none;
    flex-shrink: 0;
  }
  .nav,
  .actions {
    display: flex;
    gap: 2px;
  }
  button {
    display: grid;
    place-items: center;
    width: 28px;
    height: 26px;
    padding: 0;
    font: inherit;
    color: var(--fg-muted);
    background: transparent;
    border: 1px solid transparent;
    border-radius: 5px;
    cursor: pointer;
  }
  button:hover:not(:disabled) {
    background: var(--hover);
    color: var(--fg);
  }
  button:disabled {
    opacity: 0.35;
    cursor: default;
  }
  button.on {
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 14%, transparent);
  }
  .title {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: baseline;
    gap: 7px;
    padding: 0 4px;
    color: var(--fg);
    overflow: hidden;
    white-space: nowrap;
  }
  .crumb,
  .slash {
    color: var(--fg-muted);
    font-size: 12.5px;
  }
  .slash {
    opacity: 0.6;
  }
  .main {
    font-weight: 600;
  }
  .main.archived {
    opacity: 0.6;
  }
  .sub {
    color: var(--fg-muted);
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tag {
    font-size: 11px;
    color: var(--fg-muted);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0 5px;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent);
    align-self: center;
    flex-shrink: 0;
    visibility: hidden;
  }
  .dot.visible {
    visibility: visible;
  }
</style>
