<script lang="ts">
  import type { Keys, SearchHit, StatusDef } from "../lib/api";
  import { formatKey } from "../lib/keys";
  import { relTime } from "../lib/text";
  import StatusIcon from "./StatusIcon.svelte";

  interface Props {
    active: SearchHit[];
    more: number;
    recent: SearchHit[];
    current: string | null;
    /** Which full-window view is showing, to highlight its row. */
    screen: string;
    expanded: boolean;
    width: number;
    /** Narrow window: the expanded panel floats over the content. */
    overlay: boolean;
    statuses: StatusDef[];
    keys: Keys;
    /** Optional extra row in the footer (e.g. an update notice). */
    onToggle: () => void;
    onResize: (width: number) => void;
    onToday: () => void;
    onNotes: (query?: string) => void;
    onSettings: () => void;
    onOpen: (path: string) => void;
    onStatus: (hit: SearchHit) => void;
    onLeave: () => void;
  }
  let {
    active,
    more,
    recent,
    current,
    screen,
    expanded,
    width,
    overlay,
    statuses,
    keys,
    onToggle,
    onResize,
    onToday,
    onNotes,
    onSettings,
    onOpen,
    onStatus,
    onLeave,
  }: Props = $props();

  type Item =
    | { kind: "today" }
    | { kind: "notes" }
    | { kind: "more" }
    | { kind: "settings" }
    | { kind: "note"; hit: SearchHit };

  const items = $derived<Item[]>([
    { kind: "today" },
    { kind: "notes" },
    ...active.map((hit) => ({ kind: "note" as const, hit })),
    ...(more > 0 ? [{ kind: "more" as const }] : []),
    ...(expanded ? recent.map((hit) => ({ kind: "note" as const, hit })) : []),
    { kind: "settings" },
  ]);

  let focused = $state(-1);
  let root: HTMLElement;

  /** Keyboard focus into the sidebar (Ctrl+0). */
  export function focus() {
    const i = items.findIndex((it) => it.kind === "note" && it.hit.path === current);
    focused = i >= 0 ? i : 0;
    root.querySelector<HTMLElement>(`[data-i="${focused}"]`)?.focus();
  }

  export function focusedHit(): SearchHit | null {
    const it = items[focused];
    return it?.kind === "note" ? it.hit : null;
  }

  function run(it: Item) {
    switch (it.kind) {
      case "today":
        return onToday();
      case "notes":
        return onNotes();
      case "more":
        return onNotes("@active");
      case "settings":
        return onSettings();
      case "note":
        return onOpen(it.hit.path);
    }
  }

  function keydown(e: KeyboardEvent) {
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      focused = Math.max(0, Math.min(items.length - 1, focused + (e.key === "ArrowDown" ? 1 : -1)));
      root.querySelector<HTMLElement>(`[data-i="${focused}"]`)?.focus();
    } else if (e.key === "Enter" && items[focused]) {
      e.preventDefault();
      run(items[focused]);
    } else if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      focused = -1;
      onLeave();
    }
  }

  function startDrag(e: PointerEvent) {
    e.preventDefault();
    const startX = e.clientX;
    const startW = width;
    const move = (ev: PointerEvent) => onResize(Math.max(180, Math.min(360, startW + ev.clientX - startX)));
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  }

  const name = (h: SearchHit) => (h.kind === "ticket" && h.key ? h.key : h.title);
  const tip = (h: SearchHit) =>
    `${h.kind === "ticket" && h.label ? `${h.key} ${h.label}` : h.title}${h.status ? ` — ${h.status}` : ""}\nEdited ${relTime(h.mtime)} ago`;
  const indexOf = (it: Item) => items.indexOf(it);
</script>

