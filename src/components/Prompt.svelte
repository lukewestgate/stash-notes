<script lang="ts">
  import { dim, pop } from "../lib/motion";
  import { dismissOnBackdrop, placeAt, type Anchor } from "../lib/popup";
  import { onMount } from "svelte";

  interface Props {
    title: string;
    value: string;
    placeholder?: string;
    onSubmit: (value: string) => void;
    onClose: () => void;
    /** Open beside this (e.g. the button that opened it) instead of centered. */
    anchor?: Anchor | null;
  }
  let { title, value, placeholder = "", onSubmit, onClose, anchor = null }: Props = $props();

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

<div class="popup-backdrop" class:anchored={!!anchor} role="presentation" in:dim use:dismissOnBackdrop={onClose}>
  <div class="prompt popup-panel" class:narrow={!!anchor} use:placeAt={anchor} in:pop role="dialog" aria-label={title}>
    <label>
      <span>{title}</span>
      <input bind:this={input} bind:value={text} {placeholder} onkeydown={keydown} spellcheck="false" autocomplete="off" />
    </label>
    <div class="hint"><kbd>Enter</kbd> confirm · <kbd>Esc</kbd> cancel</div>
  </div>
</div>

<style>
  .prompt {
    width: min(520px, calc(100vw - 32px));
    padding: 12px 14px;
  }
  .prompt.narrow {
    width: min(360px, calc(100vw - 16px));
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
