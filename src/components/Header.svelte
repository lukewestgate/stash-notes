<script lang="ts">
  import type { Keys, LinkType, NoteMeta, StatusDef, TicketSystem } from "../lib/api";
  import Icon from "./Icon.svelte";
  import StatusIcon from "./StatusIcon.svelte";
  import { formatKey } from "../lib/keys";
  import { weekday } from "../lib/text";

  interface Props {
    meta: NoteMeta | null;
    dirty: boolean;
    canBack: boolean;
    canForward: boolean;
    /** The ticket system of the note's ticket, if it has one. */
    ticket: TicketSystem | null;
    /** The link types of the note's vault. */
    linkTypes: LinkType[];
    keys: Keys;
    statuses: StatusDef[];
    onStatus: () => void;
    onTags: () => void;
    onBack: () => void;
    onForward: () => void;
    onTicket: () => void;
    onLink: () => void;
    onAddLink: () => void;
    onSwitcher: () => void;
    /** The current note is open in a pinned window. */
    pinned: boolean;
    onPin: () => void;
    onArchive: () => void;
    onImage: () => void;
    onActions: () => void;
  }
  let {
    meta,
    dirty,
    canBack,
    canForward,
    ticket,
    linkTypes,
    keys,
    statuses,
    onStatus,
    onTags,
    onBack,
    onForward,
    onTicket,
    onLink,
    onAddLink,
    onSwitcher,
    pinned,
    onPin,
    onArchive,
    onImage,
    onActions,
  }: Props = $props();

  const pinKeys = $derived(keys.pinNote.split(/\s+/).map(formatKey).join(" then "));
  /** The vault's link types this note has entries for, with how many. */
  const used = $derived(meta ? linkTypes.map((t) => ({ type: t, n: meta.fields[t.field]?.length ?? 0 })).filter((u) => u.n > 0) : []);
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
    {#if ticket}
      <button class="action" onclick={onTicket} title="Open in {ticket.name} ({formatKey(keys.openTicket)})">
        <Icon name={ticket.icon} size={15} /><span>{ticket.name}</span>
      </button>
    {/if}
    {#each used as u (u.type.field)}
      <button class="action" onclick={onLink} title="Open {u.type.name} link ({formatKey(keys.openLink)})">
        <Icon name={u.type.icon} size={15} /><span>{u.type.name}</span>{#if u.n > 1}<span class="count">{u.n}</span>{/if}
      </button>
    {/each}
    {#if meta && meta.kind !== "daily" && linkTypes.length}
      <button
        class="action icon"
        onclick={onAddLink}
        title="Add a link: {linkTypes.map((t) => t.name).join(', ')} ({formatKey(keys.addLink)})"
        aria-label="Add a link"
      >
        <Icon name="link" size={15} />
      </button>
    {/if}
    {#if meta}
      <button class="action icon" onclick={onImage} title="Insert image (or paste / drop an image)" aria-label="Insert image">
        <Icon name="image-plus" size={15} />
      </button>
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
      <button class="action icon" onclick={onActions} title="Rename, move or delete this note" aria-label="Note actions"><Icon name="ellipsis" size={15} /></button>
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