{#snippet icon(kind: string)}
  <svg viewBox="0 0 20 20" width="18" height="18" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
    {#if kind === "menu"}
      <path d="M3.5 5.5h13M3.5 10h13M3.5 14.5h13" />
    {:else if kind === "today"}
      <rect x="3" y="4.5" width="14" height="12" rx="2" /><path d="M3 8.5h14M7 3v3M13 3v3" />
    {:else if kind === "notes"}
      <path d="M7 5.5h9.5M7 10h9.5M7 14.5h9.5" /><circle cx="3.8" cy="5.5" r=".6" /><circle cx="3.8" cy="10" r=".6" /><circle cx="3.8" cy="14.5" r=".6" />
    {:else if kind === "settings"}
      <circle cx="10" cy="10" r="2.6" /><path d="M10 2.5v2M10 15.5v2M2.5 10h2M15.5 10h2M4.7 4.7l1.4 1.4M13.9 13.9l1.4 1.4M4.7 15.3l1.4-1.4M13.9 6.1l1.4-1.4" />
    {/if}
  </svg>
{/snippet}

{#snippet row(it: Item, label: string, ico: string, hint: string, isCurrent: boolean)}
  {@const i = indexOf(it)}
  <button
    class="row nav"
    class:current={isCurrent}
    data-i={i}
    tabindex={focused === i ? 0 : -1}
    title={expanded ? `${label} (${hint})` : `${label} (${hint})`}
    onclick={() => run(it)}
    onfocus={() => (focused = i)}
  >
    <span class="ico">{@render icon(ico)}</span>
    {#if expanded}<span class="label">{label}</span><span class="hint">{hint}</span>{/if}
  </button>
{/snippet}

{#snippet noteRow(it: Item & { kind: "note" })}
  {@const i = indexOf(it)}
  <button
    class="row note"
    class:current={it.hit.path === current && screen === "editor"}
    data-i={i}
    tabindex={focused === i ? 0 : -1}
    title={tip(it.hit)}
    onclick={() => run(it)}
    onfocus={() => (focused = i)}
    oncontextmenu={(e) => {
      e.preventDefault();
      onStatus(it.hit);
    }}
  >
    <span class="ico sym">
      {#if it.hit.status}<StatusIcon status={it.hit.status} {statuses} />{:else}<span class="dot">·</span>{/if}
    </span>
    {#if expanded}
      <span class="label">
        <span class="name">{name(it.hit)}</span>
        {#if it.hit.kind === "ticket" && it.hit.label}<span class="sub">{it.hit.label}</span>{/if}
      </span>
    {/if}
  </button>
{/snippet}

<!-- Arrow keys move between the rows (buttons) inside. -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<nav
  class="sidebar"
  class:expanded
  class:overlay={overlay && expanded}
  style={expanded ? `width: ${width}px` : ""}
  bind:this={root}
  onkeydown={keydown}
  aria-label="Sidebar"
>
  <div class="top">
    <button class="row nav menu" onclick={onToggle} title="{expanded ? 'Collapse' : 'Expand'} sidebar ({formatKey(keys.toggleSidebar)})" tabindex="-1">
      <span class="ico">{@render icon("menu")}</span>
      {#if expanded}<span class="label brand">Stash Notes</span>{/if}
    </button>
    {@render row(items[0], "Today", "today", formatKey(keys.dailyNote), false)}
    {@render row(items[1], "Notes", "notes", formatKey(keys.notesView), screen === "notes")}
  </div>

  <div class="groups">
    {#if active.length}
      {#if expanded}<div class="group">Active</div>{:else}<div class="sep"></div>{/if}
      {#each items.filter((it) => it.kind === "note" && active.includes(it.hit)) as it (it.kind === "note" ? it.hit.path : "")}
        {@render noteRow(it as Item & { kind: "note" })}
      {/each}
      {#if more > 0}
        {@const it = items.find((x) => x.kind === "more")!}
        <button class="row more" data-i={indexOf(it)} tabindex={focused === indexOf(it) ? 0 : -1} onclick={() => run(it)} title="Show all active notes" onfocus={() => (focused = indexOf(it))}>
          <span class="ico">+{more}</span>
          {#if expanded}<span class="label">more</span>{/if}
        </button>
      {/if}
    {/if}
    {#if expanded && recent.length}
      <div class="group">Recent</div>
      {#each items.filter((it) => it.kind === "note" && recent.includes(it.hit)) as it (it.kind === "note" ? it.hit.path : "")}
        {@render noteRow(it as Item & { kind: "note" })}
      {/each}
    {/if}
  </div>

  <div class="bottom">
    {@render row(items[items.length - 1], "Settings", "settings", formatKey(keys.openSettings), screen === "settings")}
  </div>

  {#if expanded}
    <div class="grip" role="separator" aria-orientation="vertical" onpointerdown={startDrag}></div>
  {/if}
</nav>

<style>
  .sidebar {
    position: relative;
    width: 44px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg-raised);
    border-right: 1px solid var(--border);
    user-select: none;
    overflow: hidden;
    z-index: 15;
  }
  .sidebar.overlay {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    box-shadow: var(--shadow);
  }
  .top,
  .bottom {
    display: flex;
    flex-direction: column;
    padding: 4px;
    gap: 1px;
  }
  .bottom {
    border-top: 1px solid var(--border);
  }
  .groups {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 0 4px;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .group {
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--fg-muted);
    padding: 12px 8px 4px;
  }
  .sep {
    height: 1px;
    background: var(--border);
    margin: 6px 6px;
  }
  .row {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 28px;
    width: 100%;
    padding: 0 8px 0 7px;
    font: inherit;
    font-size: 13px;
    color: var(--fg);
    background: transparent;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    text-align: left;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .row:hover {
    background: var(--hover);
  }
  .row:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }
  .row.current {
    background: var(--selected);
  }
  .row.current::before {
    content: "";
    position: absolute;
    left: 0;
    top: 6px;
    bottom: 6px;
    width: 2px;
    border-radius: 2px;
    background: var(--accent);
  }
  .menu {
    color: var(--fg-muted);
  }
  .ico {
    width: 18px;
    flex-shrink: 0;
    display: flex;
    justify-content: center;
    color: var(--fg-muted);
  }
  .row.nav .ico,
  .row.current .ico {
    color: var(--fg);
  }
  .more .ico {
    font-size: 11px;
    color: var(--fg-muted);
  }
  .dot {
    color: var(--fg-muted);
  }
  .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .brand {
    font-weight: 600;
    color: var(--fg);
  }
  .hint {
    font-size: 11px;
    color: var(--fg-muted);
    opacity: 0.7;
  }
  .name {
    font-weight: 600;
  }
  .sub {
    color: var(--fg-muted);
    margin-left: 6px;
  }
  .grip {
    position: absolute;
    top: 0;
    right: -3px;
    width: 6px;
    height: 100%;
    cursor: col-resize;
  }
  .grip:hover {
    background: color-mix(in srgb, var(--accent) 30%, transparent);
  }
</style>
