<script lang="ts" module>
  /** One row of a menu. `items` makes it open a submenu instead of running. */
  export interface MenuItem {
    label: string;
    /** An icon name (lib/icons.ts), or a status to show its icon. */
    icon?: string;
    status?: string;
    /** A vault's initial on its color (theme color name). */
    badge?: { text: string; color: string };
    /** Right-aligned: a shortcut, or the current value of a submenu. */
    hint?: string;
    title?: string;
    danger?: boolean;
    disabled?: boolean;
    /** Set on a choice in a list (a status): shows a check when true. */
    checked?: boolean;
    items?: MenuEntry[];
    /** A submenu whose first nine choices can be picked with 1–9. */
    numbered?: boolean;
    run?: () => void;
  }
  /** `"-"` is a separator. */
  export type MenuEntry = MenuItem | "-";
</script>

<script lang="ts">
  // A right-click style menu: opens at its anchor with nothing dimmed, one
  // level of submenus, and the keys you'd expect (arrows, Home/End, Enter,
  // → and ← for submenus, Esc).
  import { onMount, tick } from "svelte";
  import type { StatusDef } from "../lib/api";
  import { pop } from "../lib/motion";
  import { anchorOf, dismissOnBackdrop, placeAt, type Anchor } from "../lib/popup";
  import Icon from "./Icon.svelte";
  import StatusIcon from "./StatusIcon.svelte";

  interface Props {
    label: string;
    items: MenuEntry[];
    anchor: Anchor;
    statuses?: StatusDef[];
    onClose: () => void;
  }
  let { label, items, anchor, statuses = [], onClose }: Props = $props();

  let main: HTMLDivElement;
  let subEl = $state<HTMLDivElement>();
  /** Index in `items` of the item whose submenu is open. */
  let openSub = $state<number | null>(null);
  let subAnchor = $state<Anchor | null>(null);
  const sub = $derived(openSub === null ? null : (items[openSub] as MenuItem));

  const buttons = (panel: HTMLElement | undefined) => [...(panel?.querySelectorAll<HTMLButtonElement>(":scope > button:not([aria-disabled='true'])") ?? [])];

  onMount(() => buttons(main)[0]?.focus());

  async function openSubmenu(i: number, el: HTMLElement, focus: boolean) {
    openSub = i;
    subAnchor = anchorOf(el, "right");
    if (!focus) return;
    await tick();
    const list = buttons(subEl);
    (list.find((b) => b.getAttribute("aria-checked") === "true") ?? list[0])?.focus();
  }

  function closeSubmenu(focusParent: boolean) {
    const i = openSub;
    openSub = null;
    if (focusParent && i !== null) main.querySelector<HTMLElement>(`[data-i="${i}"]`)?.focus();
  }

  function run(it: MenuItem) {
    if (it.disabled) return;
    // Close first: running may open another popup in this one's place.
    onClose();
    it.run?.();
  }

  function activate(i: number, el: HTMLElement, viaKey: boolean) {
    const it = items[i] as MenuItem;
    if (it.disabled) return;
    if (it.items) openSubmenu(i, el, viaKey);
    else run(it);
  }

  function hover(i: number, el: HTMLElement) {
    const it = items[i] as MenuItem;
    if (it.items && !it.disabled) {
      if (openSub !== i) openSubmenu(i, el, false);
    } else openSub = null;
  }

  function keydown(e: KeyboardEvent) {
    const inSub = !!subEl?.contains(document.activeElement);
    const panel = inSub ? subEl : main;
    const list = buttons(panel);
    const at = list.indexOf(document.activeElement as HTMLButtonElement);
    const focusAt = (n: number) => list[(n + list.length) % list.length]?.focus();
    const handled = () => {
      e.preventDefault();
      e.stopPropagation();
    };
    if (e.key === "ArrowDown") (handled(), focusAt(at + 1));
    else if (e.key === "ArrowUp") (handled(), focusAt(at < 0 ? -1 : at - 1));
    else if (e.key === "Home") (handled(), focusAt(0));
    else if (e.key === "End") (handled(), focusAt(-1));
    else if (e.key === "Escape" || (e.key === "ArrowLeft" && inSub)) {
      handled();
      if (inSub || openSub !== null) closeSubmenu(true);
      else onClose();
    } else if (e.key === "Tab") {
      handled();
      onClose();
    } else if (!inSub && (e.key === "ArrowRight" || e.key === "Enter" || e.key === " ")) {
      const el = document.activeElement as HTMLElement;
      const i = Number(el?.dataset.i);
      if (Number.isNaN(i)) return;
      if (e.key === "ArrowRight" && !(items[i] as MenuItem).items) return;
      handled();
      activate(i, el, true);
    } else if (inSub && (e.key === "Enter" || e.key === " ")) {
      const el = document.activeElement as HTMLElement;
      const j = Number(el?.dataset.j);
      if (Number.isNaN(j) || !sub?.items) return;
      handled();
      run(sub.items[j] as MenuItem);
    } else if (inSub && sub?.numbered && /^[1-9]$/.test(e.key)) {
      const choices = sub.items!.filter((x): x is MenuItem => x !== "-");
      const it = choices[Number(e.key) - 1];
      if (it) (handled(), run(it));
    }
  }
