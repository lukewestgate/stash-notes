// In-memory stand-in for the Rust backend, used when the UI runs in a plain
// browser (`npm run dev`). Search and linking are simplified, but every
// command answers so the whole UI can be exercised without Tauri.

import type { NoteKind, NoteMeta, SearchHit, Settings, Startup } from "./api";
import { mockEmit } from "./api";
import { ChangeSet, Text } from "@codemirror/state";
import { isoDate } from "./text";

const today = isoDate();

const settings: Settings = {
  schemaVersion: 4,
  vaultPath: "(browser demo – nothing is saved)",
  hotkey: "Alt+N",
  switcherHotkey: "Alt+J",
  timestampFormat: "yyyy-MM-dd HH:mm",
  onSummon: "last",
  startHidden: true,
  autostart: true,
  showInTaskbar: false,
  alwaysOnTop: false,
  hideAfterExternalLink: false,
  ticketPattern: "[A-Z][A-Z0-9]+-\\d+",
  jira: { urlTemplate: "https://yourco.atlassian.net/browse/{key}" },
  gerrit: { urlTemplate: "https://gerrit.example.com/c/{n}" },
  theme: "system",
  fontSize: 14,
  fontFamily: "",
  lineHeight: 1.6,
  spellCheck: false,
  accent: "blue",
  animations: "system",
  liveFormatting: true,
  smartPaste: true,
  restorePinned: true,
  notesView: { showDaily: false, showClosed: false, sort: "edited", groupByStatus: false },
  statuses: [
    { name: "todo", icon: "circle", color: "blue", active: true },
    { name: "open", icon: "circle-dot", color: "green", active: true },
    { name: "blocked", icon: "circle-slash", color: "red", active: true },
    { name: "backlog", icon: "circle-dashed", color: "gray", active: false },
    { name: "closed", icon: "circle-check", color: "purple", active: false },
  ],
  defaultStatus: "open",
  keys: {
    hide: "Esc",
    quickSwitcher: "Ctrl+K",
    dailyNote: "Ctrl+D",
    followLink: "Ctrl+Enter",
    back: "Alt+Left",
    forward: "Alt+Right",
    openJira: "Ctrl+J",
    openGerrit: "Ctrl+G",
    addGerrit: "Ctrl+Shift+G",
    timestamp: "Ctrl+;",
    backlinks: "Ctrl+Shift+B",
    archive: "Ctrl+Shift+A",
    help: "Ctrl+/",
    quit: "Ctrl+Shift+Q",
    rename: "F2",
    openSettings: "Ctrl+,",
    setStatus: "Ctrl+Shift+S",
    notesView: "Ctrl+L",
    toggleSidebar: "Ctrl+\\",
    focusSidebar: "Ctrl+0",
    pinNote: "Ctrl+E P",
  },
};

