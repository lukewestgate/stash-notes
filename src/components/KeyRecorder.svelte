<script lang="ts">
  // Click, then press the new keys. Esc cancels.
  import { formatKey, keyCapture, specFromEvent } from "../lib/keys";

  interface Props {
    value: string;
    /** Return an error message to reject the keys (e.g. taken by another app). */
    onChange: (spec: string) => Promise<string | null> | string | null;
    conflict?: string | null;
    /** Show chords ("Ctrl+E P") as typed. */
    chord?: boolean;
  }
  let { value, onChange, conflict = null, chord = false }: Props = $props();

  let recording = $state(false);
  let error = $state<string | null>(null);
  let first: string | null = null;
  let button: HTMLButtonElement;

  function stopRecording() {
    recording = false;
    keyCapture.active = false;
    window.removeEventListener("keydown", onKey, true);
    first = null;
  }

  async function onKey(e: KeyboardEvent) {
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape" && !e.ctrlKey && !e.altKey) {
      stopRecording();
      return;
    }
    const spec = specFromEvent(e);
    if (!spec) return;
    if (chord && !first) {
      // First half of a chord; the second key completes it.
      first = spec;
      return;
    }
    const full = chord ? `${first} ${spec.replace(/^Ctrl\+/, "")}` : spec;
    stopRecording();
    error = await onChange(full);
  }

  function start() {
    error = null;
    recording = true;
    keyCapture.active = true;
    window.addEventListener("keydown", onKey, true);
  }
</script>

<span class="recorder">
  <button bind:this={button} class:recording class:bad={!!(error || conflict)} onclick={() => (recording ? stopRecording() : start())} onblur={() => recording && stopRecording()}>
    {#if recording}
      {chord ? "Press the two keys…" : "Press keys…"}
    {:else}
      <kbd>{chord ? value : formatKey(value)}</kbd>
    {/if}
  </button>
  {#if error}<span class="msg">{error}</span>{:else if conflict}<span class="msg">{conflict}</span>{/if}
</span>

<style>
  .recorder {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  button {
    min-width: 120px;
    font: inherit;
    font-size: 12.5px;
    color: var(--fg);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 3px 10px;
    cursor: pointer;
    text-align: left;
  }
  button.recording {
    border-color: var(--accent);
    color: var(--accent);
  }
  button.bad {
    border-color: var(--c-red);
  }
  kbd {
    font: inherit;
  }
  .msg {
    font-size: 12px;
    color: var(--c-red);
  }
</style>
