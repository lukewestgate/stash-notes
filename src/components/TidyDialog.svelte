<script lang="ts">
  // Review and run a vault's tidy into 0.8's layout: every note in notes/,
  // templates and trash in .stash/. Shows what will move before anything does.
  import { onMount } from "svelte";
  import type { TidyPreview } from "../lib/api";
  import { dim, pop } from "../lib/motion";
  import { dismissOnBackdrop } from "../lib/popup";
  import Icon from "./Icon.svelte";

  interface Props {
    /** The vault's name. */
    name: string;
    preview: TidyPreview;
    onTidy: () => Promise<void>;
    /** "Not now": don't offer this vault again on startup (Settings still can). */
    onLater: () => void;
    onClose: () => void;
  }
  let { name, preview, onTidy, onLater, onClose }: Props = $props();

  let busy = $state(false);
  let error = $state("");
  let box: HTMLDivElement;

  const plural = (n: number, one: string, many = `${one}s`) => `${n} ${n === 1 ? one : many}`;
  const counts = $derived(
    [
      preview.daily && plural(preview.daily, "daily note"),
      preview.tickets && plural(preview.tickets, "ticket note"),
      preview.notes && plural(preview.notes, "other note"),
    ].filter(Boolean) as string[],
  );

  onMount(() => box.querySelector<HTMLElement>(".primary")?.focus());

  async function tidy() {
    if (busy) return;
    busy = true;
    error = "";
    try {
      await onTidy();
    } catch (e) {
      error = String(e instanceof Error ? e.message : e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="backdrop" role="presentation" in:dim use:dismissOnBackdrop={() => { if (!busy) onClose(); }}>
  <div class="tidy-dialog" role="dialog" aria-modal="true" aria-label="Tidy {name}'s folders" aria-busy={busy} tabindex="-1" bind:this={box} in:pop>
    <div class="dialog-header">
      <h2>Tidy {name}'s folders</h2>
      <button type="button" class="close" disabled={busy} onclick={onClose} aria-label="Close dialog"><Icon name="x" size={16} /></button>
    </div>
    <p>Stash Notes now keeps every note in one <code>notes</code> folder, and its own files in a hidden <code>.stash</code> folder. This vault still uses the older folders.</p>

    <ul class="moves">
      {#if counts.length}
        <li><Icon name="folder-input" size={15} /><span>{counts.join(", ")} move into <code>notes</code></span></li>
      {/if}
      {#if preview.archived}
        <li><Icon name="archive" size={15} /><span>{plural(preview.archived, "archived note")} {preview.archived === 1 ? "stays" : "stay"} archived, marked in {preview.archived === 1 ? "its" : "their"} properties</span></li>
      {/if}
      {#if preview.files}
        <li><Icon name="folder" size={15} /><span>{plural(preview.files, "template or trash file", "template and trash files")} move into <code>.stash</code></span></li>
      {/if}
    </ul>

    {#if preview.renamed.length}
      <div class="detail">
        <span>These names are taken in <code>notes</code>, so they get a number:</span>
        <ul>
          {#each preview.renamed as m (m.from)}<li><code>{m.from}</code> → <code>{m.to}</code></li>{/each}
        </ul>
      </div>
    {/if}
    {#if preview.left.length}
      <div class="detail">
        <span>Left where {preview.left.length === 1 ? "it is" : "they are"}:</span>
        <ul>
          {#each preview.left as f (f)}<li><code>{f}</code></li>{/each}
        </ul>
      </div>
    {/if}

    <p class="hint">File names don't change, so links keep working. Nothing is overwritten, and you can undo it afterwards.</p>
    {#if error}<p class="error" role="alert">{error}</p>{/if}

    <div class="buttons">
      <button type="button" disabled={busy} onclick={onLater}>Not now</button>
      <button type="button" class="primary" disabled={busy} onclick={tidy}>{busy ? "Tidying…" : "Tidy folders"}</button>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 45;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 20px;
    background: var(--scrim);
  }
  .tidy-dialog {
    width: 100%;
    max-width: 460px;
    max-height: calc(100dvh - 40px);
    overflow: auto;
    padding: 20px;
    color: var(--fg);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: var(--shadow);
  }
  .dialog-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 12px;
  }
  h2 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
  }
  p {
    margin: 0 0 12px;
    font-size: 13px;
    line-height: 1.5;
  }
  code {
    font-family: "Cascadia Mono", Consolas, monospace;
    font-size: 12px;
    padding: 0 3px;
    border-radius: 3px;
    background: var(--code-bg);
  }
  .moves {
    list-style: none;
    margin: 0 0 12px;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 13px;
  }
  .moves li {
    display: flex;
    align-items: flex-start;
    gap: 9px;
  }
  .moves :global(svg) {
    flex-shrink: 0;
    margin-top: 2px;
    color: var(--fg-muted);
  }
  .detail {
    margin: 0 0 12px;
    padding: 8px 10px;
    font-size: 12px;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 6px;
  }
  .detail ul {
    margin: 6px 0 0;
    padding-left: 18px;
    max-height: 120px;
    overflow: auto;
  }
  .hint {
    color: var(--fg-muted);
    font-size: 12px;
  }
  .error {
    color: var(--c-red);
    font-size: 12px;
  }
  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 16px;
  }
  button {
    min-height: 34px;
    padding: 6px 13px;
    font: inherit;
    font-size: 12px;
    font-weight: 500;
    color: inherit;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 6px;
    cursor: pointer;
  }
  button:hover:not(:disabled) {
    background: var(--hover);
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .close {
    display: grid;
    place-items: center;
    width: 26px;
    min-height: 26px;
    padding: 0;
    color: var(--fg-muted);
    background: transparent;
    border-color: transparent;
  }
  .primary {
    color: var(--bg);
    background: var(--accent);
    border-color: var(--accent);
  }
  .primary:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 85%, var(--fg));
  }
</style>
