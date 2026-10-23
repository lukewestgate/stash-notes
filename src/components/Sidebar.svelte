<script lang="ts">
  import type { Keys, NoteMeta, SearchHit, StatusDef, VaultDef } from "../lib/api";
  import { formatKey } from "../lib/keys";
  import { isoDate, relTime } from "../lib/text";
  import Icon from "./Icon.svelte";
  import StatusIcon from "./StatusIcon.svelte";

  interface Props {
    vaults: VaultDef[];
    currentVault: string;
    /** Open the vault switcher. */
    onVault: () => void;
    active: SearchHit[];
    more: number;
    recent: SearchHit[];
    current: string | null;
    /** The note open in the editor, when the editor is showing. */
    open: NoteMeta | null;
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
    /** The Links panel shows every note's links. */
    linksOn: boolean;
    onLinks: () => void;
    onSettings: () => void;
    onOpen: (path: string) => void;
    onActions: (hit: SearchHit) => void;
    onCurrentActions: () => void;
    onLeave: () => void;
  }
  let {
    vaults,
    currentVault,
    onVault,
    active,
    more,
    recent,
    current,
    open,
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
    linksOn,
    onLinks,
    onSettings,
    onOpen,
    onActions,
    onCurrentActions,
    onLeave,
  }: Props = $props();

  type Item =
    | { kind: "today" }
    | { kind: "notes" }
    | { kind: "links" }
    | { kind: "more" }
    | { kind: "settings" }
    | { kind: "note"; hit: SearchHit };

  const isToday = $derived(open?.kind === "daily" && open.date === isoDate());
  /** The open note, when no other row stands for it: shown under "Open" so it's clear where you are. */
  const openHit = $derived.by<SearchHit | null>(() => {
    if (!open || isToday) return null;
    const listed = active.some((h) => h.path === open.path) || (expanded && recent.some((h) => h.path === open.path));
    if (listed) return null;
    const { path, vault, kind, title, label, key, status, tags, archived } = open;
    return { path, vault, kind, title, label, key, status, tags, archived, mtime: 0, snippet: null, score: 0 };
  });

  const items = $derived<Item[]>([
    { kind: "today" },
    { kind: "notes" },
    { kind: "links" },
    ...(openHit ? [{ kind: "note" as const, hit: openHit }] : []),
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
      case "links":
        return onLinks();
      case "more":
        return onNotes("@active");
      case "settings":
        return onSettings();
      case "note":
        return onOpen(it.hit.path);
    }
  }

  function keydown(e: KeyboardEvent) {
    if ((e.target as HTMLElement).closest(".file-actions")) return;
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
    `${h.kind === "ticket" && h.label ? `${h.key} ${h.label}` : h.title}${h.status ? ` — ${h.status}` : ""}` +
    `${h.tags.length ? `\n${h.tags.map((t) => `#${t}`).join(" ")}` : ""}\nEdited ${relTime(h.mtime)} ago`;
  const indexOf = (it: Item) => items.indexOf(it);
</script>

{#snippet icon(kind: string)}
  <Icon name={{ today: "calendar", notes: "list", links: "link" }[kind] ?? kind} size={18} />
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
    oncontextmenu={(e) => {
      if (it.kind === "today" && isToday) { e.preventDefault(); onCurrentActions(); }
    }}
  >
    <span class="ico">{@render icon(ico)}</span>
    {#if expanded}<span class="label">{label}</span><span class="hint">{hint}</span>{/if}
  </button>
{/snippet}

{#snippet noteRow(it: Item & { kind: "note" })}
  {@const i = indexOf(it)}
  <div class="note-row">
  <button
    class="row note"
    class:current={it.hit.path === current && screen === "editor"}
    data-i={i}
    tabindex={focused === i ? 0 : -1}
    title={it.hit.mtime ? tip(it.hit) : name(it.hit)}
    onclick={() => run(it)}
    onfocus={() => (focused = i)}
    oncontextmenu={(e) => {
      e.preventDefault();
      onActions(it.hit);
    }}
  >
    <span class="ico sym">
      {#if it.hit.status}<StatusIcon status={it.hit.status} {statuses} />{:else}<span class="dot"><Icon name="file-text" size={14} /></span>{/if}
    </span>
    {#if expanded}
      <span class="label">
        <span class="name">{name(it.hit)}</span>
        {#if it.hit.kind === "ticket" && it.hit.label}<span class="sub">{it.hit.label}</span>{/if}
      </span>
    {/if}
  </button>
  {#if expanded}
    <button class="file-actions" aria-label="Actions for {it.hit.key ?? it.hit.title}" title="Note actions" onclick={() => onActions(it.hit)}><Icon name="ellipsis" size={15} /></button>
  {/if}
  </div>
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
    {#if vaults.length > 1}
      {@const v = vaults.find((x) => x.id === currentVault) ?? vaults[0]}
      <button
        class="row nav vault"
        onclick={onVault}
        title="Vault: {v.name} — switch ({formatKey(keys.switchVault)})"
        tabindex="-1"
        style:--vc="var(--c-{v.color})"
      >
        <span class="ico"><span class="badge">{v.name.slice(0, 1).toUpperCase()}</span></span>
        {#if expanded}
          <span class="label vname">{v.name}</span>
          <span class="chev"><Icon name="chevron-down" size={14} /></span>
        {/if}
      </button>
    {/if}
    {@render row(items[0], "Today", "today", formatKey(keys.dailyNote), isToday)}
    {@render row(items[1], "Notes", "notes", formatKey(keys.notesView), screen === "notes")}
    {@render row(items[2], "Links", "links", "all notes", linksOn)}
  </div>

  <div class="groups">
    {#if openHit}
      {#if expanded}<div class="group">Open</div>{:else}<div class="sep"></div>{/if}
      {@render noteRow(items[3] as Item & { kind: "note" })}
    {/if}
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
  .note-row { position: relative; display: flex; flex-shrink: 0; }
  .expanded .note-row .row { padding-right: 30px; }
  .file-actions { position: absolute; right: 2px; top: 2px; display: grid; place-items: center; width: 25px; height: 24px; padding: 0; border: none; border-radius: 5px; background: transparent; color: var(--fg-muted); cursor: pointer; }
  .file-actions:hover { background: var(--hover); color: var(--fg); }
  .file-actions:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
  .sidebar {
    position: relative;
    width: 44px;
    transition: width var(--motion) ease;
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
  .row {
    transition: background-color var(--motion-fast) ease;
  }
  .row:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }
  .row.current {
    background: color-mix(in srgb, var(--accent) 14%, transparent);
  }
  .row.current .label,
  .row.nav.current .ico {
    color: var(--accent);
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
  .badge {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border-radius: 5px;
    font-size: 11px;
    font-weight: 700;
    color: #fff;
    background: var(--vc);
  }
  .vault {
    background: color-mix(in srgb, var(--vc) 10%, transparent);
  }
  .vault:hover {
    background: color-mix(in srgb, var(--vc) 18%, transparent);
  }
  .vname {
    font-weight: 600;
  }
  .chev {
    display: flex;
    color: var(--fg-muted);
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
