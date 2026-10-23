// In-memory stand-in for the Rust backend, used when the UI runs in a plain
// browser (`npm run dev`). Search and linking are simplified, but every
// command answers so the whole UI can be exercised without Tauri.

import type { NoteKind, NoteMeta, SearchHit, Settings, Startup } from "./api";
import { mockEmit } from "./api";
import { ChangeSet, Text } from "@codemirror/state";
import { isoDate, normalizeTag } from "./text";

const today = isoDate();

const settings: Settings = {
  schemaVersion: 5,
  vaults: [
    { id: "personal", name: "Personal", path: "", color: "blue", tickets: [], links: [] },
    {
      id: "work",
      name: "Work",
      path: "",
      color: "orange",
      tickets: [{ name: "Jira", icon: "ticket", pattern: "[A-Z][A-Z0-9]+-\\d+", url: "https://yourco.atlassian.net/browse/{key}" }],
      links: [{ name: "Gerrit", icon: "git-pull-request", field: "gerrit", url: "https://gerrit.example.com/c/{n}" }],
    },
  ],
  hotkey: "Alt+N",
  switcherHotkey: "Alt+J",
  timestampFormat: "yyyy-MM-dd HH:mm",
  onSummon: "last",
  startHidden: true,
  autostart: true,
  showInTaskbar: false,
  alwaysOnTop: false,
  hideAfterExternalLink: false,
  theme: "system",
  fontSize: 14,
  fontFamily: "",
  lineHeight: 1.6,
  spellCheck: false,
  accent: "blue",
  readableWidth: true,
  animations: "system",
  liveFormatting: true,
  smartPaste: true,
  restorePinned: true,
  notesView: { showDaily: false, showClosed: false, sort: "edited", groupBy: "none" },
  statuses: [
    { name: "todo", icon: "circle", color: "blue", active: true },
    { name: "open", icon: "circle-dot", color: "green", active: true },
    { name: "blocked", icon: "circle-slash", color: "red", active: true },
    { name: "backlog", icon: "circle-dashed", color: "gray", active: false },
    { name: "closed", icon: "circle-check", color: "purple", active: false },
  ],
  tagColors: { bug: "red" },
  defaultStatus: "open",
  keys: {
    hide: "Esc",
    quickSwitcher: "Ctrl+K",
    dailyNote: "Ctrl+D",
    followLink: "Ctrl+Enter",
    back: "Alt+Left",
    forward: "Alt+Right",
    openTicket: "Ctrl+J",
    openLink: "Ctrl+G",
    addLink: "Ctrl+Shift+G",
    timestamp: "Ctrl+;",
    backlinks: "Ctrl+Shift+B",
    archive: "Ctrl+Shift+A",
    help: "Ctrl+/",
    quit: "Ctrl+Shift+Q",
    rename: "F2",
    openSettings: "Ctrl+,",
    setStatus: "Ctrl+Shift+S",
    setTags: "Ctrl+Shift+T",
    notesView: "Ctrl+L",
    toggleSidebar: "Ctrl+\\",
    focusSidebar: "Ctrl+0",
    pinNote: "Ctrl+E P",
    switchVault: "Ctrl+Shift+O",
  },
};