</script>

{#snippet lead(it: MenuItem)}
  <span class="lead" aria-hidden="true">
    {#if it.status}<StatusIcon status={it.status} {statuses} />
    {:else if it.badge}<span class="badge" style:background="var(--c-{it.badge.color})">{it.badge.text}</span>
    {:else if it.icon}<Icon name={it.icon} size={15} />{/if}
  </span>
{/snippet}

<!-- Keys are handled on each panel; the backdrop only catches outside presses. -->
<div
  class="popup-backdrop anchored"
  role="presentation"
  use:dismissOnBackdrop={onClose}
  oncontextmenu={(e) => e.preventDefault()}
>
  <div class="menu popup-panel" role="menu" aria-label={label} tabindex="-1" use:placeAt={anchor} bind:this={main} onkeydown={keydown} in:pop>
    {#each items as it, i}
      {#if it === "-"}
        <div class="sep" role="separator"></div>
      {:else}
        <button
          class="item"
          class:danger={it.danger}
          class:open={openSub === i}
          role="menuitem"
          data-i={i}
          tabindex="-1"
          title={it.title}
          aria-disabled={it.disabled ? "true" : undefined}
          aria-haspopup={it.items ? "menu" : undefined}
          aria-expanded={it.items ? openSub === i : undefined}
          onclick={(e) => activate(i, e.currentTarget, false)}
          onpointerenter={(e) => hover(i, e.currentTarget)}
        >
          {@render lead(it)}
          <span class="label">{it.label}</span>
          {#if it.hint}<span class="hint">{it.hint}</span>{/if}
          {#if it.items}<span class="chev"><Icon name="chevron-right" size={14} /></span>{/if}
        </button>
      {/if}
    {/each}
  </div>

  {#if sub?.items && subAnchor}
    {@const numbered = sub.numbered}
    <div class="menu popup-panel sub" role="menu" aria-label={sub.label} tabindex="-1" use:placeAt={subAnchor} bind:this={subEl} onkeydown={keydown}>
      {#each sub.items as it, j}
        {#if it === "-"}
          <div class="sep" role="separator"></div>
        {:else}
          {@const n = sub.items.slice(0, j).filter((x) => x !== "-").length + 1}
          <button
            class="item"
            class:danger={it.danger}
            class:checked={it.checked}
            role={it.checked === undefined ? "menuitem" : "menuitemradio"}
            aria-checked={it.checked === undefined ? undefined : it.checked}
            aria-disabled={it.disabled ? "true" : undefined}
            data-j={j}
            tabindex="-1"
            title={it.title}
            onclick={() => run(it)}
          >
            {@render lead(it)}
            <span class="label">{it.label}</span>
            {#if it.checked}<span class="hint"><Icon name="check" size={14} /></span>
            {:else if it.hint}<span class="hint">{it.hint}</span>
            {:else if numbered && n <= 9}<span class="hint">{n}</span>{/if}
          </button>
        {/if}
      {/each}
    </div>
  {/if}
</div>

<style>
  .menu {
    position: fixed;
    min-width: 220px;
    max-width: min(320px, calc(100vw - 16px));
    max-height: calc(100vh - 16px);
    overflow-y: auto;
    padding: 5px;
    background: var(--bg);
  }
  .sub {
    min-width: 180px;
    z-index: 1;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 30px;
    padding: 0 8px;
    font: inherit;
    font-size: 13px;
    color: var(--fg);
    text-align: left;
    white-space: nowrap;
    background: transparent;
    border: none;
    border-radius: 6px;
    cursor: pointer;
  }
  .item:hover:not([aria-disabled="true"]),
  .item:focus-visible,
  .item.open {
    background: var(--hover);
  }
  .item:focus-visible {
    outline: none;
    box-shadow: inset 0 0 0 1.5px var(--accent);
  }
  .item.checked {
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }
  .item[aria-disabled="true"] {
    opacity: 0.45;
    cursor: default;
  }
  .danger {
    color: var(--c-red);
  }
  .lead {
    display: flex;
    justify-content: center;
    width: 16px;
    flex-shrink: 0;
    color: var(--fg-muted);
  }
  .danger .lead {
    color: inherit;
  }
  .badge {
    display: grid;
    place-items: center;
    width: 16px;
    height: 16px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 700;
    color: #fff;
  }
  .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .hint {
    display: flex;
    max-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 11px;
    color: var(--fg-muted);
  }
  .checked .hint {
    color: inherit;
  }
  .chev {
    display: flex;
    margin-right: -2px;
    color: var(--fg-muted);
  }
  .sep {
    height: 1px;
    margin: 4px 6px;
    background: var(--border);
  }
</style>
