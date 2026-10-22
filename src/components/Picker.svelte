<script lang="ts">
  import { dim, pop } from "../lib/motion";
  import { onMount } from "svelte";

  interface Props {
    title: string;
    items: { label: string; detail?: string }[];
    onChoose: (index: number) => void;
  }
  let { title, items, onChoose }: Props = $props();

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

<div class="backdrop" role="presentation" in:dim>
  <div class="picker" in:pop role="listbox" aria-label={title} tabindex="-1" bind:this={box} onkeydown={keydown}>
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
        <span class="label">{item.label}</span>
        {#if item.detail && item.detail !== item.label}<span class="detail">{item.detail}</span>{/if}
      </button>
    {/each}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 14vh;
    background: var(--scrim);
    z-index: 25;
  }
  .picker {
    width: min(560px, calc(100vw - 32px));
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: var(--shadow);
    padding: 6px;
    outline: none;
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
  .detail {
    color: var(--fg-muted);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
