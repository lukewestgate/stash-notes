<script lang="ts">
  // The settings screen (spec section 12). Every change applies immediately
  // and is written to settings.json, which stays the source of truth: hand
  // edits to the file show up here live.
  import { onMount } from "svelte";
  import { api, type Keys, type Settings, type StatusDef } from "../lib/api";
  import { sameKeys } from "../lib/keys";
  import { COLORS } from "../lib/status";
  import { jiraUrl, gerritUrl, timestamp } from "../lib/text";
  import KeyRecorder from "./KeyRecorder.svelte";
  import StatusIcon from "./StatusIcon.svelte";

  interface Props {
    settings: Settings;
    settingsPath: string;
    /** True while the notes folder is being (re)indexed. */
    indexing: boolean;
    sidebarExpanded: boolean;
    onSidebar: (expanded: boolean) => void;
  }
  let { settings, settingsPath, indexing, sidebarExpanded, onSidebar }: Props = $props();

  const CATEGORIES = [
    "General",
    "Window",
    "Editor",
    "Notes",
    "Links",
    "Notes view",
    "Statuses",
    "Shortcuts",
    "Appearance",
  ] as const;
  type Category = (typeof CATEGORIES)[number];

  /** Which settings each category's Reset button restores. */
  const FIELDS: Record<Category, (keyof Settings)[]> = {
    General: ["autostart", "startHidden", "hotkey", "switcherHotkey", "restorePinned", "onSummon", "hideAfterExternalLink"],
    Window: ["showInTaskbar", "alwaysOnTop"],
    Editor: ["liveFormatting", "fontFamily", "fontSize", "lineHeight", "spellCheck", "smartPaste", "timestampFormat"],
    Notes: ["vaultPath", "defaultStatus", "ticketPattern"],
    Links: ["jira", "gerrit"],
    "Notes view": ["notesView"],
    Statuses: ["statuses"],
    Shortcuts: ["keys"],
    Appearance: ["theme", "accent"],
  };

  const ACTIONS: [keyof Keys, string][] = [
    ["quickSwitcher", "Quick switcher"],
    ["dailyNote", "Today's daily note"],
    ["notesView", "Notes view"],
    ["followLink", "Follow link / toggle checkbox"],
    ["back", "Back"],
    ["forward", "Forward"],
    ["openJira", "Open Jira"],
    ["openGerrit", "Open Gerrit change"],
    ["addGerrit", "Add Gerrit link"],
    ["timestamp", "Insert timestamp"],
    ["setStatus", "Set status"],
    ["backlinks", "Backlinks panel"],
    ["archive", "Archive / unarchive"],
    ["rename", "Rename note"],
    ["toggleSidebar", "Toggle sidebar"],
    ["focusSidebar", "Focus sidebar"],
    ["pinNote", "Pin / unpin note (two keys)"],
    ["help", "Shortcut help"],
    ["openSettings", "Settings"],
    ["hide", "Close overlay / hide window"],
    ["quit", "Quit"],
  ];

  const FONTS = ["", "Segoe UI", "Segoe UI Variable Text", "Inter", "Calibri", "Arial", "Cascadia Code", "Consolas"];

  let category = $state<Category>("General");
  // Filled from `settings` by the effect below (and kept in step with hand edits).
  // svelte-ignore state_referenced_locally
  let draft = $state<Settings>(structuredClone($state.snapshot(settings)) as Settings);
  let errors = $state<Record<string, string>>({});
  let defaults = $state<Settings | null>(null);
  let template = $state("");
  let templateSaved = $state("");
  let patternSample = $state("PROJ-123 and OPS-7 were mentioned");
  let patternHits = $state<string[]>([]);
  let saving = 0;

  // Hand edits to settings.json arrive as a new `settings`; show them unless
  // a save of ours is in flight.
  $effect(() => {
    const s = $state.snapshot(settings) as Settings;
    if (saving === 0) draft = structuredClone(s);
  });

  onMount(async () => {
    defaults = await api.defaultSettings().catch(() => null);
    template = templateSaved = await api.getDailyTemplate().catch(() => "");
    testPattern();
  });

  /** Apply a change: write it, or show why not and keep the old value. */
  async function save(field: string, change: (s: Settings) => void) {
    const next = structuredClone($state.snapshot(draft)) as Settings;
    change(next);
    saving++;
    try {
      await api.setSettings(next);
      draft = next;
      delete errors[field];
      errors = { ...errors };
    } catch (e) {
      errors = { ...errors, [field]: String(e instanceof Error ? e.message : e) };
      draft = structuredClone($state.snapshot(settings)) as Settings;
    } finally {
      saving--;
    }
  }

  function set<K extends keyof Settings>(key: K, value: Settings[K]) {
    return save(key, (s) => {
      s[key] = value;
    });
  }

  async function resetCategory() {
    if (!defaults) return;
    const d = defaults;
    await save(category, (s) => {
      for (const f of FIELDS[category]) (s as unknown as Record<string, unknown>)[f] = structuredClone(d[f]);
    });
    if (category === "Notes" && defaults) {
      await api.setDailyTemplate("").catch(() => {});
      template = templateSaved = await api.getDailyTemplate().catch(() => "");
    }
  }

  async function testPattern() {
    try {
      patternHits = await api.testTicketPattern(draft.ticketPattern, patternSample);
      delete errors.ticketPattern;
      errors = { ...errors };
    } catch (e) {
      patternHits = [];
      errors = { ...errors, ticketPattern: `Not a valid pattern: ${e}` };
    }
  }

  async function setPattern(p: string) {
    try {
      await api.testTicketPattern(p, "");
    } catch (e) {
      errors = { ...errors, ticketPattern: `Not a valid pattern: ${e}` };
      return;
    }
    await set("ticketPattern", p);
    testPattern();
  }

  function templateError(t: string, token: string) {
    return t.includes(token) ? null : `Must contain ${token}`;
  }

  async function setTemplate(kind: "jira" | "gerrit", value: string) {
    const token = kind === "jira" ? "{key}" : "{n}";
    const bad = templateError(value, token);
    if (bad) {
      errors = { ...errors, [kind]: bad };
      return;
    }
    await save(kind, (s) => {
      s[kind] = { urlTemplate: value.trim() };
    });
  }

  async function setHotkey(which: "hotkey" | "switcherHotkey", spec: string): Promise<string | null> {
    try {
      await api.testHotkey(spec, which === "hotkey" ? "main" : "switcher");
    } catch (e) {
      return String(e instanceof Error ? e.message : e);
    }
    await set(which, spec);
    return errors[which] ?? null;
  }

  function keyConflict(action: keyof Keys): string | null {
    const spec = draft.keys[action];
    for (const [other, label] of ACTIONS) {
      if (other === action) continue;
      const theirs = draft.keys[other];
      const a = action === "pinNote" ? spec.split(/\s+/)[0] : spec;
      const b = other === "pinNote" ? theirs.split(/\s+/)[0] : theirs;
      if (a && b && sameKeys(a, b)) return `Also used by ${label}`;
    }
    return null;
  }

  async function setKey(action: keyof Keys, spec: string): Promise<string | null> {
    const first = spec.split(/\s+/)[0];
    for (const [other, label] of ACTIONS) {
      if (other === action) continue;
      if (sameKeys(first, draft.keys[other].split(/\s+/)[0])) return `Already used by ${label}`;
    }
    await save(`key:${action}`, (s) => {
      s.keys[action] = spec;
    });
    return errors[`key:${action}`] ?? null;
  }

  async function resetKey(action: keyof Keys) {
    if (defaults) await setKey(action, defaults.keys[action]);
  }

  // ----- statuses -----

  function statusError(list: StatusDef[]): string | null {
    const names = list.map((s) => s.name.trim().toLowerCase());
    if (names.some((n) => !n)) return "Every status needs a name";
    if (names.some((n) => /\s/.test(n))) return "Status names can't contain spaces";
    if (new Set(names).size !== names.length) return "Status names must be different";
    return null;
  }

  async function editStatuses(change: (list: StatusDef[]) => void) {
    const list = structuredClone($state.snapshot(draft.statuses)) as StatusDef[];
    change(list);
    const bad = statusError(list);
    if (bad) {
      errors = { ...errors, statuses: bad };
      return;
    }
    await save("statuses", (s) => {
      s.statuses = list.map((x) => ({ ...x, name: x.name.trim().toLowerCase() }));
    });
  }

  function move(i: number, d: number) {
    editStatuses((l) => {
      const j = i + d;
      if (j < 0 || j >= l.length) return;
      [l[i], l[j]] = [l[j], l[i]];
    });
  }

  async function pickFolder() {
    const dir = await api.pickFolder().catch(() => null);
    if (dir) await set("vaultPath", dir);
  }

  async function saveTemplate() {
    if (template === templateSaved) return;
    try {
      await api.setDailyTemplate(template);
      templateSaved = template;
      delete errors.template;
      errors = { ...errors };
    } catch (e) {
      errors = { ...errors, template: String(e) };
    }
  }

  const sample = new Date(2026, 9, 22, 14, 5);
