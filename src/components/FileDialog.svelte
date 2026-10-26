<script lang="ts">
  import { onMount } from "svelte";
  import { dim, pop } from "../lib/motion";
  import { dismissOnBackdrop } from "../lib/popup";
  import Icon from "./Icon.svelte";

  // Confirms deleting a note, recommending Archive instead. The one note
  // action that still asks in a dialog; the rest are in the note menu.
  interface Props {
    title: string;
    path: string;
    onArchive: () => Promise<void>;
    onDelete: () => Promise<void>;
    onClose: () => void;
  }

  let { title, path, onArchive, onDelete, onClose }: Props = $props();
  const archived = $derived(path.slice(path.indexOf(":") + 1).startsWith("archive/"));
  let pending = $state<"archive" | "delete" | null>(null);
  const busy = $derived(pending !== null);
  let error = $state("");
  let box: HTMLFormElement;

  onMount(() => box.querySelector<HTMLElement>(".archive")?.focus());

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    await run("archive");
  }

  async function run(action: "archive" | "delete") {
    if (busy) return;
    pending = action;
    error = "";
    try {
      if (action === "archive") await onArchive();
      else await onDelete();
    } catch (e) {
      error = String(e);
    } finally {
      pending = null;
    }
  }
</script>

<div
  class="backdrop"
  role="presentation"
  in:dim
  use:dismissOnBackdrop={() => { if (!busy) onClose(); }}
>
  <div
    class="file-dialog"
    role="dialog"
    aria-modal="true"
    aria-label="Delete note"
    aria-busy={busy}
    tabindex="-1"
    in:pop
  >
    <form bind:this={box} onsubmit={submit}>
      <div class="dialog-header">
        <h2>Delete note?</h2>
        <button type="button" class="close" disabled={busy} onclick={onClose} aria-label="Close dialog">
          <Icon name="x" size={16} />
        </button>
      </div>

      <p>Are you sure you want to delete <strong>{title}</strong>?</p>
      <p class="hint">
        {archived
          ? "This note is already archived. You can keep it there instead."
          : "Archiving is recommended if you want to keep this note."}
      </p>

      {#if error}<p class="error" role="alert">{error}</p>{/if}

      <div class="buttons">
        <button type="button" class="cancel" disabled={busy} onclick={onClose}>Cancel</button>
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
  button { font: inherit; color: inherit; }
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
  button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  button:disabled { opacity: 0.5; cursor: default; }
  .error { margin-top: 12px; color: var(--c-red); font-size: 12px; }
</style>
