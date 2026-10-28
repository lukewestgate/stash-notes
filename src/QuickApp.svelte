<script lang="ts">
  // The quick switcher pop-up (Alt+J): Ctrl+K's switcher in a small window
  // of its own. Choosing or creating a note opens it in the main window.
  import { onMount } from "svelte";
  import Switcher from "./components/Switcher.svelte";
  import { api, on, type SearchHit, type Settings } from "./lib/api";
  import { applyAppearance } from "./lib/appearance";
  import { isLinkInput } from "./lib/links";
  import { daily } from "./lib/daily.svelte";
  import { isoDate } from "./lib/text";

  let settings = $state<Settings | null>(null);
  /** Bumped each time the pop-up is shown, to start from an empty query. */
  let shown = $state(0);
  let error = $state("");

  function isLink(q: string) {
    return isLinkInput(settings, q);
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

  async function addTodo(text: string, vault: string | null) {
    try {
      await api.addTodo(text, isoDate(), vault);
      await api.quickHide();
    } catch (e) {
      error = String(e);
    }
  }

  function use(s: Settings) {
    settings = s;
    daily.format = s.dailyDateFormat;
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
        {isLink}
        statuses={settings.statuses}
        tagColors={settings.tagColors}
        vaults={settings.vaults}
        currentVault={null}
        onChoose={choose}
        onCreate={create}
        createLink={(input, vault, title, rule) => api.quickCreateFromLink(input, vault, title, rule)}
        onCreatedLink={() => {}}
        onClose={() => api.quickHide()}
        onAddTodo={addTodo}
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
