<script lang="ts">
  // The quick capture box (spec section 14). Enter saves the line to its note
  // through Rust and hides; Esc hides; Shift+Enter adds a line.
  import { onMount, tick } from "svelte";
  import { api, on, type Settings } from "./lib/api";
  import { jiraKeyFromUrl, ticketRegex } from "./lib/text";

  let settings = $state<Settings | null>(null);
  let text = $state("");
  let error = $state("");
  let done = $state<string | null>(null);
  let busy = false;
  let box: HTMLTextAreaElement;

  const tickets = $derived(ticketRegex(settings?.ticketPattern ?? "[A-Z][A-Z0-9]+-\\d+"));

  /** Where the text will go, as shown under the box. */
  const target = $derived.by(() => {
    const t = text.trim();
    if (/^\[ ?\]/.test(t)) return "Today · Todo";
    const first = t.split(/\s+/)[0] ?? "";
    const full = new RegExp(`^(?:${settings?.ticketPattern ?? "[A-Z][A-Z0-9]+-\\d+"})$`);
    const key = full.test(first.replace(/[:,]$/, "").toUpperCase())
      ? first.replace(/[:,]$/, "").toUpperCase()
      : settings
        ? jiraKeyFromUrl(first, tickets, settings.jira.urlTemplate)
        : null;
    return key ?? "Today";
  });

  async function fit() {
    await tick();
    if (!box) return;
    box.style.height = "auto";
    const lines = Math.min(5, Math.max(1, text.split("\n").length));
    box.style.height = `${box.scrollHeight}px`;
    api.captureResize(52 + lines * 22).catch(() => {});
  }

  async function submit() {
    if (busy || !text.trim()) return;
    busy = true;
    error = "";
    try {
      const r = await api.captureSubmit(text);
      done = r.target;
      text = "";
      fit();
      setTimeout(() => {
        done = null;
        api.captureHide();
      }, 400);
    } catch (e) {
      // Nothing was written: stay open with the text.
      error = String(e instanceof Error ? e.message : e);
    } finally {
      busy = false;
    }
  }

  function keydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      submit();
    } else if (e.key === "Escape") {
      e.preventDefault();
      error = "";
      api.captureHide();
    }
  }

  function paste(e: ClipboardEvent) {
    if (!settings?.smartPaste || text.trim()) return;
    const key = jiraKeyFromUrl(e.clipboardData?.getData("text/plain") ?? "", tickets, settings.jira.urlTemplate);
    if (key) {
      e.preventDefault();
      text = `${key} `;
    }
  }

  onMount(() => {
    const unlisten = [
      on<null>("capture-show", () => {
        done = null;
        error = "";
        fit();
        box?.focus();
        box?.select();
      }),
      on<Settings>("settings-changed", (s) => (settings = s)),
    ];
    api.getSettings().then((s) => {
      settings = s;
      const root = document.documentElement;
      if (s.theme === "light" || s.theme === "dark") root.dataset.theme = s.theme;
    });
    box.focus();
    fit();
    return () => unlisten.forEach((p) => p.then((f) => f()));
  });
</script>

<main class="capture">
  <div class="row">
    <span class="mark" class:ok={done}>{done ? "✓" : "+"}</span>
    <textarea
      bind:this={box}
      bind:value={text}
      oninput={fit}
      onkeydown={keydown}
      onpaste={paste}
      rows="1"
      placeholder="Capture a note — start with a ticket key to file it there, [] for a todo"
      spellcheck="false"
    ></textarea>
  </div>
  <div class="target" class:error={!!error}>
    {#if error}{error}{:else if done}Saved to {done}{:else}→ {target}<span class="keys">Enter save · Shift+Enter new line · Esc cancel</span>{/if}
  </div>
</main>

<style>
  :global(html[data-window="capture"]),
  :global(html[data-window="capture"] body) {
    background: var(--bg-raised);
  }
  .capture {
    height: 100vh;
    display: flex;
    flex-direction: column;
    justify-content: center;
    padding: 8px 14px 6px;
    border: 1px solid var(--border);
    border-radius: 10px;
    box-sizing: border-box;
    background: var(--bg-raised);
  }
  .row {
    display: flex;
    align-items: flex-start;
    gap: 10px;
  }
  .mark {
    width: 18px;
    line-height: 28px;
    font-size: 18px;
    color: var(--fg-muted);
    text-align: center;
  }
  .mark.ok {
    color: var(--ok);
  }
  textarea {
    flex: 1;
    resize: none;
    font: inherit;
    font-size: 16px;
    line-height: 22px;
    padding: 3px 0;
    color: var(--fg);
    background: transparent;
    border: none;
    outline: none;
    overflow-y: auto;
    max-height: calc(5 * 22px + 6px);
  }
  .target {
    padding-left: 28px;
    font-size: 12px;
    color: var(--fg-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .target.error {
    color: var(--c-red);
  }
  .keys {
    margin-left: 12px;
    opacity: 0.7;
  }
</style>
