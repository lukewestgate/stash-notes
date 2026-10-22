<script lang="ts">
  import type { StatusDef } from "../lib/api";
  import { statusInfo } from "../lib/status";

  interface Props {
    status: string | null;
    statuses: StatusDef[];
    /** Show the name next to the symbol (header chip). */
    label?: boolean;
  }
  let { status, statuses, label = false }: Props = $props();
  const info = $derived(status ? statusInfo(status, statuses) : null);
</script>

{#if info}
  <span class="status" style="--c: var(--c-{info.color})" title={info.known ? info.name : `unknown status: ${info.name}`}
    ><span class="sym">{info.symbol}</span>{#if label}<span class="name">{info.name}</span>{/if}</span
  >
{/if}

<style>
  .status {
    display: inline-flex;
    align-items: baseline;
    gap: 5px;
    color: var(--c);
    white-space: nowrap;
  }
  .sym {
    display: inline-block;
    width: 1.1em;
    text-align: center;
    font-family: "Segoe UI Symbol", "Segoe UI", "DejaVu Sans", sans-serif;
    font-variant-emoji: text;
    line-height: 1;
  }
  .name {
    font-size: 12px;
  }
</style>
