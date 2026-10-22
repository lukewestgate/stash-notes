<script lang="ts">
  // The settings screen (spec section 12). Every change applies immediately
  // and is written to settings.json, which stays the source of truth: hand
  // edits to the file show up here live.
  import { onMount } from "svelte";
  import { api, type Keys, type Settings, type StatusDef, type TagCount, type VaultDef } from "../lib/api";
  import { toaster } from "../lib/toasts.svelte";
  import TagChip from "./TagChip.svelte";
  import { sameKeys } from "../lib/keys";
  import { COLORS } from "../lib/status";
  import { timestamp } from "../lib/text";
  import { LINK_PRESETS, TICKET_PRESETS, linkUrl, ticketUrl } from "../lib/links";
  import { LINK_ICONS } from "../lib/icons";
  import KeyRecorder from "./KeyRecorder.svelte";
  import { STATUS_ICONS } from "../lib/icons";
  import Icon from "./Icon.svelte";

  interface Props {
    settings: Settings;
    settingsPath: string;
    /** True while the notes folder is being (re)indexed. */
    indexing: boolean;
    sidebarExpanded: boolean;
    onSidebar: (expanded: boolean) => void;
    currentVault: string;
    /** Each vault's folder, by id. */
    vaultPaths: Record<string, string>;
  }
  let { settings, settingsPath, indexing, sidebarExpanded, onSidebar, currentVault, vaultPaths }: Props = $props();

  const CATEGORIES = [
    "General",
    "Vaults",
    "Window",
    "Editor",
    "Notes",
    "Notes view",
    "Statuses",
    "Tags",
    "Shortcuts",
    "Appearance",
  ] as const;
  type Category = (typeof CATEGORIES)[number];

  /** Which settings each category's Reset button restores. */
  const FIELDS: Record<Category, (keyof Settings)[]> = {
    General: ["autostart", "startHidden", "hotkey", "switcherHotkey", "restorePinned", "onSummon", "hideAfterExternalLink"],
    Window: ["showInTaskbar", "alwaysOnTop"],
    Editor: ["liveFormatting", "readableWidth", "fontFamily", "fontSize", "lineHeight", "spellCheck", "smartPaste", "timestampFormat"],
    Vaults: [],
    Notes: ["defaultStatus"],
    "Notes view": ["notesView"],
    Statuses: ["statuses"],
    Tags: ["tagColors"],
    Shortcuts: ["keys"],
    Appearance: ["theme", "accent", "animations"],
  };

  const ACTIONS: [keyof Keys, string][] = [
    ["quickSwitcher", "Quick switcher"],
    ["dailyNote", "Today's daily note"],
    ["notesView", "Notes view"],
    ["followLink", "Follow link / toggle checkbox"],
    ["back", "Back"],
    ["forward", "Forward"],
    ["openTicket", "Open the note's ticket"],
    ["openLink", "Open an attached link"],
    ["addLink", "Attach a link"],
    ["timestamp", "Insert timestamp"],
    ["setStatus", "Set status"],
    ["setTags", "Add or remove tags"],
    ["backlinks", "Backlinks panel"],
    ["archive", "Archive / unarchive"],
    ["rename", "Rename note"],
    ["switchVault", "Switch vault"],
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
  /** The status row whose icon picker is open. */
  let iconFor = $state<number | null>(null);
  // Filled from `settings` by the effect below (and kept in step with hand edits).
  // svelte-ignore state_referenced_locally
  let draft = $state<Settings>(structuredClone($state.snapshot(settings)) as Settings);
  let errors = $state<Record<string, string>>({});
  let defaults = $state<Settings | null>(null);
  let template = $state("");
  let templateSaved = $state("");
  /** The ticket system / link type whose icon picker is open: `vault:kind:index`. */
  let linkIconFor = $state<string | null>(null);
  let saving = 0;
  let tags = $state<TagCount[]>([]);
  /** The tag whose rename field is open. */
  let renaming = $state<string | null>(null);

  async function loadTags() {
    tags = await api.listTags().catch(() => []);
  }

  $effect(() => {
    if (category === "Tags") loadTags();
  });

  function setTagColor(tag: string, color: string | null) {
    save("tagColors", (s) => {
      if (color) s.tagColors[tag] = color;
      else delete s.tagColors[tag];
    });
  }

  /** Rename `from` on every note (or remove it when `to` is empty), with Undo. */
  async function renameTag(from: string, to: string) {
    renaming = null;
    const target = to.trim().replace(/^#/, "").toLowerCase();
    if (target === from) return;
    try {
      const r = await api.renameTag(from, target);
      const color = draft.tagColors[from];
      if (color) {
        await save("tagColors", (s) => {
          delete s.tagColors[from];
          if (target) s.tagColors[target] = color;
        });
      }
      const what = target ? `Renamed #${from} to #${target}` : `Removed #${from}`;
      toaster.show(`${what} on ${r.notes} ${r.notes === 1 ? "note" : "notes"}`, "done", {
        label: "Undo",
        run: async () => {
          await api.undoTagRename(r.undoId).catch((e) => toaster.show(String(e), "error"));
          if (color) await save("tagColors", (s) => {
            if (target) delete s.tagColors[target];
            s.tagColors[from] = color;
          });
          loadTags();
        },
      });
      loadTags();
    } catch (e) {
      toaster.show(String(e instanceof Error ? e.message : e), "error");
    }
  }

  // Hand edits to settings.json arrive as a new `settings`; show them unless
  // a save of ours is in flight.
  $effect(() => {
    const s = $state.snapshot(settings) as Settings;
    if (saving === 0) draft = structuredClone(s);
  });

  // svelte-ignore state_referenced_locally
  let templateVault = $state(currentVault);
  let defaultFolder = $state("");
  /** `copyFrom`: a vault whose ticket systems and link types the new one starts with ("" for none). */
  let newVault = $state({ name: "", path: "", copyFrom: "" });

  $effect(() => {
    const v = templateVault;
    api.getDailyTemplate(v).then((t) => (template = templateSaved = t), () => {});
  });

  onMount(async () => {
    defaults = await api.defaultSettings().catch(() => null);
    defaultFolder = await api.defaultVaultFolder().catch(() => "");
  });

  /** A vault id from a name, unique among the vaults (same rule as Rust's `vault_id`). */
  function vaultId(name: string): string {
    const base = name.toLowerCase().normalize("NFKD").replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "") || "vault";
    let id = base;
    for (let n = 2; draft.vaults.some((v) => v.id === id); n++) id = `${base}-${n}`;
    return id;
  }

  function editVaults(change: (list: VaultDef[]) => void) {
    return save("vaults", (s) => change(s.vaults));
  }

  async function addVault() {
    const name = newVault.name.trim();
    if (!name) {
      errors = { ...errors, vaults: "Give the new vault a name" };
      return;
    }
    if (!newVault.path.trim()) {
      errors = { ...errors, vaults: "Choose a folder for the new vault" };
      return;
    }
    const used = new Set(draft.vaults.map((v) => v.color));
    const color = COLORS.find((c) => !used.has(c)) ?? "teal";
    const from = draft.vaults.find((v) => v.id === newVault.copyFrom);
    const tickets = from ? structuredClone($state.snapshot(from.tickets)) : [];
    const links = from ? structuredClone($state.snapshot(from.links)) : [];
    await editVaults((l) => l.push({ id: vaultId(name), name, path: newVault.path.trim(), color, tickets, links }));
    if (!errors.vaults) newVault = { name: "", path: "", copyFrom: "" };
  }

  async function removeVault(i: number) {
    const before = structuredClone($state.snapshot(draft.vaults)) as VaultDef[];
    const gone = before[i];
    await editVaults((l) => l.splice(i, 1));
    toaster.show(`Removed vault ${gone.name} — its notes are still in the folder`, "done", {
      label: "Undo",
      run: () => save("vaults", (s) => (s.vaults = before)),
    });
  }

  async function chooseVaultFolder(i: number | null) {
    const dir = await api.pickFolder().catch(() => null);
    if (!dir) return;
    if (i === null) {
      newVault.path = dir;
      if (!newVault.name.trim()) newVault.name = dir.split(/[\\/]/).filter(Boolean).pop() ?? "";
    } else await editVaults((l) => (l[i].path = dir));
  }

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
      await api.setDailyTemplate(templateVault, "").catch(() => {});
      template = templateSaved = await api.getDailyTemplate(templateVault).catch(() => "");
    }
  }

  /** A key the ticket system makes, for its URL preview. */
  function sampleKey(pattern: string): string {
    const prefix = /^([A-Za-z][A-Za-z0-9_]*)-\\d\+$/.exec(pattern)?.[1];
    return prefix ? `${prefix}-42` : "PROJ-123";
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

  async function saveTemplate() {
    if (template === templateSaved) return;
    try {
      await api.setDailyTemplate(templateVault, template);
      templateSaved = template;
      delete errors.template;
      errors = { ...errors };
    } catch (e) {
      errors = { ...errors, template: String(e) };
    }
  }

  const sample = new Date(2026, 9, 22, 14, 5);
</script>

{#snippet linkIcon(key: string, icon: string, pick: (icon: string) => void)}
  <span class="licon">
    <button class="icon-btn" title="Choose an icon" aria-label="Icon" onclick={() => (linkIconFor = linkIconFor === key ? null : key)}><Icon name={icon} size={15} /></button>
    {#if linkIconFor === key}
      <div class="icon-grid" role="listbox" tabindex="-1">
        {#each LINK_ICONS as name}
          <button role="option" aria-selected={icon === name} class:on={icon === name} title={name} onclick={() => { pick(name); linkIconFor = null; }}><Icon {name} size={16} /></button>
        {/each}
      </div>
    {/if}
  </span>
{/snippet}

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
        {@render toggle("Hide the window after opening a link", draft.hideAfterExternalLink, (v) => set("hideAfterExternalLink", v), "a ticket, attached link or URL")}
        <p class="note">New versions install by running the newer installer; your notes and settings are kept.</p>
      {:else if category === "Window"}
        {@render toggle("Show in taskbar", draft.showInTaskbar, (v) => set("showInTaskbar", v))}
        {@render toggle("Always on top", draft.alwaysOnTop, (v) => set("alwaysOnTop", v))}
        {@render toggle("Sidebar expanded", sidebarExpanded, onSidebar, "remembered between launches")}
      {:else if category === "Editor"}
        {@render toggle("Live formatting", draft.liveFormatting, (v) => set("liveFormatting", v), "hide markdown markers off the caret line")}
        {@render toggle("Comfortable line width", draft.readableWidth, (v) => set("readableWidth", v), "keep text in a centred column on wide windows")}
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
        {@render toggle("Smart paste of ticket and link URLs", draft.smartPaste, (v) => set("smartPaste", v), "uses each vault's links; Ctrl+Shift+V always pastes as typed")}
        <div class="row">
          <span class="text">Timestamp format<span class="hint">yyyy MM dd HH hh mm ss a</span></span>
          <span class="inline">
            <input value={draft.timestampFormat} onchange={(e) => set("timestampFormat", e.currentTarget.value || "yyyy-MM-dd HH:mm")} />
            <span class="preview">{timestamp(sample, draft.timestampFormat)}</span>
          </span>
        </div>
      {:else if category === "Vaults"}
        <p class="note">
          A vault is a folder of notes. The sidebar, Notes view and Today show one vault at a time; search (<b>{draft.keys.quickSwitcher}</b>
          and the pop-up) covers all of them. Switch with <b>{draft.keys.switchVault}</b>, the sidebar or the tray. Removing a vault
          here never deletes its files.
        </p>
        {#each draft.vaults as v, i (v.id)}
          <div class="vcard" style:--vc="var(--c-{v.color})">
            <div class="vhead">
              <span class="badge">{v.name.slice(0, 1).toUpperCase()}</span>
              <input class="vname" value={v.name} onchange={(e) => editVaults((l) => (l[i].name = e.currentTarget.value.trim() || l[i].name))} aria-label="Vault name" />
              <span class="vid" title="Used in links ([[{v.id}:Note]]) and searches (in:{v.id}); never changes">in:{v.id}{#if v.id === currentVault}&nbsp;· current{/if}</span>
              <span class="grow"></span>
              <span class="ops">
                <button class="mini" onclick={() => api.openVaultFolder(v.id)} title="Open the folder" aria-label="Open folder"><Icon name="external-link" size={14} /></button>
                <button class="mini" disabled={i === 0} onclick={() => editVaults((l) => ([l[i - 1], l[i]] = [l[i], l[i - 1]]))} aria-label="Move up"><Icon name="arrow-up" size={14} /></button>
                <button class="mini" disabled={i === draft.vaults.length - 1} onclick={() => editVaults((l) => ([l[i], l[i + 1]] = [l[i + 1], l[i]]))} aria-label="Move down"><Icon name="arrow-down" size={14} /></button>
                <button class="mini" disabled={draft.vaults.length === 1} title={draft.vaults.length === 1 ? "Keep at least one vault" : "Remove (the files stay)"} onclick={() => removeVault(i)} aria-label="Remove"><Icon name="trash" size={14} /></button>
              </span>
            </div>
            <div class="row">
              <span class="text">Folder</span>
              <span class="inline">
                <input
                  class="wide"
                  value={v.path}
                  placeholder={defaultFolder}
                  title={vaultPaths[v.id] ?? ""}
                  onchange={(e) => editVaults((l) => (l[i].path = e.currentTarget.value.trim()))}
                  aria-label="Folder"
                />
                <button class="btn" onclick={() => chooseVaultFolder(i)}>Choose…</button>
              </span>
            </div>
            <div class="row">
              <span class="text">Color</span>
              <span class="swatches">
                {#each COLORS as c}
                  <button class="swatch" class:on={v.color === c} style="background: var(--c-{c})" title={c} aria-label={c} onclick={() => editVaults((l) => (l[i].color = c))}></button>
                {/each}
              </span>
            </div>
            <div class="links">
              <div class="lhead">
                <span class="text">Tickets<span class="hint">a note per item, named by its key; pasting its URL gives the key</span></span>
                <select
                  value=""
                  onchange={(e) => {
                    const p = TICKET_PRESETS[Number(e.currentTarget.value)];
                    e.currentTarget.value = "";
                    if (p) editVaults((l) => l[i].tickets.push(structuredClone(p.system)));
                  }}
                  aria-label="Add a ticket system"
                >
                  <option value="" disabled>Add…</option>
                  {#each TICKET_PRESETS as p, k}<option value={k}>{p.label}</option>{/each}
                </select>
              </div>
              {#each v.tickets as t, j}
                {@render linkIcon(`${v.id}:t:${j}`, t.icon, (icon) => editVaults((l) => (l[i].tickets[j].icon = icon)))}
                <div class="lrow">
                  <input class="lname" value={t.name} onchange={(e) => editVaults((l) => (l[i].tickets[j].name = e.currentTarget.value.trim() || t.name))} aria-label="Name" />
                  <input class="mono lpat" value={t.pattern} title="Key pattern (a regular expression)" onchange={(e) => editVaults((l) => (l[i].tickets[j].pattern = e.currentTarget.value.trim()))} aria-label="Key pattern" />
                  <input class="mono wide" value={t.url} title={"URL: {key} is the key, {n} its number"} onchange={(e) => editVaults((l) => (l[i].tickets[j].url = e.currentTarget.value.trim()))} aria-label="URL" />
                  <button class="mini" onclick={() => editVaults((l) => l[i].tickets.splice(j, 1))} title="Remove" aria-label="Remove"><Icon name="trash" size={14} /></button>
                </div>
                <div class="lpreview">{sampleKey(t.pattern)} → {ticketUrl(t, sampleKey(t.pattern))}</div>
              {:else}
                <div class="lnone">None — this vault's notes are plain notes.</div>
              {/each}
              <div class="lhead">
                <span class="text">Attached links<span class="hint">a note collects them in front matter; pasting one adds it</span></span>
                <select
                  value=""
                  onchange={(e) => {
                    const p = LINK_PRESETS[Number(e.currentTarget.value)];
                    e.currentTarget.value = "";
                    if (p) editVaults((l) => l[i].links.push(structuredClone(p.type)));
                  }}
                  aria-label="Add a link type"
                >
                  <option value="" disabled>Add…</option>
                  {#each LINK_PRESETS as p, k}<option value={k}>{p.label}</option>{/each}
                </select>
              </div>
              {#each v.links as t, j}
                {@render linkIcon(`${v.id}:l:${j}`, t.icon, (icon) => editVaults((l) => (l[i].links[j].icon = icon)))}
                <div class="lrow">
                  <input class="lname" value={t.name} onchange={(e) => editVaults((l) => (l[i].links[j].name = e.currentTarget.value.trim() || t.name))} aria-label="Name" />
                  <input class="mono lpat" value={t.field} title="Front matter key" onchange={(e) => editVaults((l) => (l[i].links[j].field = e.currentTarget.value.trim().toLowerCase()))} aria-label="Front matter key" />
                  <input class="mono wide" value={t.url} title={"URL for a bare entry: {n} is the entry; pasted URLs on this site are recognised"} onchange={(e) => editVaults((l) => (l[i].links[j].url = e.currentTarget.value.trim()))} aria-label="URL" />
                  <button class="mini" onclick={() => editVaults((l) => l[i].links.splice(j, 1))} title="Remove" aria-label="Remove"><Icon name="trash" size={14} /></button>
                </div>
                <div class="lpreview">{t.field}: 12345 → {linkUrl(t, "12345")}</div>
              {:else}
                <div class="lnone">None.</div>
              {/each}
            </div>
          </div>
        {/each}
        {#if indexing}<div class="progress">Indexing notes in the background…</div>{/if}
        <div class="add-vault">
          <span class="text">Add a vault</span>
          <span class="inline">
            <input placeholder="Name, e.g. Work" bind:value={newVault.name} aria-label="New vault name" />
            <input class="wide" placeholder="Folder" bind:value={newVault.path} aria-label="New vault folder" />
            <button class="btn" onclick={() => chooseVaultFolder(null)}>Choose…</button>
            <select bind:value={newVault.copyFrom} aria-label="Links for the new vault">
              <option value="">Plain notes</option>
              {#each draft.vaults.filter((x) => x.tickets.length || x.links.length) as x}<option value={x.id}>Links like {x.name}</option>{/each}
            </select>
            <button class="btn primary" onclick={addVault}>Add vault</button>
          </span>
        </div>
        {@render error("vaults")}
      {:else if category === "Notes"}
        <div class="row">
          <span class="text">Default status for new tickets</span>
          <select value={draft.defaultStatus} onchange={(e) => set("defaultStatus", e.currentTarget.value)}>
            <option value="">no status</option>
            {#each draft.statuses as s}<option value={s.name}>{s.name}</option>{/each}
          </select>
        </div>
        <div class="block">
          <span class="text">Daily note template<span class="hint">{"{{date}}"} and {"{{weekday}}"} are filled in; saved to templates/daily.md in the vault</span></span>
          {#if draft.vaults.length > 1}
            <select bind:value={templateVault} aria-label="Vault">
              {#each draft.vaults as v}<option value={v.id}>{v.name}</option>{/each}
            </select>
          {/if}
          <textarea rows="8" bind:value={template} onblur={saveTemplate} spellcheck="false"></textarea>
          {@render error("template")}
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
        <div class="row">
          <span class="text">Group by</span>
          <select
            value={draft.notesView.groupBy}
            onchange={(e) => save("notesView", (s) => (s.notesView.groupBy = e.currentTarget.value as Settings["notesView"]["groupBy"]))}
          >
            <option value="none">Nothing</option>
            <option value="status">Status</option>
            <option value="tag">Tag</option>
          </select>
        </div>
      {:else if category === "Statuses"}
        <table class="statuses">
          <thead><tr><th>Icon</th><th>Name</th><th>Color</th><th>Active</th><th></th></tr></thead>
          <tbody>
            {#each draft.statuses as st, i (i)}
              <tr>
                <td class="icon-cell" onfocusout={(e) => iconFor === i && !e.currentTarget.contains(e.relatedTarget as Node) && (iconFor = null)}>
                  <button
                    class="icon-btn"
                    style="color: var(--c-{st.color})"
                    title="Choose an icon"
                    aria-label="Icon for {st.name}"
                    aria-expanded={iconFor === i}
                    onclick={() => (iconFor = iconFor === i ? null : i)}><Icon name={st.icon} size={16} /></button
                  >
                  {#if iconFor === i}
                    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
                    <div
                      class="icon-grid"
                      role="listbox"
                      tabindex="-1"
                      style="color: var(--c-{st.color})"
                      onkeydown={(e) => e.key === "Escape" && (e.stopPropagation(), (iconFor = null))}
                    >
                      {#each STATUS_ICONS as name}
                        <button
                          role="option"
                          aria-selected={st.icon === name}
                          class:on={st.icon === name}
                          title={name}
                          {@attach (el) => { if (st.icon === name) el.focus(); }}
                          onclick={() => {
                            editStatuses((l) => (l[i].icon = name));
                            iconFor = null;
                          }}><Icon {name} size={16} /></button
                        >
                      {/each}
                    </div>
                  {/if}
                </td>
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
                  <button class="mini" disabled={i === 0} onclick={() => move(i, -1)} aria-label="Move up"><Icon name="arrow-up" size={14} /></button>
                  <button class="mini" disabled={i === draft.statuses.length - 1} onclick={() => move(i, 1)} aria-label="Move down"><Icon name="arrow-down" size={14} /></button>
                  <button class="mini" disabled={st.name === "closed"} title={st.name === "closed" ? "closed archives notes and can't be removed" : "Remove"} onclick={() => editStatuses((l) => l.splice(i, 1))} aria-label="Remove"><Icon name="trash" size={14} /></button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
        <button class="btn" onclick={() => editStatuses((l) => l.push({ name: `status${l.length + 1}`, icon: "diamond", color: "teal", active: true }))}>Add status</button>
        {@render error("statuses")}
        <p class="note">Notes keep whatever status they have; renaming a status here doesn't rewrite notes. Setting <b>closed</b> moves a note into the archive.</p>
      {:else if category === "Tags"}
        {#if tags.length}
          <table class="tags">
            <thead><tr><th>Tag</th><th>Notes</th><th>Color</th><th></th></tr></thead>
            <tbody>
              {#each tags as t (t.name)}
                <tr>
                  <td>
                    {#if renaming === t.name}
                      <input
                        value={t.name}
                        {@attach (el) => { el.focus(); el.select(); }}
                        onkeydown={(e) => {
                          if (e.key === "Enter") renameTag(t.name, e.currentTarget.value);
                          else if (e.key === "Escape") { e.stopPropagation(); renaming = null; }
                        }}
                        onblur={() => (renaming = null)}
                        aria-label="New name for #{t.name}"
                      />
                    {:else}
                      <TagChip tag={t.name} colors={draft.tagColors} />
                    {/if}
                  </td>
                  <td class="num-cell">{t.count}</td>
                  <td class="swatches">
                    <button class="swatch none" class:on={!draft.tagColors[t.name]} title="No color" aria-label="No color" onclick={() => setTagColor(t.name, null)}></button>
                    {#each COLORS as c}
                      <button class="swatch" class:on={draft.tagColors[t.name] === c} style="background: var(--c-{c})" title={c} aria-label={c} onclick={() => setTagColor(t.name, c)}></button>
                    {/each}
                  </td>
                  <td class="ops">
                    <button class="mini wide-mini" onclick={() => (renaming = t.name)} title="Rename on every note">Rename</button>
                    <button class="mini" onclick={() => renameTag(t.name, "")} title="Remove from every note" aria-label="Remove #{t.name}"><Icon name="trash" size={14} /></button>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        {:else}
          <p class="note">No tags yet. Press <b>{draft.keys.setTags}</b> on a note to add one.</p>
        {/if}
        <p class="note">Tags live in each note's front matter (<code>tags: [bug, research]</code>). Renaming or removing a tag rewrites those notes, and nested tags (<code>bug/ui</code>) follow their parent. A nested tag without its own color uses its parent's.</p>
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
        <div class="row">
          <span class="text">Animations<span class="hint">pop-ups, toasts and the sidebar slide and fade in</span></span>
          <select value={draft.animations} onchange={(e) => set("animations", e.currentTarget.value as Settings["animations"])}>
            <option value="system">Follow Windows</option>
            <option value="on">On</option>
            <option value="off">Off</option>
          </select>
        </div>
      {/if}
      {@render error(category)}
    </div>

    <footer class="foot">
      <button class="btn" onclick={() => api.openSettingsFile()} title={settingsPath}>Open settings.json</button>
      <button class="btn" onclick={() => api.openVaultFolder(currentVault)}>Open notes folder</button>
      <span class="grow"></span>
      {#if FIELDS[category].length}
        <button class="btn" onclick={resetCategory} disabled={!defaults}>Reset {category} to defaults</button>
      {/if}
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
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 20%, transparent);
  }
  .wide {
    width: min(380px, 50vw);
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
    min-height: 22px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    vertical-align: middle;
  }
  .ops .mini {
    width: 24px;
    padding: 0;
  }
  .ops .mini.wide-mini {
    width: auto;
    padding: 0 8px;
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
  .swatch.none {
    background: transparent;
    border: 1px dashed var(--fg-muted);
  }
  .num-cell {
    color: var(--fg-muted);
    font-variant-numeric: tabular-nums;
  }
  .tags input {
    width: 160px;
  }
  .links {
    display: grid;
    grid-template-columns: auto 1fr;
    align-items: center;
    column-gap: 8px;
    padding: 4px 0 8px;
  }
  .lhead {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 10px 0 4px;
  }
  .lrow {
    display: flex;
    gap: 6px;
    align-items: center;
    min-width: 0;
  }
  .lrow .lname {
    width: 110px;
  }
  .lrow .lpat {
    width: 150px;
  }
  .lrow .wide {
    flex: 1;
    min-width: 0;
    width: auto;
  }
  .lpreview,
  .lnone {
    grid-column: 2;
    font-size: 11.5px;
    color: var(--fg-muted);
    padding: 2px 0 6px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .lnone {
    grid-column: 1 / -1;
  }
  .licon {
    position: relative;
  }
  .vcard {
    border: 1px solid var(--border);
    border-left: 3px solid var(--vc);
    border-radius: 8px;
    padding: 8px 12px 4px;
    margin: 10px 0;
  }
  .vhead {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .badge {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 700;
    color: #fff;
    background: var(--vc);
  }
  .vname {
    font-weight: 600;
    width: 180px;
  }
  .vid {
    font-size: 11px;
    color: var(--fg-muted);
  }
  .grow {
    flex: 1;
  }
  .add-vault {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 14px 0 4px;
  }
  .btn {
    white-space: nowrap;
  }
  .btn.primary {
    color: #fff;
    background: var(--accent);
    border-color: var(--accent);
  }
  .icon-cell {
    position: relative;
  }
  .icon-btn {
    display: grid;
    place-items: center;
    width: 30px;
    height: 26px;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 6px;
    cursor: pointer;
  }
  .icon-grid {
    position: absolute;
    z-index: 5;
    top: calc(100% - 2px);
    left: 4px;
    display: grid;
    grid-template-columns: repeat(6, 30px);
    gap: 2px;
    padding: 6px;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: var(--shadow);
  }
  .icon-grid button {
    display: grid;
    place-items: center;
    height: 30px;
    color: inherit;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 6px;
    cursor: pointer;
  }
  .icon-grid button:hover {
    background: var(--hover);
  }
  .icon-grid button.on {
    border-color: currentColor;
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
</style>