</script>

{#snippet toggle(label: string, value: boolean, onchange: (v: boolean) => void, hint?: string)}
  <label class="row">
    <span class="text">{label}{#if hint}<span class="hint">{hint}</span>{/if}</span>
    <input type="checkbox" class="switch" checked={value} onchange={(e) => onchange(e.currentTarget.checked)} />
  </label>
{/snippet}

{#snippet error(field: string)}
  {#if errors[field]}<div class="error">{errors[field]}</div>{/if}
{/snippet}

<section class="settings" aria-label="Settings">
  <nav class="cats">
    <h2>Settings</h2>
    {#each CATEGORIES as c}
      <button class:active={category === c} onclick={() => (category = c)}>{c}</button>
    {/each}
  </nav>

  <div class="panel">
    <div class="body">
      <h3>{category}</h3>

      {#if category === "General"}
        {@render toggle("Launch at login", draft.autostart, (v) => set("autostart", v))}
        {@render toggle("Start hidden in the tray", draft.startHidden, (v) => set("startHidden", v), "when launched at login")}
        {@render toggle("Restore pinned notes at login", draft.restorePinned, (v) => set("restorePinned", v))}
        <div class="row">
          <span class="text">On summon, open</span>
          <select value={draft.onSummon} onchange={(e) => set("onSummon", e.currentTarget.value as "last" | "daily")}>
            <option value="last">the last opened note</option>
            <option value="daily">today's daily note</option>
          </select>
        </div>
        <div class="row">
          <span class="text">Show / hide hotkey<span class="hint">global — works from any app</span></span>
          <KeyRecorder value={draft.hotkey} onChange={(k) => setHotkey("hotkey", k)} />
        </div>
        <div class="row">
          <span class="text">Quick switcher hotkey<span class="hint">global — search and open notes from any app</span></span>
          <KeyRecorder value={draft.switcherHotkey} onChange={(k) => setHotkey("switcherHotkey", k)} />
        </div>
        {@render toggle("Hide the window after opening a Jira or Gerrit link", draft.hideAfterExternalLink, (v) => set("hideAfterExternalLink", v))}
        <p class="note">New versions install by running the newer installer; your notes and settings are kept.</p>
      {:else if category === "Window"}
        {@render toggle("Show in taskbar", draft.showInTaskbar, (v) => set("showInTaskbar", v))}
        {@render toggle("Always on top", draft.alwaysOnTop, (v) => set("alwaysOnTop", v))}
        {@render toggle("Sidebar expanded", sidebarExpanded, onSidebar, "remembered between launches")}
      {:else if category === "Editor"}
        {@render toggle("Live formatting", draft.liveFormatting, (v) => set("liveFormatting", v), "hide markdown markers off the caret line")}
        <div class="row">
          <span class="text">Font</span>
          <span class="inline">
            <select value={draft.fontFamily} onchange={(e) => set("fontFamily", e.currentTarget.value)}>
              {#each FONTS as f}<option value={f}>{f || "System UI"}</option>{/each}
              {#if !FONTS.includes(draft.fontFamily)}<option value={draft.fontFamily}>{draft.fontFamily}</option>{/if}
            </select>
            <input
              class="num"
              type="number"
              min="10"
              max="24"
              value={draft.fontSize}
              onchange={(e) => set("fontSize", Math.max(10, Math.min(24, Number(e.currentTarget.value) || 14)))}
            /> px
          </span>
        </div>
        <div class="row">
          <span class="text">Line height</span>
          <input
            class="num"
            type="number"
            step="0.05"
            min="1.1"
            max="2.4"
            value={draft.lineHeight}
            onchange={(e) => set("lineHeight", Math.max(1.1, Math.min(2.4, Number(e.currentTarget.value) || 1.6)))}
          />
        </div>
        {@render toggle("Spell check", draft.spellCheck, (v) => set("spellCheck", v))}
        {@render toggle("Smart paste of Jira and Gerrit links", draft.smartPaste, (v) => set("smartPaste", v), "Ctrl+Shift+V always pastes as typed")}
        <div class="row">
          <span class="text">Timestamp format<span class="hint">yyyy MM dd HH hh mm ss a</span></span>
          <span class="inline">
            <input value={draft.timestampFormat} onchange={(e) => set("timestampFormat", e.currentTarget.value || "yyyy-MM-dd HH:mm")} />
            <span class="preview">{timestamp(sample, draft.timestampFormat)}</span>
          </span>
        </div>
      {:else if category === "Notes"}
        <div class="row">
          <span class="text">Notes folder</span>
          <span class="inline">
            <input class="wide" value={draft.vaultPath} onchange={(e) => set("vaultPath", e.currentTarget.value)} />
            <button class="btn" onclick={pickFolder}>Choose…</button>
          </span>
        </div>
        {#if indexing}<div class="progress">Indexing notes in the background…</div>{/if}
        {@render error("vaultPath")}
        <div class="row">
          <span class="text">Default status for new tickets</span>
          <select value={draft.defaultStatus} onchange={(e) => set("defaultStatus", e.currentTarget.value)}>
            <option value="">no status</option>
            {#each draft.statuses as s}<option value={s.name}>{s.symbol} {s.name}</option>{/each}
          </select>
        </div>
        <div class="row">
          <span class="text">Ticket key pattern<span class="hint">a regular expression</span></span>
          <input class="wide mono" value={draft.ticketPattern} onchange={(e) => setPattern(e.currentTarget.value)} />
        </div>
        <div class="row sub">
          <span class="text">Try it</span>
          <span class="inline">
            <input class="wide" bind:value={patternSample} oninput={testPattern} />
            <span class="preview">{patternHits.length ? patternHits.join(", ") : "no keys"}</span>
          </span>
        </div>
        {@render error("ticketPattern")}
        <div class="block">
          <span class="text">Daily note template<span class="hint">{"{{date}}"} and {"{{weekday}}"} are filled in; saved to templates/daily.md</span></span>
          <textarea rows="8" bind:value={template} onblur={saveTemplate} spellcheck="false"></textarea>
          {@render error("template")}
        </div>
      {:else if category === "Links"}
        <div class="block">
          <span class="text">Jira URL template</span>
          <input class="wide mono" value={draft.jira.urlTemplate} onchange={(e) => setTemplate("jira", e.currentTarget.value)} />
          <span class="preview">PROJ-123 → {jiraUrl(draft.jira.urlTemplate, "PROJ-123")}</span>
          {@render error("jira")}
        </div>
        <div class="block">
          <span class="text">Gerrit URL template<span class="hint">for bare change numbers</span></span>
          <input class="wide mono" value={draft.gerrit.urlTemplate} onchange={(e) => setTemplate("gerrit", e.currentTarget.value)} />
          <span class="preview">12345 → {gerritUrl(draft.gerrit.urlTemplate, "12345")}</span>
          {@render error("gerrit")}
        </div>
      {:else if category === "Notes view"}
        {@render toggle("Show daily notes", draft.notesView.showDaily, (v) => save("notesView", (s) => (s.notesView.showDaily = v)))}
        {@render toggle("Show closed and archived notes", draft.notesView.showClosed, (v) => save("notesView", (s) => (s.notesView.showClosed = v)))}
        <div class="row">
          <span class="text">Default sort</span>
          <select
            value={draft.notesView.sort}
            onchange={(e) => save("notesView", (s) => (s.notesView.sort = e.currentTarget.value as Settings["notesView"]["sort"]))}
          >
            <option value="edited">Last edited</option>
            <option value="opened">Last opened</option>
            <option value="status">Status, then last edited</option>
            <option value="title">Title or key</option>
          </select>
        </div>
        {@render toggle("Group by status", draft.notesView.groupByStatus, (v) => save("notesView", (s) => (s.notesView.groupByStatus = v)))}
      {:else if category === "Statuses"}
        <table class="statuses">
          <thead><tr><th></th><th>Symbol</th><th>Name</th><th>Color</th><th>Active</th><th></th></tr></thead>
          <tbody>
            {#each draft.statuses as st, i (i)}
              <tr>
                <td><StatusIcon status={st.name} statuses={draft.statuses} /></td>
                <td><input class="sym" maxlength="2" value={st.symbol} onchange={(e) => editStatuses((l) => (l[i].symbol = e.currentTarget.value || "•"))} /></td>
                <td><input value={st.name} onchange={(e) => editStatuses((l) => (l[i].name = e.currentTarget.value))} /></td>
                <td class="swatches">
                  {#each COLORS as c}
                    <button
                      class="swatch"
                      class:on={st.color === c}
                      style="background: var(--c-{c})"
                      title={c}
                      aria-label={c}
                      onclick={() => editStatuses((l) => (l[i].color = c))}
                    ></button>
                  {/each}
                </td>
                <td><input type="checkbox" checked={st.active} onchange={(e) => editStatuses((l) => (l[i].active = e.currentTarget.checked))} /></td>
                <td class="ops">
                  <button class="mini" disabled={i === 0} onclick={() => move(i, -1)} aria-label="Move up">↑</button>
                  <button class="mini" disabled={i === draft.statuses.length - 1} onclick={() => move(i, 1)} aria-label="Move down">↓</button>
                  <button class="mini" disabled={st.name === "closed"} title={st.name === "closed" ? "closed archives notes and can't be removed" : "Remove"} onclick={() => editStatuses((l) => l.splice(i, 1))} aria-label="Remove">×</button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
        <button class="btn" onclick={() => editStatuses((l) => l.push({ name: `status${l.length + 1}`, symbol: "◆", color: "teal", active: true }))}>Add status</button>
        {@render error("statuses")}
        <p class="note">Notes keep whatever status they have; renaming a status here doesn't rewrite notes. Setting <b>closed</b> moves a note into the archive.</p>
      {:else if category === "Shortcuts"}
        <table class="keys">
          <tbody>
            {#each ACTIONS as [action, label]}
              <tr>
                <td>{label}</td>
                <td>
                  <KeyRecorder
                    value={draft.keys[action]}
                    chord={action === "pinNote"}
                    conflict={keyConflict(action)}
                    onChange={(k) => setKey(action, k)}
                  />
                </td>
                <td><button class="mini" disabled={!defaults || draft.keys[action] === defaults.keys[action]} onclick={() => resetKey(action)}>Reset</button></td>
              </tr>
            {/each}
          </tbody>
        </table>
        <p class="note">The global hotkeys are under General. Ctrl+B / Ctrl+I (bold, italic) and Ctrl+Shift+V (plain paste) are fixed.</p>
      {:else if category === "Appearance"}
        <div class="row">
          <span class="text">Theme</span>
          <select value={draft.theme} onchange={(e) => set("theme", e.currentTarget.value as Settings["theme"])}>
            <option value="system">Follow Windows</option>
            <option value="light">Light</option>
            <option value="dark">Dark</option>
          </select>
        </div>
        <div class="row">
          <span class="text">Accent color</span>
          <span class="swatches">
            {#each COLORS as c}
              <button class="swatch big" class:on={draft.accent === c} style="background: var(--c-{c})" title={c} aria-label={c} onclick={() => set("accent", c)}></button>
            {/each}
          </span>
        </div>
      {/if}
      {@render error(category)}
    </div>

    <footer class="foot">
      <button class="btn" onclick={() => api.openSettingsFile()} title={settingsPath}>Open settings.json</button>
      <button class="btn" onclick={() => api.openVaultFolder()}>Open notes folder</button>
      <span class="grow"></span>
      <button class="btn" onclick={resetCategory} disabled={!defaults}>Reset {category} to defaults</button>
    </footer>
  </div>
</section>

<style>
  .settings {
    flex: 1;
    min-height: 0;
    display: flex;
    background: var(--bg);
  }
  .cats {
    width: 180px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding: 12px 8px;
    border-right: 1px solid var(--border);
    background: var(--bg-raised);
    overflow-y: auto;
  }
  .cats h2 {
    font-size: 15px;
    margin: 2px 8px 10px;
  }
  .cats button {
    text-align: left;
    font: inherit;
    color: var(--fg);
    background: transparent;
    border: none;
    border-radius: 6px;
    padding: 6px 10px;
    cursor: pointer;
  }
  .cats button:hover {
    background: var(--hover);
  }
  .cats button.active {
    background: var(--selected);
    font-weight: 600;
  }
  .panel {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .body {
    flex: 1;
    overflow-y: auto;
    padding: 16px 24px 24px;
    max-width: 760px;
  }
  h3 {
    margin: 0 0 12px;
    font-size: 16px;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    min-height: 40px;
    padding: 4px 0;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 60%, transparent);
  }
  .row.sub {
    border-bottom: none;
  }
  .block {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px 0;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 60%, transparent);
  }
  .text {
    display: flex;
    flex-direction: column;
  }
  .hint {
    font-size: 12px;
    color: var(--fg-muted);
  }
  .inline {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  input:not([type="checkbox"]),
  select,
  textarea {
    font: inherit;
    font-size: 13px;
    color: var(--fg);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 4px 8px;
    outline: none;
  }
  input:focus,
  select:focus,
  textarea:focus {
    border-color: var(--accent);
  }
  .wide {
    width: min(380px, 50vw);
  }
  .block .wide {
    width: 100%;
  }
  .num {
    width: 70px;
  }
  .mono,
  textarea {
    font-family: "Cascadia Mono", Consolas, monospace;
  }
  textarea {
    width: 100%;
    resize: vertical;
  }
  .preview {
    font-size: 12px;
    color: var(--fg-muted);
    word-break: break-all;
  }
  .switch {
    width: 34px;
    height: 18px;
    accent-color: var(--accent);
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
  }
  .btn:hover:not(:disabled) {
    background: var(--hover);
  }
  .btn:disabled {
    opacity: 0.5;
  }
  .mini {
    font: inherit;
    font-size: 12px;
    color: var(--fg-muted);
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0 6px;
    cursor: pointer;
  }
  .mini:disabled {
    opacity: 0.35;
    cursor: default;
  }
  .error {
    margin: 6px 0;
    font-size: 12.5px;
    color: var(--c-red);
  }
  .progress {
    font-size: 12.5px;
    color: var(--accent);
    padding: 4px 0;
  }
  .note {
    font-size: 12.5px;
    color: var(--fg-muted);
  }
  table {
    border-collapse: collapse;
    width: 100%;
  }
  th {
    text-align: left;
    font-size: 11.5px;
    font-weight: 600;
    color: var(--fg-muted);
    padding: 4px 6px;
  }
  td {
    padding: 5px 6px;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 60%, transparent);
  }
  .statuses input:not([type="checkbox"]) {
    width: 110px;
  }
  .statuses input.sym {
    width: 40px;
    text-align: center;
  }
  .ops {
    white-space: nowrap;
  }
  .swatches {
    display: inline-flex;
    gap: 4px;
    flex-wrap: wrap;
  }
  .swatch {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 2px solid transparent;
    cursor: pointer;
    padding: 0;
  }
  .swatch.big {
    width: 22px;
    height: 22px;
  }
  .swatch.on {
    border-color: var(--fg);
  }
  .keys td:first-child {
    width: 45%;
  }
  .foot {
    display: flex;
    gap: 8px;
    padding: 10px 24px;
    border-top: 1px solid var(--border);
  }
  .grow {
    flex: 1;
  }
</style>
