<script lang="ts">
  import { onMount } from "svelte";
  import type { StatusDef } from "../lib/api";
  import StatusIcon from "./StatusIcon.svelte";

  interface Props {
    statuses: StatusDef[];
    current: string | null;
    title: string;
    onChoose: (status: string | null) => void;
  }
  let { statuses, current, title, onChoose }: Props = $props();

  let filter = $state("");
  let sel = $state(0);
  let input: HTMLInputElement;

  type Item = { name: string | null; label: string };
  const items = $derived<Item[]>(
    [...statuses.map((s) => ({ name: s.name, label: s.name })), { name: null, label: "no status" }].filter((i) =>
      i.label.toLowerCase().includes(filter.trim().toLowerCase()),
    ),
  );

  onMount(() => {
    input.focus();
    const i = statuses.findIndex((s) => s.name === current);
    sel = i >= 0 ? i : 0;
  });

  function keydown(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      sel = (sel + 1) % Math.max(items.length, 1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      sel = (sel - 1 + items.length) % Math.max(items.length, 1);
    } else if (e.key === "Enter" && items[sel]) {
      e.preventDefault();
      onChoose(items[sel].name);
    } else if (/^[1-9]$/.test(e.key) && !filter && Number(e.key) <= statuses.length) {
      e.preventDefault();
      onChoose(statuses[Number(e.key) - 1].name);
    }
  }
</script>

<div class="backdrop" role="presentation">
  <div class="picker" role="dialog" aria-label={title}>
    <input bind:this={input} bind:value={filter} oninput={() => (sel = 0)} onkeydown={keydown} placeholder={title} spellcheck="false" />
    {#each items as item, i (item.label)}
      <button class="row" class:selected={i === sel} onmousemove={() => (sel = i)} onclick={() => onChoose(item.name)}>
        <span class="n">{item.name && !filter ? statuses.findIndex((s) => s.name === item.name) + 1 : ""}</span>
        {#if item.name}<StatusIcon status={item.name} {statuses} />{:else}<span class="none">–</span>{/if}
        <span class="label">{item.label}</span>
        {#if item.name === current}<span class="cur">current</span>{/if}
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
    width: min(300px, calc(100vw - 32px));
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: var(--shadow);
    padding: 6px;
  }
  input {
    width: 100%;
    font: inherit;
    color: var(--fg);
    caret-color: currentColor;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 6px 8px;
    margin-bottom: 4px;
    outline: none;
  }
  .row {
    display: flex;
    align-items: baseline;
    gap: 8px;
    width: 100%;
    font: inherit;
    color: var(--fg);
    background: transparent;
    border: none;
    border-radius: 6px;
    padding: 5px 8px;
    cursor: pointer;
    text-align: left;
  }
  .row.selected {
    background: var(--selected);
  }
  .n {
    width: 10px;
    color: var(--fg-muted);
    font-size: 12px;
  }
  .none {
    width: 1.1em;
    text-align: center;
    color: var(--fg-muted);
  }
  .label {
    flex: 1;
  }
  .cur {
    font-size: 11px;
    color: var(--fg-muted);
  }
</style>
