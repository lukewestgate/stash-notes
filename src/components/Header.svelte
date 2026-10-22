<script lang="ts">
  import type { Keys, NoteMeta, StatusDef } from "../lib/api";
  import Icon from "./Icon.svelte";
  import StatusIcon from "./StatusIcon.svelte";
  import { formatKey } from "../lib/keys";
  import { weekday } from "../lib/text";

  interface Props {
    meta: NoteMeta | null;
    dirty: boolean;
    canBack: boolean;
    canForward: boolean;
    hasKey: boolean;
    keys: Keys;
    statuses: StatusDef[];
    onStatus: () => void;
    onTags: () => void;
    onBack: () => void;
    onForward: () => void;
    onJira: () => void;
    onGerrit: () => void;
    onAddGerrit: () => void;
    onSwitcher: () => void;
    /** The current note is open in a pinned window. */
    pinned: boolean;
    onPin: () => void;
    onArchive: () => void;
  }
  let {
    meta,
    dirty,
    canBack,
    canForward,
    hasKey,
    keys,
    statuses,
    onStatus,
    onTags,
    onBack,
    onForward,
    onJira,
    onGerrit,
    onAddGerrit,
    onSwitcher,
    pinned,
    onPin,
    onArchive,
  }: Props = $props();

  const pinKeys = $derived(keys.pinNote.split(/\s+/).map(formatKey).join(" then "));
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

  <div class="actions">
    {#if meta && meta.kind !== "daily"}
      <button class="chip" onclick={onStatus} title="Set status ({formatKey(keys.setStatus)})">
        {#if meta.status}<StatusIcon status={meta.status} {statuses} label />{:else}<span class="nostatus"
            ><Icon name="circle-dashed" size={14} />status</span
          >{/if}
      </button>
    {/if}
    {#if meta && meta.kind !== "daily"}
      <button
        class="action"
        class:icon={!meta.tags.length}
        onclick={onTags}
        title="Tags ({formatKey(keys.setTags)})"
        aria-label="Tags"
      >
        <Icon name={meta.tags.length > 1 ? "tags" : "tag"} size={15} />{#if meta.tags.length}<span class="count">{meta.tags.length}</span>{/if}
      </button>
    {/if}
    {#if hasKey}
      <button class="action" onclick={onJira} title="Open in Jira ({formatKey(keys.openJira)})">
        <Icon name="ticket" size={15} /><span>Jira</span>
      </button>
    {/if}
    {#if meta && meta.gerrit.length > 0}
      <button class="action" onclick={onGerrit} title="Open Gerrit change ({formatKey(keys.openGerrit)})">
        <Icon name="git-pull-request" size={15} /><span>Gerrit</span>{#if meta.gerrit.length > 1}<span class="count"
            >{meta.gerrit.length}</span
          >{/if}
      </button>
    {:else if meta && meta.kind !== "daily"}
      <button class="action icon" onclick={onAddGerrit} title="Add a Gerrit change ({formatKey(keys.addGerrit)})" aria-label="Add Gerrit change">
        <Icon name="git-pull-request" size={15} />
      </button>
    {/if}
    {#if meta}
      <span class="divider"></span>
      <button
        class="action icon"
        class:on={pinned}
        onclick={onPin}
        title="{pinned ? 'Unpin' : 'Pin beside your work'} ({pinKeys})"
        aria-label={pinned ? "Unpin" : "Pin"}
        aria-pressed={pinned}
      >
        <Icon name={pinned ? "pin-off" : "pin"} size={15} />
      </button>
      <button
        class="action icon"
        onclick={onArchive}
        title="{meta.archived ? 'Unarchive' : 'Archive'} ({formatKey(keys.archive)})"
        aria-label={meta.archived ? "Unarchive" : "Archive"}
      >
        <Icon name="archive" size={15} />
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
  .nav button {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    padding: 0;
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
  .actions {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .action {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 26px;
    padding: 0 8px;
  }
  .action.icon {
    width: 28px;
    padding: 0;
    justify-content: center;
  }
  .action.on {
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 14%, transparent);
  }
  .chip {
    display: inline-flex;
    align-items: center;
    height: 26px;
    margin-right: 2px;
  }
  .divider {
    width: 1px;
    height: 16px;
    margin: 0 4px;
    background: var(--border);
  }
  .nostatus {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    color: var(--fg-muted);
  }
  .count {
    color: var(--fg-muted);
    font-variant-numeric: tabular-nums;
  }
</style>
