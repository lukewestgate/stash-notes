<script lang="ts">
  import { iconNode, svgAttrs } from "../lib/icons";

  interface Props {
    name: string;
    size?: number;
  }
  let { name, size = 16 }: Props = $props();
  const node = $derived(iconNode(name));
</script>

<svg {...svgAttrs(size)}>
  {#each node as [tag, attrs]}
    <svelte:element this={tag} {...attrs} />
  {/each}
</svg>

<style>
  svg {
    flex: none;
    display: block;
  }
</style>