const notes = new Map<string, string>([
  [
    `daily/${today}.md`,
    `# ${today}\n\n## Todo\n\n- [x] reply to PROJ-123 rollout question\n- [ ] review [[Rollout plan]]\n- [ ] \n\n## Notes\n\n`,
  ],
  [
    "tickets/PROJ-123.md",
    `---\nstatus: open\ngerrit:\n  - 12345\n  - https://gerrit.example.com/c/proj/+/12399\n---\n\n# PROJ-123 Rollout date question\n\n- ${today} 14:05 asked about rollout date, see [[Rollout plan]]\n- ${today} 16:40 fix pushed, waiting on review\n`,
  ],
  ["tickets/OPS-4123.md", "---\nstatus: blocked\ngerrit:\n---\n\n# OPS-4123 Disk alerts on build agents\n\n"],
  ["notes/rollout-plan.md", "# Rollout plan\n\nPhase one: internal users. Tracked in PROJ-123.\n\nSee https://example.com/rollout\n"],
  ["archive/notes/old-idea.md", "# Old idea\n\nSomething archived.\n"],
]);
const mtimes = new Map<string, number>([...notes.keys()].map((k, i) => [k, Date.now() - i * 60_000]));
const recents = new Map<string, number>();
const versions = new Map<string, number>();
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
  const archived = path.startsWith("archive/");
  const p = archived ? path.slice(8) : path;
  const s = stem(p);
  const { fields, body } = frontMatter(text);
  const kind: NoteKind = p.startsWith("daily/") ? "daily" : p.startsWith("tickets/") && ticketFull.test(s) ? "ticket" : "note";
  const heading = /^# (.+)$/m.exec(body)?.[1]?.trim();
  const named = heading?.startsWith(s) ? heading.slice(s.length).replace(/^[\s:–—-]+/, "") : heading;
  const label = named || (typeof fields.title === "string" ? fields.title : null);
  const gerrit = fields.gerrit;
  return {
    path,
    kind,
    archived,
    title: kind === "note" ? (heading ?? s) : s,
    label: kind === "ticket" ? label : null,
    key: kind === "ticket" ? s : null,
    date: kind === "daily" ? s : null,
    jira: typeof fields.jira === "string" ? fields.jira : null,
    gerrit: Array.isArray(gerrit) ? gerrit : typeof gerrit === "string" ? [gerrit] : [],
    aliases: [],
    status: kind !== "daily" && typeof fields.status === "string" ? fields.status : null,
  };
}

function hit(path: string, snippet: string | null = null, score = 0): SearchHit {
  const m = meta(path, notes.get(path)!);
  return {
    path,
    kind: m.kind,
    title: m.title,
    label: m.label,
    key: m.key,
    status: m.status,
    archived: m.archived,
    mtime: mtimes.get(path) ?? 0,
    snippet,
    score,
  };
}

function resolve(target: string): string | null {
  const t = target.trim().toLowerCase();
  for (const [p, text] of notes) {
    const m = meta(p, text);
    if (stem(p).toLowerCase() === t || m.title.toLowerCase() === t) return p;
  }
  return null;
}

function slug(s: string) {
  return s.toLowerCase().replace(/[^\p{L}\p{N}]+/gu, "-").replace(/^-|-$/g, "") || "untitled";
}

function ensure(target: string): string {
  const found = resolve(target);
  if (found) return found;
  const t = target.trim();
  let path: string;
  let body: string;
  if (/^\d{4}-\d{2}-\d{2}$/.test(t)) {
    path = `daily/${t}.md`;
    body = `# ${t}\n\n## Todo\n\n- [ ] \n\n## Notes\n\n`;
  } else if (ticketFull.test(t.split(/\s+/)[0].toUpperCase())) {
    const [key, ...name] = t.split(/\s+/);
    path = `tickets/${key.toUpperCase()}.md`;
    body = `---\nstatus: open\ngerrit:\n---\n\n# ${key.toUpperCase()} ${name.join(" ")}\n\n`;
  } else {
    path = `notes/${slug(t)}.md`;
    body = `# ${t}\n\n`;
  }
  notes.set(path, body);
  mtimes.set(path, Date.now());
  return path;
}

function changed(path: string) {
  mtimes.set(path, Date.now());
  setTimeout(() => mockEmit("vault-changed", [path]), 50);
}

type A = Record<string, any>;

