<script lang="ts">
  import type { Keys, NoteMeta } from "../lib/api";
  import { formatKey } from "../lib/keys";
  import { weekday } from "../lib/text";

  interface Props {
    meta: NoteMeta | null;
    dirty: boolean;
    canBack: boolean;
    canForward: boolean;
    hasKey: boolean;
    keys: Keys;
    onBack: () => void;
    onForward: () => void;
    onJira: () => void;
    onGerrit: () => void;
    onSwitcher: () => void;
  }
  let { meta, dirty, canBack, canForward, hasKey, keys, onBack, onForward, onJira, onGerrit, onSwitcher }: Props =
    $props();
</script>

<header class="header">
  <div class="nav">
    <button class="icon" disabled={!canBack} onclick={onBack} title="Back ({formatKey(keys.back)})" aria-label="Back"
      >‹</button
    >
    <button
      class="icon"
      disabled={!canForward}
      onclick={onForward}
      title="Forward ({formatKey(keys.forward)})"
      aria-label="Forward">›</button
    >
  </div>

  <button class="title" onclick={onSwitcher} title="Switch note ({formatKey(keys.quickSwitcher)})">
    {#if meta}
      <span class="main" class:archived={meta.archived}>{meta.title}</span>
      {#if meta.kind === "ticket" && meta.label}
        <span class="sub">{meta.label}</span>
      {:else if meta.kind === "daily" && meta.date}
        <span class="sub">{weekday(meta.date)}</span>
      {/if}
      {#if meta.archived}<span class="tag">archived</span>{/if}
    {/if}
    <span class="dot" class:visible={dirty} title="Unsaved changes"></span>
  </button>

  <div class="links">
    {#if hasKey}
      <button onclick={onJira} title="Open in Jira ({formatKey(keys.openJira)})">Jira</button>
    {/if}
    {#if meta && meta.gerrit.length > 0}
      <button onclick={onGerrit} title="Open Gerrit change ({formatKey(keys.openGerrit)})">
        Gerrit{#if meta.gerrit.length > 1}&nbsp;<span class="count">{meta.gerrit.length}</span>{/if}
      </button>
    {/if}
  </div>
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
  .nav {
    display: flex;
    gap: 2px;
  }
  button {
    font: inherit;
    color: var(--fg-muted);
    background: transparent;
    border: 1px solid transparent;
    border-radius: 5px;
    padding: 2px 8px;
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
  .icon {
    font-size: 18px;
    line-height: 1;
    padding: 2px 7px 4px;
  }
  .title {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: baseline;
    gap: 10px;
    text-align: left;
    color: var(--fg);
    overflow: hidden;
    white-space: nowrap;
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
  .links {
    display: flex;
    gap: 4px;
  }
  .links button {
    border-color: var(--border);
    color: var(--fg);
  }
  .count {
    color: var(--fg-muted);
  }
</style>