const notes = new Map<string, string>([
  [`personal:daily/${today}.md`, `# ${today}\n\n## Todo\n\n- [ ] groceries, see [[Shopping list]]\n\n## Notes\n\n`],
  ["personal:notes/shopping-list.md", "---\ntags: [home]\n---\n# Shopping list\n\n- milk\n- coffee\n"],
  [
    `work:daily/${today}.md`,
    `# ${today}\n\n## Todo\n\n- [x] reply to PROJ-123 rollout question\n- [ ] review [[Rollout plan]]\n- [ ] \n\n## Notes\n\n`,
  ],
  [
    "work:tickets/PROJ-123.md",
    `---\nstatus: open\ngerrit:\n  - 12345\n  - https://gerrit.example.com/c/proj/+/12399\n---\n\n# PROJ-123 Rollout date question\n\n- ${today} 14:05 asked about rollout date, see [[Rollout plan]]\n- ${today} 16:40 fix pushed, waiting on review\n`,
  ],
  ["work:tickets/OPS-4123.md", "---\nstatus: blocked\ntags: [bug, infra]\ngerrit:\n---\n\n# OPS-4123 Disk alerts on build agents\n\n"],
  ["work:notes/rollout-plan.md", "# Rollout plan\n\nPhase one: internal users. Tracked in PROJ-123.\n\nSee https://example.com/rollout\n"],
  ["work:archive/notes/old-idea.md", "# Old idea\n\nSomething archived.\n"],
]);
const mtimes = new Map<string, number>([...notes.keys()].map((k, i) => [k, Date.now() - i * 60_000]));
const recents = new Map<string, number>();
const versions = new Map<string, number>();
const pinned = new Set<string>();
let nextUndo = 1;
const deletedNotes = new Map<number, { path: string; text: string }>();
const renamedNotes = new Map<number, { from: string; to: string; previous: Map<string, string> }>();
let dailyTemplate = "# {{date}}\n\n## Todo\n\n- [ ] \n\n## Notes\n\n";
let clients = 0;

/** Replace a note's text from the "backend", as a buffer update open views receive. */
function serverEdit(path: string, next: string) {
  const prev = notes.get(path) ?? "";
  notes.set(path, next);
  if (prev === next) return;
  const v = versions.get(path) ?? 0;
  versions.set(path, v + 1);
  const changes = ChangeSet.of({ from: 0, to: prev.length, insert: next }, prev.length).toJSON();
  setTimeout(() => mockEmit("buffer-updates", { path, version: v, updates: [{ changes, clientID: "app" }] }), 0);
}

const ticketFull = /^[A-Z][A-Z0-9]+-\d+$/;

/** `work:notes/x.md` -> `notes/x.md`. */
function rel(p: string) {
  return p.slice(p.indexOf(":") + 1);
}

function vaultOf(p: string) {
  return p.slice(0, p.indexOf(":"));
}

function stem(p: string) {
  return p.split("/").pop()!.replace(/\.md$/, "");
}

