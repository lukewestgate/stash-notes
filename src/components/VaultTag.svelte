<script lang="ts">
  import type { VaultDef } from "../lib/api";

  let { vault, vaults }: { vault: string; vaults: VaultDef[] } = $props();
  const def = $derived(vaults.find((v) => v.id === vault));
</script>

{#if def}
  <span class="vault" style:--c="var(--c-{def.color})" title="In vault {def.name}"><span class="dot"></span>{def.name}</span>
{/if}

<style>
  .vault {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    line-height: 16px;
    color: var(--fg-muted);
    white-space: nowrap;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 2px;
    background: var(--c);
  }
</style>
