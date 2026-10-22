<script lang="ts">
  import { highlight } from "../lib/highlight";

  let { text, query }: { text: string; query: string } = $props();
  const parts = $derived(query.trim() ? highlight(text, query) : [{ text, hit: false }]);
</script>

{#each parts as p}{#if p.hit}<mark>{p.text}</mark>{:else}{p.text}{/if}{/each}

<style>
  mark {
    color: inherit;
    font-weight: 600;
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    border-radius: 2px;
  }
</style>