function frontMatter(text: string): { fields: Record<string, string | string[]>; body: string } {
  const m = /^---\r?\n([\s\S]*?)\r?\n---\r?\n?/.exec(text);
  if (!m) return { fields: {}, body: text };
  const fields: Record<string, string | string[]> = {};
  let list: string | null = null;
  for (const line of m[1].split(/\r?\n/)) {
    const item = /^\s*-\s+(.*)$/.exec(line);
    if (item && list) (fields[list] as string[]).push(item[1].trim());
    else {
      const kv = /^(\w+):\s*(.*)$/.exec(line);
      if (kv) {
        if (kv[2]) {
          fields[kv[1]] = kv[2].replace(/\s+#.*$/, "");
          list = null;
        } else {
          fields[kv[1]] = [];
          list = kv[1];
        }
      }
    }
  }
  return { fields, body: text.slice(m[0].length) };
}

function meta(path: string, text: string): NoteMeta {
  const archived = rel(path).startsWith("archive/");
  const p = archived ? rel(path).slice(8) : rel(path);
  const s = stem(p);
  const { fields, body } = frontMatter(text);
  const kind: NoteKind = p.startsWith("daily/") ? "daily" : p.startsWith("tickets/") && ticketFull.test(s) ? "ticket" : "note";
  const heading = /^# (.+)$/m.exec(body)?.[1]?.trim();
  const named = heading?.startsWith(s) ? heading.slice(s.length).replace(/^[\s:–—-]+/, "") : heading;
  const label = named || (typeof fields.title === "string" ? fields.title : null);
  const rawTags = fields.tags;
  const tagList = Array.isArray(rawTags) ? rawTags : typeof rawTags === "string" ? rawTags.replace(/^\[|\]$/g, "").split(",") : [];
  return {
    path,
    vault: vaultOf(path),
    kind,
    archived,
    title: kind === "note" ? (heading ?? s) : s,
    label: kind === "ticket" ? label : null,
    key: kind === "ticket" ? s : null,
    date: kind === "daily" ? s : null,
    ticketUrl: typeof fields.jira === "string" ? fields.jira : null,
    fields: Object.fromEntries(
      Object.entries(fields)
        .filter(([k]) => !["status", "tags", "aliases", "title", "jira"].includes(k))
        .map(([k, v]) => [k, Array.isArray(v) ? v : v ? [v] : []] as [string, string[]])
        .filter(([, v]) => v.length),
    ),
    aliases: [],
    status: kind !== "daily" && typeof fields.status === "string" ? fields.status : null,
    tags: kind === "daily" ? [] : [...new Set(tagList.map((t) => normalizeTag(t)).filter((t): t is string => !!t))],
  };
}

/** `#tag` filters (all must match; a parent matches nested tags). */
function tagFilter(q: string): (h: SearchHit) => boolean {
  const want = q.split(/\s+/).filter((w) => w.startsWith("#")).map((w) => normalizeTag(w)).filter((t): t is string => !!t);
  return (h) => want.every((w) => h.tags.some((t) => t === w || t.startsWith(`${w}/`)));
}

function setTagsIn(text: string, tags: string[]): string {
  const line = tags.length ? `tags: [${tags.join(", ")}]\n` : "";
  if (/^tags:.*\n/m.test(text)) return text.replace(/^tags:.*\n/m, line);
  if (!line) return text;
  if (text.startsWith("---\n")) return `---\n${line}${text.slice(4)}`;
  return `---\n${line}---\n\n${text}`;
}

function hit(path: string, snippet: string | null = null, score = 0): SearchHit {
  const m = meta(path, notes.get(path)!);
  return {
    path,
    vault: m.vault,
    kind: m.kind,
    title: m.title,
    label: m.label,
    key: m.key,
    status: m.status,
    tags: m.tags,
    archived: m.archived,
    mtime: mtimes.get(path) ?? 0,
    snippet,
    score,
  };
}

function resolve(target: string, vault: string): string | null {
  const [v, t0] = /^[a-z0-9_-]+:/.test(target) ? [target.slice(0, target.indexOf(":")), target.slice(target.indexOf(":") + 1)] : [vault, target];
  const t = t0.trim().toLowerCase();
  const order = [...notes.keys()].sort((x, y) => Number(vaultOf(y) === v) - Number(vaultOf(x) === v));
  for (const p of order) {
    if (target.includes(":") && vaultOf(p) !== v) continue;
    const m = meta(p, notes.get(p)!);
    if (rel(p).replace(/\.md$/, "").toLowerCase() === t.replace(/\.md$/, "") || stem(p).toLowerCase() === t || m.title.toLowerCase() === t) return p;
  }
  return null;
}

function slug(s: string) {
  return s.toLowerCase().replace(/[^\p{L}\p{N}]+/gu, "-").replace(/^-|-$/g, "") || "untitled";
}

function ensure(target: string, vault: string, announce = false): string {
  const found = resolve(target, vault);
  if (found) return found;
  const t = target.trim();
  let path: string;
  let body: string;
  if (/^\d{4}-\d{2}-\d{2}$/.test(t)) {
    path = `${vault}:daily/${t}.md`;
    body = dailyTemplate.replace("{{date}}", t);
  } else if (ticketFull.test(t.split(/\s+/)[0].toUpperCase())) {
    const [key, ...name] = t.split(/\s+/);
    // Ticket keys live in the work vault in the demo.
    path = `work:tickets/${key.toUpperCase()}.md`;
    body = `---\nstatus: open\n---\n\n# ${key.toUpperCase()} ${name.join(" ")}\n\n`;
  } else {
    path = `${vault}:notes/${slug(t)}.md`;
    body = `# ${t}\n\n`;
  }
  if (notes.has(path)) return path;
  notes.set(path, body);
  mtimes.set(path, Date.now());
  if (announce) mockEmit("note-created", meta(path, body));
  return path;
}

/** The current vault (the main window follows the note it opens). */
let current = "work";

/** `in:` words in a query -> the vault ids they pick (null: none given). */
function inScope(query: string): Set<string> | null {
  const words = query.split(/\s+/).filter((w) => w.startsWith("in:")).map((w) => w.slice(3).toLowerCase());
  if (!words.length) return null;
  return new Set(settings.vaults.filter((v) => words.some((w) => v.id.startsWith(w) || v.name.toLowerCase().startsWith(w))).map((v) => v.id));
}

/** The query without filter words. */
function plain(query: string): string {
  return query.split(/\s+/).filter((w) => !/^(#|@|in:)/.test(w)).join(" ").trim();
}

function changed(path: string) {
  mtimes.set(path, Date.now());
  setTimeout(() => mockEmit("vault-changed", [path]), 50);
}

type A = Record<string, any>;
const images = new Map<string, Uint8Array>();

function followMove(from: string, to: string) {
  versions.set(to, versions.get(from) ?? 0);
  versions.delete(from);
  if (recents.has(from)) { recents.set(to, recents.get(from)!); recents.delete(from); }
  if (pinned.delete(from)) pinned.add(to);
  mockEmit("note-moved", { from, to });
  mockEmit("pins-changed", [...pinned]);
}

function rewriteLinks(text: string, owner: string, from: string, to: string, qualifyOutbound = false) {
  return text.replace(/\[\[([^\]#|]+)([^\]]*)\]\]/g, (link, target: string, suffix: string) => {
    const resolved = resolve(target.trim(), vaultOf(owner));
    if (resolved === from) return `[[${to.replace(/\.md$/, "")}${suffix}]]`;
    if (qualifyOutbound && resolved) return `[[${resolved.replace(/\.md$/, "")}${suffix}]]`;
    return link;
  });
}

export function handle(cmd: string, a: A): unknown {
  switch (cmd) {
    case "import_image": {
      const bytes = new Uint8Array(a.bytes);
      if (!bytes.length || bytes.length > 20 * 1024 * 1024) throw new Error("Images must be between 1 byte and 20 MB.");
      const format = bytes[0] === 0x89 && bytes[1] === 0x50 ? "png"
        : bytes[0] === 0xff && bytes[1] === 0xd8 ? "jpg"
        : bytes[0] === 0x47 && bytes[1] === 0x49 ? "gif"
        : bytes[0] === 0x52 && bytes[8] === 0x57 ? "webp"
        : bytes[0] === 0x42 && bytes[1] === 0x4d ? "bmp" : null;
      if (!format) throw new Error("Unsupported image. Use PNG, JPEG, GIF, WebP or BMP.");
      const source = `/assets/image-${crypto.randomUUID()}.${format}`;
      images.set(`${vaultOf(a.path)}:${source}`, bytes);
      return source;
    }
    case "read_image": {
      const source = new URL(a.source, `https://vault/${rel(a.path)}`).pathname;
      const bytes = images.get(`${vaultOf(a.path)}:${source}`);
      if (!bytes) throw new Error("Image file not found.");
      return bytes.slice().buffer;
    }
    case "get_startup":
      return {
        settings,
        vaultPaths: { personal: "(browser demo) Personal", work: "(browser demo) Work" },
        currentVault: current,
        settingsPath: "settings.json",
        warnings: [],
        lastPath: null,
        lastAnchor: null,
        indexReady: true,
        ui: {},
      } satisfies Startup;
    case "get_settings":
      return settings;
    case "set_settings":
      Object.assign(settings, a.settings);
      setTimeout(() => mockEmit("settings-changed", structuredClone(settings)), 0);
      return;
    case "default_settings":
      return { ...structuredClone(settings), hotkey: "Alt+N", theme: "system", accent: "blue" };
    case "test_hotkey":
      if (/^(Ctrl\+Alt\+Del)$/i.test(a.spec)) throw new Error(`\`${a.spec}\` is taken by another app`);
      return;
    case "test_ticket_pattern": {
      const re = new RegExp(`\\b(?:${a.pattern})\\b`, "g");
      return [...String(a.sample).matchAll(re)].map((m) => m[0]);
    }
    case "pick_folder":
      return "C:\\Users\\me\\Notes";
    case "get_vault_paths":
      return { personal: "(browser demo) Personal", work: "(browser demo) Work" };
    case "default_vault_folder":
      return "(browser demo – nothing is saved)";
    case "switch_vault":
      current = a.vault;
      return null;
    case "get_daily_template":
      return dailyTemplate;
    case "set_daily_template":
      dailyTemplate = a.text;
      return;
    case "buffer_open": {
      const text = notes.get(a.path);
      if (text === undefined) throw new Error(`no such note ${a.path}`);
      recents.set(a.path, Date.now());
      current = vaultOf(a.path);
      return {
        text,
        version: versions.get(a.path) ?? 0,
        meta: meta(a.path, text),
        dirty: false,
        deleted: false,
        clientID: `mock-${++clients}`,
      };
    }
    case "buffer_push": {
      const v = versions.get(a.path) ?? 0;
      if (a.version !== v) return false;
      let text = notes.get(a.path) ?? "";
      for (const u of a.updates) text = ChangeSet.fromJSON(u.changes).apply(Text.of(text.split("\n"))).toString();
      notes.set(a.path, text);
      versions.set(a.path, v + a.updates.length);
      changed(a.path);
      setTimeout(() => {
        mockEmit("buffer-updates", { path: a.path, version: v, updates: a.updates });
        mockEmit("buffer-state", { path: a.path, dirty: false, meta: meta(a.path, text) });
      }, 0);
      return true;
    }
    case "buffer_pull":
      return a.version === (versions.get(a.path) ?? 0) ? [] : null;
    case "buffer_close":
    case "buffer_flush":
    case "buffer_resolve":
      return;
    case "pin_toggle": {
      const on = !pinned.delete(a.path);
      if (on) {
        pinned.add(a.path);
        window.open(`${location.pathname}?pin=${encodeURIComponent(a.path)}`, "_blank", "width=340,height=420");
      }
      mockEmit("pins-changed", [...pinned]);
      return on;
    }
    case "pinned_paths":
      return [...pinned];
    case "pin_path":
      return new URLSearchParams(location.search).get("pin");
    case "pin_close":
      window.close();
      return;
    case "pin_set_on_top":
      return;
    case "smoke_enabled":
      return false;
    case "smoke_step":
    case "quick_hide":
    case "quick_resize":
      return;
    case "quick_open":
      mockEmit("open-note", a.path);
      return;
    case "quick_create": {
      const path = ensure(plain(a.title), a.vault ?? current, true);
      mockEmit("open-note", path);
      return path;
    }
    case "open_in_main":
      mockEmit("open-note", a.path);
      return;
    case "resolve_link":
      return ensure(a.target, a.from ? vaultOf(a.from) : current, true);
    case "note_preview": {
      const path = resolve(a.target, a.from ? vaultOf(a.from) : current);
      if (!path) throw new Error("Linked note not found");
      return { path, text: notes.get(path)! };
    }
    case "create_note": {
      const path = ensure(plain(a.title), a.vault, true);
      const tags = String(a.title).split(/\s+/).filter((w) => w.startsWith("#")).map((w) => normalizeTag(w)).filter((t): t is string => !!t);
      if (tags.length) notes.set(path, setTagsIn(notes.get(path)!, tags));
      return path;
    }
    case "daily_note":
      return ensure(a.date, a.vault);
    case "archive_note": {
      const r = rel(a.path);
      const to = `${vaultOf(a.path)}:${r.startsWith("archive/") ? r.slice(8) : `archive/${r}`}`;
      const text = notes.get(a.path);
      if (text === undefined) throw new Error("Note not found");
      if (notes.has(to)) throw new Error("An archived note already exists at this path");
      notes.set(to, text);
      notes.delete(a.path);
      followMove(a.path, to);
      mockEmit("buffer-state", { path: to, dirty: false, meta: meta(to, text) });
      changed(to);
      return to;
    }
    case "rename_note": {
      const text = notes.get(a.path);
      if (text === undefined) throw new Error("Note not found");
      if (meta(a.path, text).kind !== "note") throw new Error("Daily and ticket files keep their date or key");
      if (!a.title.trim() || /[\[\]|#\n\r]/.test(a.title)) throw new Error("Invalid note title");
      const to = a.path.replace(/[^/]+$/, `${slug(a.title)}.md`);
      if (to !== a.path && notes.has(to)) throw new Error("A note with this name already exists");
      const previous = new Map<string, string>();
      const updates = new Map<string, string>();
      for (const [p, old] of notes) {
        let next = rewriteLinks(old, p, a.path, to);
        if (p === a.path) next = next.replace(/^# .*$/m, `# ${a.title}`);
        if (next !== old || p === a.path) { previous.set(p, old); updates.set(p, next); }
      }
      for (const [p, next] of updates) { if (p !== a.path) serverEdit(p, next); }
      notes.set(to, text);
      if (to !== a.path) { notes.delete(a.path); followMove(a.path, to); }
      serverEdit(to, updates.get(a.path)!);
      const undoId = nextUndo++;
      renamedNotes.set(undoId, { from: a.path, to, previous });
      changed(to);
      return { newPath: to, linksUpdated: previous.size - 1, filesChanged: previous.size - 1, undoId };
    }
    case "move_note": {
      const text = notes.get(a.path);
      if (text === undefined) throw new Error("Note not found");
      if (!settings.vaults.some((v) => v.id === a.vault)) throw new Error("Vault not found");
      const destination = rel(a.path);
      if (!destination.endsWith(".md") || destination.split("/").some((s) => !s || s.startsWith(".")) || destination.startsWith("templates/") || /[:<>"|?*\0]/.test(destination)) throw new Error("Invalid destination path");
      const to = `${a.vault}:${destination}`;
      if (to === a.path) return to;
      if (notes.has(to)) throw new Error(`${destination} already exists`);
      const updates = new Map<string, string>();
      for (const [p, old] of notes) updates.set(p, rewriteLinks(old, p, a.path, to, p === a.path && vaultOf(p) !== a.vault));
      let moved = updates.get(a.path)!;
      if (vaultOf(a.path) !== a.vault) {
        moved = moved.replace(/!\[[^\]]*\]\((\/assets\/[^)]+)\)/g, (markdown, source) => {
          const bytes = images.get(`${vaultOf(a.path)}:${source}`);
          if (!bytes) throw new Error(`Image file not found: ${source}`);
          const dest = `/assets/image-${crypto.randomUUID()}.${source.split(".").pop()}`;
          images.set(`${a.vault}:${dest}`, bytes.slice());
          return markdown.replace(source, dest);
        });
      }
      for (const [p, next] of updates) if (p !== a.path) serverEdit(p, next);
      notes.set(to, text);
      notes.delete(a.path);
      followMove(a.path, to);
      serverEdit(to, moved);
      mockEmit("buffer-state", { path: to, dirty: false, meta: meta(to, moved) });
      changed(to);
      return to;
    }
    case "delete_note": {
      const text = notes.get(a.path);
      if (text === undefined) throw new Error("Note not found");
      const id = nextUndo++;
      deletedNotes.set(id, { path: a.path, text });
      notes.delete(a.path);
      versions.delete(a.path);
      recents.delete(a.path);
      pinned.delete(a.path);
      mockEmit("note-deleted", a.path);
      mockEmit("pins-changed", [...pinned]);
      changed(a.path);
      return id;
    }
    case "undo_delete": {
      const saved = deletedNotes.get(a.undoId);
      if (!saved) throw new Error("Nothing to undo");
      if (notes.has(saved.path)) throw new Error("A note already exists at the original path");
      notes.set(saved.path, saved.text);
      deletedNotes.delete(a.undoId);
      changed(saved.path);
      return saved.path;
    }
    case "set_status": {
      let path: string = a.path;
      const text = notes.get(path)!;
      const st: string | null = a.status;
      const m = /^status:.*$/m.exec(text);
      let edit;
      if (m) edit = { from: m.index, to: m.index + m[0].length + (st ? 0 : 1), insert: st ? `status: ${st}` : "" };
      else if (text.startsWith("---\n")) edit = st ? { from: 4, to: 4, insert: `status: ${st}\n` } : null;
      else edit = st ? { from: 0, to: 0, insert: `---\nstatus: ${st}\n---\n\n` } : null;
      const next = edit ? text.slice(0, edit.from) + edit.insert + text.slice(edit.to) : text;
      serverEdit(path, next);
      const archived = rel(path).startsWith("archive/");
      if ((st === "closed") !== archived) {
        const to = `${vaultOf(path)}:${archived ? rel(path).slice(8) : `archive/${rel(path)}`}`;
        notes.delete(path);
        notes.set(to, next);
        versions.set(to, versions.get(path) ?? 0);
        path = to;
      }
      changed(path);
      return { path, edit: null, meta: meta(path, next) };
    }
    case "set_tags": {
      const text = notes.get(a.path)!;
      const tags = [...new Set((a.tags as string[]).map((t) => normalizeTag(t)).filter((t): t is string => !!t))];
      const next = setTagsIn(text, tags);
      serverEdit(a.path, next);
      changed(a.path);
      return { path: a.path, edit: null, meta: meta(a.path, next) };
    }
    case "list_tags": {
      const counts = new Map<string, number>();
      for (const p of notes.keys()) for (const t of hit(p).tags) counts.set(t, (counts.get(t) ?? 0) + 1);
      return [...counts].map(([name, count]) => ({ name, count })).sort((x, y) => y.count - x.count || x.name.localeCompare(y.name));
    }
    case "rename_tag": {
      let n = 0;
      for (const p of [...notes.keys()]) {
        const tags = hit(p).tags;
        if (!tags.some((t) => t === a.from || t.startsWith(`${a.from}/`))) continue;
        const next = tags.flatMap((t) => (t === a.from || t.startsWith(`${a.from}/`) ? (a.to ? [a.to + t.slice(a.from.length)] : []) : [t]));
        serverEdit(p, setTagsIn(notes.get(p)!, [...new Set(next)]));
        changed(p);
        n++;
      }
      return { notes: n, undoId: 0 };
    }
    case "undo_tag_rename":
      throw new Error("undo is not available in the browser demo");
    case "undo_rename": {
      const saved = renamedNotes.get(a.undoId);
      if (!saved) throw new Error("Nothing to undo");
      if (saved.from !== saved.to && notes.has(saved.from)) throw new Error("The original path already exists");
      notes.set(saved.from, notes.get(saved.to)!);
      if (saved.from !== saved.to) { notes.delete(saved.to); followMove(saved.to, saved.from); }
      for (const [p, text] of saved.previous) serverEdit(p, text);
      renamedNotes.delete(a.undoId);
      changed(saved.from);
      return saved.from;
    }
    case "search": {
      const tagged = tagFilter(String(a.query));
      const scope = inScope(String(a.query));
      const prefer = a.prefer ?? current;
      const q = plain(String(a.query)).toLowerCase();
      const visible = [...notes.keys()]
        .filter((p) => (a.includeArchived || !rel(p).startsWith("archive/")) && tagged(hit(p)) && (!scope || scope.has(vaultOf(p))))
        .sort((x, y) => Number(vaultOf(y) === prefer) - Number(vaultOf(x) === prefer));
      if (!q) {
        return {
          hits: visible
            .filter((p) => p !== a.current)
            .sort((x, y) => (recents.get(y) ?? mtimes.get(y)!) - (recents.get(x) ?? mtimes.get(x)!))
            .map((p) => hit(p)),
          exact: false,
        };
      }
      const hits: SearchHit[] = [];
      let exact = false;
      for (const p of visible) {
        const h = hit(p);
        const title = `${h.title} ${h.label ?? ""}`.toLowerCase();
        const digits = /^\d+$/.test(q) && h.kind === "ticket" && h.title.endsWith(q);
        if (h.title.toLowerCase() === q || (q === "today" && h.title === today)) exact = true;
        if (title.includes(q) || digits || (q === "today" && h.title === today)) hits.push({ ...h, score: 2 });
        else {
          const line = notes.get(p)!.split("\n").find((l) => l.toLowerCase().includes(q));
          if (line) hits.push({ ...h, snippet: line.trim(), score: 1 });
        }
      }
      hits.sort((x, y) => y.score - x.score);
      return { hits, exact };
    }
    case "list_notes": {
      const q = String(a.query).trim().toLowerCase();
      const filters = q.split(/\s+/).filter((w) => w.startsWith("@")).map((w) => w.slice(1));
      const text = q.split(/\s+/).filter((w) => w && !/^(@|#|in:)/.test(w)).join(" ");
      const scope = inScope(q) ?? new Set([a.vault]);
      const hits = [...notes.keys()]
        .filter((p) => scope.has(vaultOf(p)))
        .map((p) => hit(p, notes.get(p)!.replace(/^---[\s\S]*?---\n/, "").split("\n").find((l) => l.trim() && !l.startsWith("#")) ?? null))
        .filter(tagFilter(q))
        .filter((h) => a.includeDaily || h.kind !== "daily")
        .filter((h) => a.includePutAway || (!h.archived && h.status !== "closed"))
        .filter((h) => !filters.length || filters.some((f) => (f === "active" ? ["todo", "open", "blocked"].includes(h.status ?? "") : f === "none" ? !h.status : h.status === f)))
        .filter((h) => !text || `${h.title} ${h.label ?? ""} ${notes.get(h.path)}`.toLowerCase().includes(text))
        .sort((x, y) => (a.sort === "title" ? x.title.localeCompare(y.title) : y.mtime - x.mtime));
      return { hits, exact: hits.some((h) => h.title.toLowerCase() === text) };
    }
    case "sidebar": {
      const all = [...notes.keys()].filter((p) => vaultOf(p) === a.vault).map((p) => hit(p)).filter((h) => !h.archived);
      const rank = (s: string | null) => ["open", "blocked", "todo"].indexOf(s ?? "");
      const active = all.filter((h) => rank(h.status) >= 0).sort((x, y) => rank(x.status) - rank(y.status) || y.mtime - x.mtime);
      const recent = [...recents.entries()]
        .sort((x, y) => y[1] - x[1])
        .map(([p]) => p)
        .filter((p) => notes.has(p) && vaultOf(p) === a.vault && !rel(p).startsWith("daily/") && !rel(p).startsWith("archive/") && !active.some((h) => h.path === p))
        .slice(0, 10)
        .map((p) => hit(p));
      return { active: active.slice(0, 8), more: Math.max(0, active.length - 8), recent };
    }
    case "set_ui":
      return;
    case "backlinks": {
      const m = meta(a.path, notes.get(a.path) ?? "");
      const names = [m.title.toLowerCase(), stem(a.path).toLowerCase()];
      return [...notes.keys()]
        .filter((p) => p !== a.path)
        .filter((p) => {
          const t = notes.get(p)!.toLowerCase();
          return names.some((n) => t.includes(`[[${n}`)) || (m.key && notes.get(p)!.includes(m.key));
        })
        .map((p) => hit(p));
    }
    case "archived_names":
      return [...notes.keys()].filter((p) => vaultOf(p) === a.vault && rel(p).startsWith("archive/")).map((p) => meta(p, notes.get(p)!).title.toLowerCase());
    case "link_edit": {
      const content: string = a.content;
      const m = new RegExp(`^---\\n[\\s\\S]*?\\n${a.field}:[^\\n]*\\n((?:\\s+-[^\\n]*\\n)*)`).exec(content);
      if (m) {
        const at = m.index + m[0].length;
        return { from: at, to: at, insert: `  - ${a.item}\n` };
      }
      if (content.startsWith("---\n")) return { from: 4, to: 4, insert: `${a.field}:\n  - ${a.item}\n` };
      return { from: 0, to: 0, insert: `---\n${a.field}:\n  - ${a.item}\n---\n\n` };
    }
    case "open_url":
      window.open(a.url, "_blank", "noopener");
      return;
    case "read_clipboard":
      return "";
    case "hide_window":
    case "quit_app":
    case "save_session":
    case "open_settings_file":
    case "open_vault_folder":
      return;
  }
  throw new Error(`mock: unknown command ${cmd}`);
}
