<script lang="ts">
  import { fly } from "svelte/transition";
  import { toaster, type ToastKind } from "../lib/toasts.svelte";
  import { motion } from "../lib/motion";
  import Icon from "./Icon.svelte";

  const ICON: Record<ToastKind, string> = {
    info: "info",
    done: "circle-check",
    warn: "triangle-alert",
    error: "circle-alert",
  };
</script>

<div
  class="toasts"
  aria-live="polite"
  role="status"
  onpointerenter={() => toaster.pause()}
  onpointerleave={() => toaster.resume()}
>
  {#each toaster.list as t (t.id)}
    <div class="toast {t.kind}" transition:fly={{ x: 24, duration: motion(160) }} role={t.kind === "error" ? "alert" : undefined}>
      <span class="ico"><Icon name={ICON[t.kind]} size={16} /></span>
      <span class="text">{t.text}</span>
      {#if t.action}
        <button
          class="action"
          onclick={() => {
            t.action!.run();
            toaster.dismiss(t.id);
          }}>{t.action.label}</button
        >
      {/if}
      <button class="close" onclick={() => toaster.dismiss(t.id)} aria-label="Dismiss" title="Dismiss"
        ><Icon name="x" size={14} /></button
      >
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: 12px;
    bottom: 36px;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 6px;
    z-index: 40;
    pointer-events: none;
  }
  .toast {
    display: flex;
    align-items: center;
    gap: 9px;
    max-width: min(520px, calc(100vw - 24px));
    padding: 6px 6px 6px 10px;
    background: var(--toast-bg);
    color: var(--toast-fg);
    border-radius: 8px;
    box-shadow: var(--shadow);
    font-size: 13px;
    pointer-events: auto;
  }
  .ico {
    display: flex;
    opacity: 0.85;
  }
  .done .ico {
    color: var(--toast-ok, #4ade80);
  }
  .warn .ico {
    color: var(--toast-warn, #fbbf24);
  }
  .error .ico {
    color: var(--toast-error, #f87171);
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  button {
    font: inherit;
    color: inherit;
    background: transparent;
    border: none;
    cursor: pointer;
  }
  .action {
    font-size: 12.5px;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: 5px;
    border: 1px solid color-mix(in srgb, currentColor 35%, transparent);
  }
  .action:hover,
  .close:hover {
    background: color-mix(in srgb, currentColor 14%, transparent);
  }
  .close {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 5px;
    opacity: 0.7;
  }
  .close:hover {
    opacity: 1;
  }
</style>
