<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";

  let { icon, title, children }: { icon: string; title: string; children?: Snippet } = $props();
</script>

<div class="empty">
  <span class="ico"><Icon name={icon} size={28} /></span>
  <div class="title">{title}</div>
  {#if children}<div class="hint">{@render children()}</div>{/if}
</div>

<style>
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 28px 16px;
    text-align: center;
    color: var(--fg-muted);
  }
  .ico {
    display: flex;
    opacity: 0.55;
    margin-bottom: 2px;
  }
  .title {
    font-weight: 600;
    color: var(--fg);
  }
  .hint {
    font-size: 12.5px;
    max-width: 360px;
    line-height: 1.6;
  }
  .hint :global(kbd) {
    font: inherit;
    font-size: 11.5px;
    padding: 0 5px;
    border: 1px solid var(--border);
    border-bottom-width: 2px;
    border-radius: 4px;
    background: var(--bg-raised);
  }
</style>
