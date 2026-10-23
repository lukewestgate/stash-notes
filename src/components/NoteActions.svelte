<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "./Icon.svelte";
  import { dim, pop } from "../lib/motion";

  let { title, canRename, canStatus, onRename, onMove, onDelete, onStatus, onClose }: {
    title: string; canRename: boolean; canStatus: boolean;
    onRename: () => void; onMove: () => void; onDelete: () => void;
    onStatus: () => void; onClose: () => void;
  } = $props();
  let box: HTMLDivElement;
  onMount(() => box.querySelector<HTMLButtonElement>(".item:not(:disabled)")?.focus());
  function keydown(event: KeyboardEvent) {
    if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
    event.preventDefault();
    const buttons = [...box.querySelectorAll<HTMLButtonElement>("button:not(:disabled)")];
    const current = buttons.indexOf(document.activeElement as HTMLButtonElement);
    const next = event.key === "Home" ? 0 : event.key === "End" ? buttons.length - 1 :
      (current + (event.key === "ArrowUp" ? -1 : 1) + buttons.length) % buttons.length;
    buttons[next]?.focus();
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="backdrop" role="presentation" in:dim onclick={(event) => { if (event.target === event.currentTarget) onClose(); }}>
  <div class="note-actions" role="dialog" aria-modal="true" aria-label="Note actions for {title}" tabindex="-1" bind:this={box} onkeydown={keydown} in:pop>
    <div class="heading"><span>{title}</span><button class="close" onclick={onClose} aria-label="Close note actions"><Icon name="x" size={15} /></button></div>
    <button class="item" onclick={onRename} disabled={!canRename} title={canRename ? "Rename note" : "Daily and ticket files keep their date or key"}><Icon name="pencil" size={16} /><span>Rename…</span><kbd aria-hidden="true">F2</kbd></button>
    <button class="item" onclick={onMove}><Icon name="folder-input" size={16} /><span>Move…</span></button>
    {#if canStatus}<button class="item" onclick={onStatus}><Icon name="circle-dashed" size={16} /><span>Set status…</span></button>{/if}
    <div class="separator"></div>
    <button class="item danger" onclick={onDelete}><Icon name="trash" size={16} /><span>Delete…</span></button>
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; display: flex; justify-content: center; align-items: flex-start; padding-top: 14vh; background: var(--scrim); z-index: 25; }
  .note-actions { width: min(360px, calc(100vw - 32px)); background: var(--bg-raised); color: var(--fg); border: 1px solid var(--border); border-radius: 10px; box-shadow: var(--shadow); padding: 6px; }
  .heading { display: flex; align-items: center; gap: 8px; padding: 3px 6px 8px; color: var(--fg-muted); font-size: 12px; }
  .heading span { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  button { font: inherit; color: inherit; background: transparent; border: none; border-radius: 6px; cursor: pointer; }
  .close { display: flex; padding: 4px; }
  .item { display: flex; align-items: center; gap: 10px; width: 100%; padding: 9px 10px; text-align: left; font-size: 13px; }
  .item span { flex: 1; }
  button:hover { background: var(--hover); }
  button:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
  button:disabled { opacity: .4; cursor: default; }
  .danger { color: var(--c-red); }
  .separator { height: 1px; margin: 5px; background: var(--border); }
  kbd { font: inherit; color: var(--fg-muted); font-size: 11px; }
</style>
