<script lang="ts">
  import type { StatusDef } from "../lib/api";
  import { statusInfo } from "../lib/status";
  import Icon from "./Icon.svelte";

  interface Props {
    status: string | null;
    statuses: StatusDef[];
    /** Show the name next to the icon (header chip). */
    label?: boolean;
    size?: number;
  }
  let { status, statuses, label = false, size = 14 }: Props = $props();
  const info = $derived(status ? statusInfo(status, statuses) : null);
</script>

{#if info}
  <span class="status" style="--c: var(--c-{info.color})" title={info.known ? info.name : `unknown status: ${info.name}`}
    ><Icon name={info.icon} {size} />{#if label}<span class="name">{info.name}</span>{/if}</span
  >
{/if}

<style>
  .status {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--c);
    white-space: nowrap;
  }
  .name {
    font-size: 12px;
  }
</style>
