<script lang="ts">
  let { tag, colors, onclick }: { tag: string; colors: Record<string, string>; onclick?: () => void } = $props();
  // A nested tag (bug/ui) takes its parent's color unless it has its own.
  const color = $derived(colors[tag] ?? colors[tag.split("/")[0]] ?? null);
</script>

{#if onclick}
  <button class="tag" class:colored={color} style:--c={color ? `var(--c-${color})` : null} {onclick} title="Notes tagged #{tag}">#{tag}</button>
{:else}
  <span class="tag" class:colored={color} style:--c={color ? `var(--c-${color})` : null}>#{tag}</span>
{/if}

<style>
  .tag {
    display: inline-flex;
    align-items: center;
    font: inherit;
    font-size: 11px;
    line-height: 16px;
    padding: 0 6px;
    border-radius: 999px;
    white-space: nowrap;
    color: var(--fg-muted);
    background: color-mix(in srgb, var(--fg-muted) 12%, transparent);
    border: none;
  }
  .tag.colored {
    color: var(--c);
    background: color-mix(in srgb, var(--c) 14%, transparent);
  }
  button.tag {
    cursor: pointer;
  }
  button.tag:hover {
    background: color-mix(in srgb, var(--c, var(--fg-muted)) 24%, transparent);
  }
</style>
