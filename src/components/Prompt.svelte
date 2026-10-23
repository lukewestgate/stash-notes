<script lang="ts">
  import { dim, pop } from "../lib/motion";
  import { dismissOnBackdrop } from "../lib/popup";
  import { onMount } from "svelte";

  interface Props {
    title: string;
    value: string;
    placeholder?: string;
    onSubmit: (value: string) => void;
    onClose: () => void;
  }
  let { title, value, placeholder = "", onSubmit, onClose }: Props = $props();

  // svelte-ignore state_referenced_locally
  let text = $state(value);
  let input: HTMLInputElement;

  onMount(() => {
    input.focus();
    input.select();
  });

  function keydown(e: KeyboardEvent) {
    if (e.key === "Enter" && text.trim()) {
      e.preventDefault();
      onSubmit(text.trim());
    }
  }
</script>

<div class="backdrop" role="presentation" in:dim use:dismissOnBackdrop={onClose}>
  <div class="prompt" in:pop role="dialog" aria-label={title}>
    <label>
      <span>{title}</span>
      <input bind:this={input} bind:value={text} {placeholder} onkeydown={keydown} spellcheck="false" autocomplete="off" />
    </label>
    <div class="hint"><kbd>Enter</kbd> confirm · <kbd>Esc</kbd> cancel</div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 14vh;
    background: var(--scrim);
    z-index: 25;
  }
  .prompt {
    width: min(520px, calc(100vw - 32px));
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: var(--shadow);
    padding: 12px 14px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 12px;
    color: var(--fg-muted);
  }
  input {
    font: inherit;
    font-size: 14px;
    color: var(--fg);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 7px 9px;
    outline: none;
  }
  input:focus {
    border-color: var(--accent);
  }
  .hint {
    margin-top: 8px;
    font-size: 11.5px;
    color: var(--fg-muted);
  }
  kbd {
    font: inherit;
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 0 4px;
  }
</style>
