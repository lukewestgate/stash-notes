<script lang="ts">
  // The quick switcher pop-up (Alt+J): Ctrl+K's switcher in a small window
  // of its own. Choosing or creating a note opens it in the main window.
  import { onMount } from "svelte";
  import Switcher from "./components/Switcher.svelte";
  import { api, on, type SearchHit, type Settings } from "./lib/api";
  import { applyAppearance } from "./lib/appearance";
  import { anyTickets, ticketFromUrl } from "./lib/links";

  let settings = $state<Settings | null>(null);
  /** Bumped each time the pop-up is shown, to start from an empty query. */
  let shown = $state(0);
  let error = $state("");

  function isTicket(q: string) {
    const first = q.trim().split(/\s+/)[0] ?? "";
    const re = anyTickets(settings?.vaults ?? []);
    return !!re && new RegExp(`^(?:${re.source})$`).test(first.toUpperCase());
  }

  function pasteKey(text: string): string | null {
    if (!settings?.smartPaste) return null;
    return ticketFromUrl(text, settings.vaults)?.key ?? null;
  }

  async function choose(hit: SearchHit) {
    try {
      await api.quickOpen(hit.path);
    } catch (e) {
      error = String(e);
    }
  }

  async function create(query: string, vault: string | null) {
    try {
      await api.quickCreate(query, vault);
    } catch (e) {
      error = String(e);
    }
  }

  function use(s: Settings) {
    settings = s;
    applyAppearance(s);
  }

  onMount(() => {
    const unlisten = [
      on<null>("quick-show", () => {
        error = "";
        shown++;
      }),
      on<Settings>("settings-changed", use),
    ];
    api.getSettings().then(use);
    return () => unlisten.forEach((p) => p.then((f) => f()));
  });
</script>

<main class="quick">
  {#if settings}
    {#key shown}
      <Switcher
        embedded
        mode="open"
        current={null}
        {isTicket}
        statuses={settings.statuses}
        tagColors={settings.tagColors}
        vaults={settings.vaults}
        currentVault={null}
        {pasteKey}
        onChoose={choose}
        onCreate={create}
        onClose={() => api.quickHide()}
        onSize={(h) => api.quickResize(h + (error ? 24 : 0) + 2)}
      />
    {/key}
    {#if error}<div class="error">{error}</div>{/if}
  {/if}
</main>

<style>
  :global(html[data-window="quick"]),
  :global(html[data-window="quick"] body) {
    background: var(--bg-raised);
  }
  .quick {
    border: 1px solid var(--border);
    box-sizing: border-box;
    min-height: 100vh;
  }
  .error {
    padding: 4px 12px;
    font-size: 12px;
    color: var(--c-red);
  }
</style>