export function handle(cmd: string, a: A): unknown {
  switch (cmd) {
    case "get_startup":
      return {
        settings,
        vaultPath: settings.vaultPath,
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
    case "get_daily_template":
      return dailyTemplate;
    case "set_daily_template":
      dailyTemplate = a.text;
      return;
    case "buffer_open": {
      const text = notes.get(a.path);
      if (text === undefined) throw new Error(`no such note ${a.path}`);
      recents.set(a.path, Date.now());
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
    case "pin_toggle":
      window.open(`${location.pathname}?pin=${encodeURIComponent(a.path)}`, "_blank", "width=340,height=420");
      return true;
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
      const path = ensure(a.title);
      mockEmit("open-note", path);
      return path;
    }
    case "open_in_main":
      mockEmit("open-note", a.path);
      return;
    case "resolve_link":
    case "create_note":
      return ensure(a.target ?? a.title);
    case "daily_note":
      return ensure(a.date);
    case "archive_note": {
      const to = a.path.startsWith("archive/") ? a.path.slice(8) : `archive/${a.path}`;
      notes.set(to, notes.get(a.path)!);
      notes.delete(a.path);
      versions.set(to, versions.get(a.path) ?? 0);
      changed(to);
      return to;
    }
    case "rename_note": {
      const text = notes.get(a.path)!;
      const to = `notes/${slug(a.title)}.md`;
      notes.delete(a.path);
      notes.set(to, text.replace(/^# .*$/m, `# ${a.title}`));
      changed(to);
      return { newPath: to, linksUpdated: 0, filesChanged: 0, undoId: 0 };
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
      const archived = path.startsWith("archive/");
      if ((st === "closed") !== archived) {
        const to = archived ? path.slice(8) : `archive/${path}`;
        notes.delete(path);
        notes.set(to, next);
        versions.set(to, versions.get(path) ?? 0);
        path = to;
      }
      changed(path);
      return { path, edit: null, meta: meta(path, next) };
    }
    case "undo_rename":
      throw new Error("undo is not available in the browser demo");
    case "search": {
      const q = String(a.query).trim().toLowerCase();
      const visible = [...notes.keys()].filter((p) => a.includeArchived || !p.startsWith("archive/"));
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
      const text = q.split(/\s+/).filter((w) => w && !w.startsWith("@")).join(" ");
      const hits = [...notes.keys()]
        .map((p) => hit(p, notes.get(p)!.replace(/^---[\s\S]*?---\n/, "").split("\n").find((l) => l.trim() && !l.startsWith("#")) ?? null))
        .filter((h) => a.includeDaily || h.kind !== "daily")
        .filter((h) => a.includePutAway || (!h.archived && h.status !== "closed"))
        .filter((h) => !filters.length || filters.some((f) => (f === "active" ? ["todo", "open", "blocked"].includes(h.status ?? "") : f === "none" ? !h.status : h.status === f)))
        .filter((h) => !text || `${h.title} ${h.label ?? ""} ${notes.get(h.path)}`.toLowerCase().includes(text))
        .sort((x, y) => (a.sort === "title" ? x.title.localeCompare(y.title) : y.mtime - x.mtime));
      return { hits, exact: hits.some((h) => h.title.toLowerCase() === text) };
    }
    case "sidebar": {
      const all = [...notes.keys()].map((p) => hit(p)).filter((h) => !h.archived);
      const rank = (s: string | null) => ["open", "blocked", "todo"].indexOf(s ?? "");
      const active = all.filter((h) => rank(h.status) >= 0).sort((x, y) => rank(x.status) - rank(y.status) || y.mtime - x.mtime);
      const recent = [...recents.entries()]
        .sort((x, y) => y[1] - x[1])
        .map(([p]) => p)
        .filter((p) => notes.has(p) && !p.startsWith("daily/") && !active.some((h) => h.path === p))
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
      return [...notes.keys()].filter((p) => p.startsWith("archive/")).map((p) => meta(p, notes.get(p)!).title.toLowerCase());
    case "gerrit_edit": {
      const content: string = a.content;
      const m = /^---\n[\s\S]*?\ngerrit:[^\n]*\n((?:\s+-[^\n]*\n)*)/.exec(content);
      if (m) {
        const at = m.index + m[0].length;
        return { from: at, to: at, insert: `  - ${a.link}\n` };
      }
      return { from: 0, to: 0, insert: `---\ngerrit:\n  - ${a.link}\n---\n\n` };
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
