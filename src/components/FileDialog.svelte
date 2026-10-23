<script lang="ts">
  import { onMount } from "svelte";
  import type { VaultDef } from "../lib/api";
  import { dim, pop } from "../lib/motion";
  import Icon from "./Icon.svelte";

  interface Props {
    mode: "move" | "delete";
    title: string;
    path: string;
    vaults: VaultDef[];
    onMove: (vault: string) => Promise<void>;
    onArchive: () => Promise<void>;
    onDelete: () => Promise<void>;
    onClose: () => void;
  }

  let { mode, title, path, vaults, onMove, onArchive, onDelete, onClose }: Props = $props();
  const initialVault = $derived(path.slice(0, path.indexOf(":")));
  const sourceVault = $derived(vaults.find((v) => v.id === initialVault));
  const destinations = $derived(vaults.filter((v) => v.id !== initialVault));
  const archived = $derived(path.slice(path.indexOf(":") + 1).startsWith("archive/"));
  let vault = $state("");
  let pending = $state<"move" | "archive" | "delete" | null>(null);
  const busy = $derived(pending !== null);
  let error = $state("");
  let box: HTMLFormElement;

  onMount(() => box.querySelector<HTMLElement>(
    mode === "move" ? "select:not(:disabled), .cancel" : ".archive",
  )?.focus());

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    await run(mode === "move" ? "move" : "archive");
  }

  async function run(action: "move" | "archive" | "delete") {
    if (busy || (action === "move" && !vault)) return;
    pending = action;
    error = "";
    try {
      if (action === "move") await onMove(vault);
      else if (action === "archive") await onArchive();
      else await onDelete();
    } catch (e) {
      error = String(e);
    } finally {
      pending = null;
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
  class="backdrop"
  role="presentation"
  in:dim
  onclick={(event) => { if (!busy && event.target === event.currentTarget) onClose(); }}
>
  <div
    class="file-dialog"
    role="dialog"
    aria-modal="true"
    aria-label={mode === "move" ? "Move note" : "Delete note"}
    aria-busy={busy}
    tabindex="-1"
    in:pop
  >
    <form bind:this={box} onsubmit={submit}>
      <div class="dialog-header">
        <h2>{mode === "move" ? "Move note" : "Delete note?"}</h2>
        <button type="button" class="close" disabled={busy} onclick={onClose} aria-label="Close dialog">
          <Icon name="x" size={16} />
        </button>
      </div>

      {#if mode === "move"}
        <p>Move <strong>{title}</strong> from {sourceVault?.name ?? initialVault} to another vault.</p>
        <label>
          Destination vault
          <select bind:value={vault} disabled={busy || !destinations.length} required>
            <option value="" disabled>Choose a vault</option>
            {#each destinations as v (v.id)}<option value={v.id}>{v.name}</option>{/each}
          </select>
        </label>
        {#if !destinations.length}<p class="hint">Add another vault in Settings to move notes.</p>{/if}
      {:else}
        <p>Are you sure you want to delete <strong>{title}</strong>?</p>
        <p class="hint">
          {archived
            ? "This note is already archived. You can keep it there instead."
            : "Archiving is recommended if you want to keep this note."}
        </p>
      {/if}

      {#if error}<p class="error" role="alert">{error}</p>{/if}

      <div class="buttons">
        <button type="button" class="cancel" disabled={busy} onclick={onClose}>Cancel</button>
        {#if mode === "move"}
          <button type="submit" class="primary" disabled={busy || !vault}>{pending === "move" ? "Moving…" : "Move note"}</button>
        {:else}
          <button
            type="submit"
            class="archive primary"
            disabled={busy}
            aria-label="{archived ? 'Keep archived' : 'Archive note'} Recommended"
          >{pending === "archive" ? "Working…" : archived ? "Keep archived" : "Archive"}</button>
          <button
            type="button"
            class="danger"
            disabled={busy}
            onclick={() => run("delete")}
            aria-label="Delete note"
          >{pending === "delete" ? "Deleting…" : "Delete"}</button>
        {/if}
      </div>
    </form>
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
  .file-dialog {
    width: 100%;
    max-width: 400px;
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
    line-height: 1.4;
  }
  p {
    margin: 0;
    overflow-wrap: anywhere;
    font-size: 13px;
    line-height: 1.5;
  }
  strong { font-weight: 600; }
  .hint {
    margin-top: 8px;
    color: var(--fg-muted);
    font-size: 12px;
  }
  button, select { font: inherit; color: inherit; }
  button {
    min-height: 34px;
    padding: 6px 13px;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
  }
  button:hover:not(:disabled) {
    background: var(--hover);
    border-color: var(--fg-muted);
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
  .close:hover:not(:disabled) { color: var(--fg); border-color: transparent; }
  .buttons {
    display: flex;
    justify-content: flex-end;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 24px;
  }
  .primary {
    color: var(--bg);
    background: var(--accent);
    border-color: var(--accent);
  }
  .primary:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 85%, var(--fg));
    border-color: var(--accent);
  }
  .danger {
    color: var(--c-red);
    background: transparent;
    border-color: color-mix(in srgb, var(--c-red) 35%, var(--border));
  }
  .danger:hover:not(:disabled) {
    background: color-mix(in srgb, var(--c-red) 8%, var(--bg));
    border-color: var(--c-red);
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 16px;
    font-size: 12px;
    font-weight: 500;
  }
  select {
    width: 100%;
    min-height: 34px;
    padding: 6px 10px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 13px;
  }
  select:hover:not(:disabled) { border-color: var(--fg-muted); }
  :is(button, select):focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  button:disabled, select:disabled { opacity: 0.5; cursor: default; }
  .error { margin-top: 12px; color: var(--c-red); font-size: 12px; }
</style>
