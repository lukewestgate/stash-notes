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
  .backdrop { position: fixed; inset: 0; z-index: 25; display: flex; justify-content: center; align-items: flex-start; padding: 14vh 16px 16px; background: var(--scrim); }
  .file-dialog { width: min(480px, 100%); max-height: 80vh; overflow: auto; padding: 18px; color: var(--fg); background: var(--bg-raised); border: 1px solid var(--border); border-radius: 10px; box-shadow: var(--shadow); }
  .dialog-header { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin-bottom: 12px; }
  h2 { margin: 0; font-size: 16px; }
  p { margin: 0; font-size: 13px; line-height: 1.5; overflow-wrap: anywhere; }
  .hint { margin-top: 8px; color: var(--fg-muted); font-size: 12px; }
  label { display: flex; flex-direction: column; gap: 6px; margin-top: 12px; font-size: 12px; }
  button, select { font: inherit; color: var(--fg); background: var(--bg); border: 1px solid var(--border); border-radius: 6px; padding: 8px 10px; }
  select { width: 100%; font-size: 14px; }
  button { cursor: pointer; font-size: 13px; }
  .close { display: grid; place-items: center; padding: 4px; border: none; background: transparent; }
  .buttons { display: flex; flex-wrap: wrap; justify-content: flex-end; gap: 8px; margin-top: 18px; }
  .primary { color: var(--bg); background: var(--accent); border-color: var(--accent); }
  .danger, .error { color: var(--c-red); }
  .error { margin-top: 12px; }
  button:disabled, select:disabled { opacity: .5; cursor: default; }
  :is(button, select):focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
</style>
