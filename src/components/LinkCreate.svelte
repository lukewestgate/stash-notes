<script lang="ts">
  // Create a note from a link (or a key a link rule knows): where it goes,
  // its title and what its template makes, before anything is written. Notes
  // already about the link are offered first. Used by the switcher, the
  // Alt+J pop-up and the Notes view.
  import { onMount } from "svelte";
  import { api, type LinkPreview, type SearchHit, type VaultDef } from "../lib/api";
  import Icon from "./Icon.svelte";
  import VaultTag from "./VaultTag.svelte";
  import { noteName } from "../lib/daily.svelte";

  interface Props {
    /** The link or key, then optionally a title: `PROJ-123 Fix the login`. */
    input: string;
    vaults: VaultDef[];
    /** Where it goes unless changed (null: the current vault). */
    vault: string | null;
    /** Create it; resolves to its path. */
    create: (input: string, vault: string | null, title: string, rule: string | null) => Promise<string>;
    onCreated: (path: string) => void;
    onOpen: (hit: SearchHit) => void;
    onCancel: () => void;
  }
  let { input, vaults, vault: initialVault, create, onCreated, onOpen, onCancel }: Props = $props();

  // svelte-ignore state_referenced_locally
  let vault = $state<string | null>(initialVault);
  let title = $state<string | null>(null);
  let rule = $state<string | null>(null);
  let p = $state<LinkPreview | null>(null);
  let error = $state("");
  let busy = $state(false);
  let titleInput = $state<HTMLInputElement>();
  let seq = 0;
  /** The first preview runs at once; later ones wait for typing to pause. */
  let first = true;

  async function refresh() {
    const my = ++seq;
    try {
      const next = await api.linkPreview(input, vault, title, rule);
      if (my !== seq) return;
      p = next;
      vault ??= next.vault;
      title ??= next.title;
      error = "";
    } catch (e) {
      if (my === seq) error = String(e instanceof Error ? e.message : e);
    }
  }

  let timer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    void [input, vault, title, rule];
    clearTimeout(timer);
    timer = setTimeout(refresh, first ? 0 : 120);
    first = false;
  });

  onMount(() => requestAnimationFrame(() => titleInput?.focus()));

  const ready = $derived(!!p?.plan && !p.choices.length && !p.plan.errors.length);
  /** The note body without its front matter, as it would be written. */
  const body = $derived(p?.plan ? p.plan.content.replace(/^---\n[\s\S]*?\n---\n\n?/, "") : "");

  async function go() {
    if (busy) return;
    busy = true;
    try {
      // Create what the preview shows, not what it showed before the last keystroke.
      clearTimeout(timer);
      await refresh();
      if (!ready) return;
      const path = await create(input, vault, title ?? "", rule);
      onCreated(path);
    } catch (e) {
      error = String(e instanceof Error ? e.message : e);
    } finally {
      busy = false;
    }
  }

  /** Enter opens the existing note when there is one; Ctrl+Enter creates another. */
  async function submit(another: boolean) {
    clearTimeout(timer);
    await refresh();
    if (p?.existing.length && !another) onOpen(p.existing[0]);
    else go();
  }

  function keydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      onCancel();
    } else if (e.key === "Enter" && (e.target as HTMLElement).tagName !== "BUTTON") {
      e.preventDefault();
      submit(e.ctrlKey);
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="create" role="group" aria-label="Create note from link" onkeydown={keydown}>
  <div class="head">
    <Icon name={p?.rule?.icon ?? "link"} size={15} />
    <span class="what">Create note from link</span>
    {#if p?.rule}<span class="rule">{p.rule.name}</span>{/if}
  </div>
  {#if p?.url}<div class="url" title={p.url}>{p.url}</div>{/if}

  {#if p && p.choices.length}
    <fieldset class="choices">
      <legend>{p.key} could be more than one link — which is it?</legend>
      {#each p.choices as c (c.rule.id)}
        <label><input type="radio" name="rule" value={c.rule.id} onchange={() => (rule = c.rule.id)} /> <b>{c.rule.name}</b> <span class="muted">{c.url}</span></label>
      {/each}
    </fieldset>
  {:else if p && !p.url}
    <div class="muted">Paste a link, or type a key a link rule knows (Settings → Links).</div>
  {/if}

  <div class="fields">
    <label class="grow">Title
      <input bind:this={titleInput} value={title ?? ""} oninput={(e) => (title = e.currentTarget.value)} placeholder={p?.key ? "What it's about" : "A title"} spellcheck="false" autocomplete="off" />
    </label>
    {#if vaults.length > 1}
      <label>Vault
        <select value={vault ?? ""} onchange={(e) => (vault = e.currentTarget.value)}>
          {#each vaults as v}<option value={v.id}>{v.name}</option>{/each}
        </select>
      </label>
    {/if}
  </div>

  {#if p?.plan}
    <div class="preview" aria-live="polite">
      <div class="pline"><span class="muted">{p.template ? `${p.rule?.name} template` : "No template — a plain note"}</span> · <b>{p.plan.title}</b></div>
      <div class="pline muted">{p.path?.slice(p.path.indexOf(":") + 1)} · keeps the link as its primary link</div>
      {#if p.plan.errors.length}
        <div class="bad">{p.plan.errors.join("; ")}</div>
      {:else}
        <pre>{body.trimEnd()}</pre>
      {/if}
    </div>
  {/if}

  {#if p?.existing.length}
    <div class="existing">
      <span class="muted">Already a note about this link:</span>
      {#each p.existing as h (h.path)}
        <button class="note" onclick={() => onOpen(h)}>
          <Icon name="file-text" size={13} /><span>{h.label ? `${h.title} ${h.label}` : noteName(h)}</span>
          {#if vaults.length > 1}<VaultTag vault={h.vault} {vaults} />{/if}
          {#if h.archived}<span class="muted">archived</span>{/if}
        </button>
      {/each}
    </div>
  {/if}

  {#if error}<div class="bad" role="alert">{error}</div>{/if}

  <div class="buttons">
    {#if p?.existing.length}
      <button class="btn primary" onclick={() => p && onOpen(p.existing[0])}>Open existing <kbd>Enter</kbd></button>
      <button class="btn" disabled={!ready || busy} onclick={go}>Create another note <kbd>Ctrl+Enter</kbd></button>
    {:else}
      <button class="btn primary" disabled={!ready || busy} onclick={go}>Create note <kbd>Enter</kbd></button>
    {/if}
    <button class="btn" onclick={onCancel}>Cancel <kbd>Esc</kbd></button>
  </div>
</div>

<style>
  .create {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px 14px;
    font-size: 13px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--fg);
  }
  .what {
    font-weight: 600;
  }
  .rule {
    font-size: 11.5px;
    color: var(--fg-muted);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0 7px;
  }
  .url {
    font-size: 12px;
    color: var(--fg-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .muted {
    color: var(--fg-muted);
  }
  .choices {
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 6px 10px;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .choices legend {
    font-size: 12px;
    padding: 0 4px;
  }
  .choices label {
    display: flex;
    align-items: baseline;
    gap: 6px;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .fields {
    display: flex;
    gap: 10px;
  }
  .fields label {
    display: flex;
    flex-direction: column;
    gap: 3px;
    font-size: 12px;
    color: var(--fg-muted);
  }
  .fields .grow {
    flex: 1;
  }
  input:not([type="radio"]),
  select {
    font: inherit;
    font-size: 13.5px;
    color: var(--fg);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 5px 8px;
    outline: none;
  }
  input:focus,
  select:focus {
    border-color: var(--accent);
  }
  .preview {
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 6px 10px;
    background: var(--bg);
  }
  .pline {
    font-size: 12.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  pre {
    margin: 6px 0 2px;
    max-height: 110px;
    overflow: auto;
    font-family: "Cascadia Mono", Consolas, monospace;
    font-size: 11.5px;
    color: var(--fg-muted);
    white-space: pre-wrap;
  }
  .existing {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .note {
    display: flex;
    align-items: center;
    gap: 6px;
    font: inherit;
    color: var(--fg);
    background: transparent;
    border: none;
    border-radius: 5px;
    padding: 3px 6px;
    text-align: left;
    cursor: pointer;
  }
  .note:hover,
  .note:focus-visible {
    background: var(--selected);
  }
  .bad {
    color: var(--c-red);
    font-size: 12.5px;
  }
  .buttons {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }
  .btn {
    font: inherit;
    font-size: 12.5px;
    color: var(--fg);
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 4px 10px;
    cursor: pointer;
    white-space: nowrap;
  }
  .btn:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .btn.primary {
    color: #fff;
    background: var(--accent);
    border-color: var(--accent);
  }
  kbd {
    font: inherit;
    font-size: 11px;
    opacity: 0.75;
    margin-left: 4px;
  }
</style>
