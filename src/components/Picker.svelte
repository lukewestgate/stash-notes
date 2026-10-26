<script lang="ts">
  import { dim, pop } from "../lib/motion";
  import { dismissOnBackdrop, placeAt, type Anchor } from "../lib/popup";
  import { onMount } from "svelte";

  interface Props {
    title: string;
    /** `color`: a theme color name for a swatch before the label. */
    items: { label: string; detail?: string; color?: string }[];
    onChoose: (index: number) => void;
    onClose: () => void;
    /** Open beside this (e.g. the button that opened it) instead of centered. */
    anchor?: Anchor | null;
  }
  let { title, items, onChoose, onClose, anchor = null }: Props = $props();

  let sel = $state(0);
  let box: HTMLDivElement;

  onMount(() => box.focus());

  function keydown(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      sel = (sel + 1) % items.length;
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      sel = (sel - 1 + items.length) % items.length;
    } else if (e.key === "Enter") {
      e.preventDefault();
      onChoose(sel);
    } else if (/^[1-9]$/.test(e.key) && Number(e.key) <= items.length) {
      e.preventDefault();
      onChoose(Number(e.key) - 1);
    }
  }
</script>

<div class="popup-backdrop" class:anchored={!!anchor} role="presentation" in:dim use:dismissOnBackdrop={onClose}>
  <div class="picker popup-panel" class:narrow={!!anchor} use:placeAt={anchor} in:pop role="listbox" aria-label={title} tabindex="-1" bind:this={box} onkeydown={keydown}>
    <div class="title">{title}</div>
    {#each items as item, i}
      <button
        class="row"
        class:selected={i === sel}
        role="option"
        aria-selected={i === sel}
        onmousemove={() => (sel = i)}
        onclick={() => onChoose(i)}
      >
        <span class="n">{i + 1}</span>
        {#if item.color}<span class="swatch" style:background="var(--c-{item.color})"></span>{/if}
        <span class="label">{item.label}</span>
        {#if item.detail && item.detail !== item.label}<span class="detail">{item.detail}</span>{/if}
      </button>
    {/each}
  </div>
</div>

<style>
  .picker {
    width: min(560px, calc(100vw - 32px));
    padding: 6px;
  }
  .picker.narrow {
    width: min(300px, calc(100vw - 16px));
  }
  .title {
    font-size: 12px;
    color: var(--fg-muted);
    padding: 4px 8px 6px;
  }
  .row {
    display: flex;
    align-items: baseline;
    gap: 10px;
    width: 100%;
    font: inherit;
    text-align: left;
    color: var(--fg);
    background: transparent;
    border: none;
    border-radius: 6px;
    padding: 6px 8px;
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
  }
  .row.selected {
    background: var(--selected);
  }
  .n {
    color: var(--fg-muted);
    font-size: 12px;
  }
  .swatch {
    width: 9px;
    height: 9px;
    border-radius: 3px;
    flex: none;
    align-self: center;
  }
  .detail {
    color: var(--fg-muted);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
